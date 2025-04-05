use flow_like_storage::Path;
use flow_like_storage::object_store::{ObjectStore, PutPayload};
use flow_like_types::Message;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

///Write a Serde Serializable Struct to compressed file using bitcode + lz4
pub async fn compress_to_file<T>(
    store: Arc<dyn ObjectStore>,
    file_path: Path,
    input: &T,
) -> flow_like_types::Result<()>
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
) -> flow_like_types::Result<()>
where
    T: Serialize + Deserialize<'static>,
{
    let data = flow_like_types::json::to_vec(input)?;
    let compressed = compress_prepend_size(&data);
    let _result = store.put(&file_path, PutPayload::from(compressed)).await?;
    Ok(())
}

/// Read from a compressed file and deserialize it into a Serde Deserializable Struct
pub async fn from_compressed<T>(
    store: Arc<dyn ObjectStore>,
    file_path: Path,
) -> flow_like_types::Result<T>
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
) -> flow_like_types::Result<T>
where
    T: Serialize + DeserializeOwned,
{
    let reader = store.get(&file_path).await?;
    let bytes = reader.bytes().await?;
    let data = decompress_size_prepended(&bytes)?;

    let data: T = flow_like_types::json::from_slice(&data)?;
    Ok(data)
}
