use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use object_store::path::Path;
use object_store::{ObjectStore, PutPayload};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

///Write a Serde Serializable Struct to compressed file using bitcode + lz4
pub async fn compress_to_file<T>(
    store: Arc<dyn ObjectStore>,
    file_path: Path,
    input: &T,
) -> anyhow::Result<()>
where
    T: Serialize + Deserialize<'static>,
{
    println!("Compressing to file: {:?}", file_path);
    let data = bitcode::serialize(input)?;
    println!("Data transformed {} bytes", data.len());
    let compressed = compress_prepend_size(&data);
    println!("Compressed {} bytes", compressed.len());
    let _result = store.put(&file_path, PutPayload::from(compressed)).await?;
    Ok(())
}

pub async fn compress_to_file_json<T>(
    store: Arc<dyn ObjectStore>,
    file_path: Path,
    input: &T,
) -> anyhow::Result<()>
where
    T: Serialize + Deserialize<'static>,
{
    let data = serde_json::to_vec(input)?;
    let compressed = compress_prepend_size(&data);
    let _result = store.put(&file_path, PutPayload::from(compressed)).await?;
    Ok(())
}

/// Read from a compressed file and deserialize it into a Serde Deserializable Struct
pub async fn from_compressed<T>(store: Arc<dyn ObjectStore>, file_path: Path) -> anyhow::Result<T>
where
    T: Serialize + DeserializeOwned,
{
    let reader = store.get(&file_path).await?;
    let bytes = reader.bytes().await?;
    let data = decompress_size_prepended(&bytes)?;

    let data: T = bitcode::deserialize(&data)?;
    Ok(data)
}

pub async fn from_compressed_json<T>(store: Arc<dyn ObjectStore>, file_path: Path) -> anyhow::Result<T>
where
    T: Serialize + DeserializeOwned,
{
    let reader = store.get(&file_path).await?;
    let bytes = reader.bytes().await?;
    let data = decompress_size_prepended(&bytes)?;

    let data: T = serde_json::from_slice(&data)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestStruct {
        value1: Vec<u8>,
        value2: String,
        value3: u64,
        value4: Vec<u8>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestArray {
        variables: Vec<TestStruct>,
    }

    #[tokio::test]
    async fn bitcode_serde_value() {
        let s = TestStruct {
            value1: serde_json::to_vec(&serde_json::json!({"key1": "value1"})).unwrap(),
            value2: "value2".to_string(),
            value3: 3,
            value4: serde_json::to_vec(&serde_json::json!(1)).unwrap(),
        };

        let ser = bitcode::serialize(&s).unwrap();
        let deser: TestStruct = bitcode::deserialize(&ser).unwrap();
        assert_eq!(s, deser);
    }

    #[tokio::test]
    async fn bitcode_serde_array() {
        let mut array = Vec::new();
        array.push(TestStruct {
            value1: serde_json::to_vec(&serde_json::json!({"key1": "value1"})).unwrap(),
            value2: "value2".to_string(),
            value3: 3,
            value4: serde_json::to_vec(&serde_json::json!(1)).unwrap(),
        });

        array.push(TestStruct {
            value1: serde_json::to_vec(&serde_json::json!("Gimme Cake")).unwrap(),
            value2: "value3".to_string(),
            value3: 4,
            value4: serde_json::to_vec(&serde_json::json!(false)).unwrap(),
        });

        let s = TestArray { variables: array };

        let ser = bitcode::serialize(&s).unwrap();
        let deser: TestArray = bitcode::deserialize(&ser).unwrap();

        assert_eq!(s, deser);
    }
}
