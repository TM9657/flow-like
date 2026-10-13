use async_trait::async_trait;
use bytes::Bytes;
use flow_like_storage_files::{object_store, upload::put_payload};
use futures::TryStreamExt;
use object_store::{
    ClientOptions, ObjectStoreExt, PutPayload, RetryConfig,
    aws::{AmazonS3, AmazonS3Builder},
    client::{
        HttpClient, HttpConnector, HttpError, HttpErrorKind, HttpRequest, HttpResponse,
        HttpResponseBody, HttpService,
    },
    path::Path,
};
use std::sync::{Arc, Mutex};

const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug)]
struct RecordedPut {
    content_length: usize,
    payload_hash: String,
    frame_sizes: Vec<usize>,
    body: Vec<u8>,
    rejected: bool,
}

#[derive(Clone, Debug, Default)]
struct BoundedTransport(Arc<Mutex<Vec<RecordedPut>>>);

impl HttpConnector for BoundedTransport {
    fn connect(&self, _: &ClientOptions) -> object_store::Result<HttpClient> {
        Ok(HttpClient::new(self.clone()))
    }
}

#[async_trait]
impl HttpService for BoundedTransport {
    async fn call(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let (parts, body) = request.into_parts();
        assert_eq!(parts.method, "PUT");
        assert!(parts.uri.path().ends_with("/uploads/regression.pdf"));
        assert!(parts.uri.query().is_none());
        assert!(parts.headers.contains_key("authorization"));

        let mut record = RecordedPut {
            content_length: parts.headers["content-length"]
                .to_str()
                .unwrap()
                .parse()
                .unwrap(),
            payload_hash: parts.headers["x-amz-content-sha256"]
                .to_str()
                .unwrap()
                .to_owned(),
            frame_sizes: Vec::new(),
            body: Vec::new(),
            rejected: false,
        };
        // Keep the request's frame boundaries while consuming its body.
        let mut frames = HttpResponseBody::new(body).bytes_stream();
        while let Some(frame) = frames.try_next().await? {
            record.frame_sizes.push(frame.len());
            if frame.len() > MAX_FRAME_BYTES {
                record.rejected = true;
                self.0.lock().unwrap().push(record);
                return Err(HttpError::new(
                    HttpErrorKind::Unknown,
                    std::io::Error::other("transport rejected an oversized HTTP body frame"),
                ));
            }
            record.body.extend_from_slice(&frame);
        }

        assert_eq!(record.body.len(), record.content_length);
        self.0.lock().unwrap().push(record);
        let mut response = HttpResponse::new(HttpResponseBody::from(Bytes::new()));
        response
            .headers_mut()
            .insert("etag", "\"uploaded-etag\"".parse().unwrap());
        response
            .headers_mut()
            .insert("x-amz-version-id", "uploaded-version".parse().unwrap());
        Ok(response)
    }
}

fn s3_store(transport: BoundedTransport) -> AmazonS3 {
    AmazonS3Builder::new()
        .with_bucket_name("test-bucket")
        .with_region("eu-west-1")
        .with_access_key_id("test-access-key")
        .with_secret_access_key("test-secret-key")
        .with_http_connector(transport)
        .with_retry(RetryConfig {
            max_retries: 0,
            ..Default::default()
        })
        .build()
        .unwrap()
}

#[tokio::test]
async fn large_single_put_uses_bounded_http_frames_without_changing_signed_content() {
    let transport = BoundedTransport::default();
    let store = s3_store(transport.clone());
    let path = Path::from("uploads/regression.pdf");
    let bytes = Bytes::from(
        (0..2_800_000)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let payload = PutPayload::from_bytes(bytes.clone());

    // This simulated transport rejects the old single-frame request.
    let error = store.put(&path, payload.clone()).await.unwrap_err();
    assert!(error.to_string().contains("oversized HTTP body frame"));

    let result = put_payload(&store, &path, payload).await.unwrap();
    assert_eq!(result.e_tag.as_deref(), Some("\"uploaded-etag\""));
    assert_eq!(result.version.as_deref(), Some("uploaded-version"));

    let records = transport.0.lock().unwrap();
    assert_eq!(records.len(), 2);
    assert!(records[0].rejected);
    assert_eq!(records[0].frame_sizes, [bytes.len()]);
    assert!(!records[1].rejected);
    assert!(records[1].frame_sizes.len() > 1);
    assert_eq!(records[1].content_length, bytes.len());
    assert_eq!(records[1].body.as_slice(), bytes.as_ref());
    assert_eq!(records[1].payload_hash.len(), 64);
    assert_eq!(records[1].payload_hash, records[0].payload_hash);
}

#[tokio::test]
async fn empty_and_small_s3_uploads_preserve_content_and_metadata() {
    for size in [0, 1, 1_400_000] {
        let transport = BoundedTransport::default();
        let store = s3_store(transport.clone());
        let bytes = Bytes::from(vec![42; size]);
        let result = put_payload(
            &store,
            &Path::from("uploads/regression.pdf"),
            PutPayload::from_bytes(bytes.clone()),
        )
        .await
        .unwrap();

        assert_eq!(result.e_tag.as_deref(), Some("\"uploaded-etag\""));
        assert_eq!(result.version.as_deref(), Some("uploaded-version"));
        let records = transport.0.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert!(!records[0].rejected);
        assert_eq!(records[0].content_length, size);
        assert_eq!(records[0].body.as_slice(), bytes.as_ref());
    }
}
