use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use object_store::path::Path;
use object_store::{ObjectStore, PutPayload};
use prost::Message;
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
    T: Message,
{
    let mut data = Vec::new();
    input.encode(&mut data)?;
    let compressed = compress_prepend_size(&data);
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
    T: Message + Default,
{
    let reader = store.get(&file_path).await?;
    let bytes = reader.bytes().await?;
    let data = decompress_size_prepended(&bytes)?;

    let message = T::decode(&data[..])?;
    Ok(message)
}

pub async fn from_compressed_json<T>(
    store: Arc<dyn ObjectStore>,
    file_path: Path,
) -> anyhow::Result<T>
where
    T: Serialize + DeserializeOwned,
{
    let reader = store.get(&file_path).await?;
    let bytes = reader.bytes().await?;
    let data = decompress_size_prepended(&bytes)?;

    let data: T = serde_json::from_slice(&data)?;
    Ok(data)
}
