use crate::{Error, Result};
use burn::tensor::{Device, Tensor, TensorData};
use safetensors::{Dtype, SafeTensors, tensor::TensorInfo};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Component, Path},
    sync::Mutex,
};

struct Shard {
    file: Mutex<File>,
    offset: usize,
}

/// Reads checkpoint tensors on demand, retaining only one tensor's host buffer during import.
pub struct Weights {
    shards: Vec<Shard>,
    tensors: BTreeMap<String, (usize, TensorInfo)>,
}

pub fn validate_checkpoint_filename(name: &str) -> Result<()> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
        || name.contains(['/', '\\'])
    {
        return Err(Error::Invalid(format!(
            "Checkpoint file must be a filename: {name}"
        )));
    }
    Ok(())
}

impl Weights {
    pub fn open(directory: &Path) -> Result<Self> {
        let index = directory.join("model.safetensors.index.json");
        let (files, declared) = if index.is_file() {
            #[derive(serde::Deserialize)]
            struct Index {
                weight_map: BTreeMap<String, String>,
            }
            let index: Index = serde_json::from_slice(&std::fs::read(index)?)?;
            let files: BTreeSet<String> = index.weight_map.values().cloned().collect();
            (files.into_iter().collect(), Some(index.weight_map))
        } else if directory.join("model.safetensors").is_file() {
            (vec!["model.safetensors".into()], None)
        } else {
            let mut files = Vec::new();
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                if entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "safetensors")
                    && entry.file_type()?.is_file()
                {
                    files.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
            files.sort();
            (files, None)
        };
        if files.is_empty() {
            return Err(Error::Invalid("No SafeTensors checkpoint found".into()));
        }
        let mut shards = Vec::new();
        let mut tensors = BTreeMap::new();
        for (shard_id, name) in files.iter().enumerate() {
            validate_checkpoint_filename(name)?;
            let mut file = File::open(directory.join(name))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            let (header_length, metadata) = SafeTensors::read_metadata(&bytes)
                .map_err(|e| Error::Invalid(format!("Invalid checkpoint {name}: {e}")))?;
            for (key, info) in metadata.tensors() {
                if let Some(declared) = &declared
                    && declared.get(&key) != Some(name)
                {
                    return Err(Error::Invalid(format!(
                        "Checkpoint index disagrees with shard {name} for {key}"
                    )));
                }
                if tensors
                    .insert(key.clone(), (shard_id, info.clone()))
                    .is_some()
                {
                    return Err(Error::Invalid(format!("Duplicate checkpoint tensor {key}")));
                }
            }
            shards.push(Shard {
                file: Mutex::new(file),
                offset: header_length + 8,
            });
        }
        if let Some(declared) = declared
            && declared.keys().any(|key| !tensors.contains_key(key))
        {
            return Err(Error::Invalid(
                "Checkpoint index names missing tensors".into(),
            ));
        }
        Ok(Self { shards, tensors })
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tensors.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.tensors.keys().map(String::as_str)
    }

    pub fn shape(&self, name: &str) -> Result<Vec<usize>> {
        Ok(self.info(name)?.1.shape.clone())
    }

    fn info(&self, name: &str) -> Result<&(usize, TensorInfo)> {
        self.tensors
            .get(name)
            .ok_or_else(|| Error::Invalid(format!("Checkpoint is missing tensor {name}")))
    }

    pub fn values(&self, name: &str) -> Result<Vec<f32>> {
        let (shard_id, info) = self.info(name)?;
        let shard = &self.shards[*shard_id];
        let mut file = shard
            .file
            .lock()
            .map_err(|_| Error::Invalid("Checkpoint reader lock was poisoned".into()))?;
        let mut bytes = vec![0; info.data_offsets.1 - info.data_offsets.0];
        file.seek(SeekFrom::Start((shard.offset + info.data_offsets.0) as u64))?;
        file.read_exact(&mut bytes)?;
        let values: Vec<f32> = match info.dtype {
            Dtype::F32 => bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect(),
            Dtype::F16 => bytes
                .chunks_exact(2)
                .map(|b| half::f16::from_bits(u16::from_le_bytes(b.try_into().unwrap())).to_f32())
                .collect(),
            Dtype::BF16 => bytes
                .chunks_exact(2)
                .map(|b| half::bf16::from_bits(u16::from_le_bytes(b.try_into().unwrap())).to_f32())
                .collect(),
            Dtype::F64 => bytes
                .chunks_exact(8)
                .map(|b| f64::from_le_bytes(b.try_into().unwrap()) as f32)
                .collect(),
            dtype => {
                return Err(Error::Invalid(format!(
                    "Tensor {name} has unsupported weight dtype {dtype:?}"
                )));
            }
        };
        if values.iter().any(|v| !v.is_finite()) {
            return Err(Error::Invalid(format!(
                "Tensor {name} contains non-finite weights"
            )));
        }
        Ok(values)
    }

    pub fn scalar(&self, name: &str) -> Result<f32> {
        let values = self.values(name)?;
        if values.len() != 1 {
            return Err(Error::Invalid(format!("Tensor {name} must be scalar")));
        }
        Ok(values[0])
    }

    pub fn tensor<const D: usize>(&self, name: &str, device: &Device) -> Result<Tensor<D>> {
        let shape = self.shape(name)?;
        if shape.len() != D || shape.contains(&0) {
            return Err(Error::Invalid(format!(
                "Tensor {name} needs rank {D} with positive dimensions; found {shape:?}"
            )));
        }
        let shape: [usize; D] = shape
            .try_into()
            .map_err(|_| Error::Invalid(format!("Tensor {name} rank mismatch")))?;
        Ok(Tensor::from_data(
            TensorData::new(self.values(name)?, shape),
            device,
        ))
    }
}

#[cfg(all(test, feature = "cpu"))]
mod tests {
    use super::*;

    #[test]
    fn imports_bfloat16_and_scalar_weights() {
        let dir = tempfile::tempdir().unwrap();
        let half = [0x80, 0x3f, 0x00, 0xc0];
        let scalar = 0.25_f32.to_le_bytes();
        let tensors = [
            (
                "linear.weight",
                safetensors::tensor::TensorView::new(Dtype::BF16, vec![1, 2], &half).unwrap(),
            ),
            (
                "tau",
                safetensors::tensor::TensorView::new(Dtype::F32, vec![], &scalar).unwrap(),
            ),
        ];
        safetensors::tensor::serialize_to_file(
            tensors,
            None,
            &dir.path().join("model.safetensors"),
        )
        .unwrap();
        let weights = Weights::open(dir.path()).unwrap();
        assert_eq!(weights.values("linear.weight").unwrap(), [1.0, -2.0]);
        assert_eq!(weights.scalar("tau").unwrap(), 0.25);
        assert_eq!(
            weights
                .tensor::<2>("linear.weight", &Device::flex())
                .unwrap()
                .dims(),
            [1, 2]
        );
        assert!(
            weights
                .tensor::<1>("linear.weight", &Device::flex())
                .is_err()
        );
    }

    #[test]
    fn rejects_shard_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("model.safetensors.index.json"),
            r#"{"weight_map":{"weight":"../secret.safetensors"}}"#,
        )
        .unwrap();
        assert!(Weights::open(dir.path()).is_err());
    }

    #[test]
    fn rejects_missing_indexed_tensor_and_duplicate_shard_tensors() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = 1.0_f32.to_le_bytes();
        for name in ["a.safetensors", "b.safetensors"] {
            let tensor = safetensors::tensor::TensorView::new(Dtype::F32, vec![1], &bytes).unwrap();
            safetensors::tensor::serialize_to_file(
                [("weight", tensor)],
                None,
                &dir.path().join(name),
            )
            .unwrap();
        }
        assert!(Weights::open(dir.path()).is_err());
        std::fs::write(
            dir.path().join("model.safetensors.index.json"),
            r#"{"weight_map":{"weight":"a.safetensors","missing":"a.safetensors"}}"#,
        )
        .unwrap();
        assert!(Weights::open(dir.path()).is_err());
    }
}
