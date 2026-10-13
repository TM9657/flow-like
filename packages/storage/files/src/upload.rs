use bytes::Bytes;
use futures::{StreamExt, TryStreamExt, stream};
use object_store::{
    Error, ObjectStore, ObjectStoreExt, PutPayload, PutPayloadMut, PutResult, Result, path::Path,
};

const UPLOAD_CHUNK_SIZE: usize = 64 * 1024;
const MULTIPART_PART_SIZE: usize = 8 * 1024 * 1024;
const MAX_UPLOAD_PARTS: usize = 10_000;
const MAX_CONCURRENT_PARTS: usize = 4;

/// Write an object using bounded transport buffers and automatic multipart uploads.
///
/// Small objects use one PUT. Objects above 8 MiB use multipart uploads when the
/// store supports them. Slices share the input allocation, so preparing requests
/// does not copy the file contents.
pub async fn put_payload(
    store: &dyn ObjectStore,
    path: &Path,
    payload: PutPayload,
) -> Result<PutResult> {
    let len = payload.content_length();
    if len <= MULTIPART_PART_SIZE {
        return store.put(path, transport_payload(payload)).await;
    }

    let mut upload = match store.put_multipart(path).await {
        Ok(upload) => upload,
        // SMB and HTTP stores accept ordinary PUTs without multipart support.
        Err(Error::NotSupported { .. } | Error::NotImplemented { .. }) => {
            return store.put(path, transport_payload(payload)).await;
        }
        Err(error) => return Err(error),
    };

    let result = stream::iter(upload_parts(payload, multipart_part_size(len)))
        .map(|part| upload.put_part(transport_payload(part)))
        .buffer_unordered(MAX_CONCURRENT_PARTS)
        .try_collect::<Vec<_>>()
        .await;

    // Drop pending part futures before aborting. Preserve the write error even
    // if cleanup also fails, since it explains why the object was not written.
    if let Err(error) = result {
        let _ = upload.abort().await;
        return Err(error);
    }

    match upload.complete().await {
        Ok(result) => Ok(result),
        Err(error) => {
            let _ = upload.abort().await;
            Err(error)
        }
    }
}

fn transport_payload(payload: PutPayload) -> PutPayload {
    // object_store's native HTTP client emits each Bytes as a body frame.
    // Multipart parts must be several MiB, but their buffers can remain small.
    payload
        .into_iter()
        .flat_map(|bytes| {
            (0..bytes.len())
                .step_by(UPLOAD_CHUNK_SIZE)
                .map(move |start| {
                    bytes.slice(start..start.saturating_add(UPLOAD_CHUNK_SIZE).min(bytes.len()))
                })
        })
        .collect()
}

fn multipart_part_size(len: usize) -> usize {
    // S3 allows at most 10,000 parts. Round up to a transport-buffer boundary,
    // keeping all non-final parts the same size for S3-compatible stores.
    len.div_ceil(MAX_UPLOAD_PARTS)
        .max(MULTIPART_PART_SIZE)
        .div_ceil(UPLOAD_CHUNK_SIZE)
        * UPLOAD_CHUNK_SIZE
}

fn upload_parts(payload: PutPayload, part_size: usize) -> impl Iterator<Item = PutPayload> {
    let mut chunks = payload.into_iter();
    let mut pending = Bytes::new();
    std::iter::from_fn(move || {
        let mut part = PutPayloadMut::new();
        while part.content_length() < part_size {
            if pending.is_empty() {
                match chunks.next() {
                    Some(chunk) => pending = chunk,
                    None => break,
                }
            }
            let len = pending.len().min(part_size - part.content_length());
            if len > 0 {
                part.push(pending.split_to(len));
            }
        }
        (!part.is_empty()).then(|| part.freeze())
    })
}

#[cfg(test)]
mod tests;
