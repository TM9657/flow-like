use flow_like_types::{
    Cacheable, Result, bail,
    reqwest::{self, Url},
    utils::data_url::pathbuf_to_data_url,
};
use futures::StreamExt;
use local_store::LocalObjectStore;
use object_store::{ObjectStore, path::Path, signer::Signer};
use std::{sync::Arc, time::Duration};
pub mod local_store;

#[derive(Clone)]
pub enum FlowLikeStore {
    Local(Arc<LocalObjectStore>),
    AWS(Arc<object_store::aws::AmazonS3>),
    Azure(Arc<object_store::azure::MicrosoftAzure>),
    Google(Arc<object_store::gcp::GoogleCloudStorage>),
    Other(Arc<dyn ObjectStore>),
}

impl Cacheable for FlowLikeStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl FlowLikeStore {
    pub fn as_generic(&self) -> Arc<dyn ObjectStore> {
        match self {
            FlowLikeStore::Local(store) => store.clone() as Arc<dyn ObjectStore>,
            FlowLikeStore::AWS(store) => store.clone() as Arc<dyn ObjectStore>,
            FlowLikeStore::Azure(store) => store.clone() as Arc<dyn ObjectStore>,
            FlowLikeStore::Google(store) => store.clone() as Arc<dyn ObjectStore>,
            FlowLikeStore::Other(store) => store.clone() as Arc<dyn ObjectStore>,
        }
    }

    pub async fn sign(&self, method: &str, path: &Path, expires_after: Duration) -> Result<Url> {
        let method = match method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "PUT" => reqwest::Method::PUT,
            "POST" => reqwest::Method::POST,
            "DELETE" => reqwest::Method::DELETE,
            _ => bail!("Invalid HTTP Method"),
        };

        let url: Url = match self {
            FlowLikeStore::AWS(store) => store.signed_url(method, path, expires_after).await?,
            FlowLikeStore::Google(store) => store.signed_url(method, path, expires_after).await?,
            FlowLikeStore::Azure(store) => store.signed_url(method, path, expires_after).await?,
            FlowLikeStore::Local(store) => {
                let local_path = store.path_to_filesystem(path)?;
                println!("Local path: {:?}", local_path);
                let data_url = pathbuf_to_data_url(&local_path).await?;
                return Ok(Url::parse(&data_url)?);
            }
            FlowLikeStore::Other(_) => bail!("Sign not implemented for this store"),
        };

        Ok(url)
    }

    pub async fn hash(&self, path: &Path) -> Result<String> {
        let store = self.as_generic();
        let meta = store.head(path).await?;

        if let Some(hash) = meta.e_tag {
            return Ok(hash);
        }

        let mut hash = blake3::Hasher::new();
        let mut reader = store.get(path).await?.into_stream();

        while let Some(data) = reader.next().await {
            let data = data?;
            hash.update(&data);
        }

        let finalized = hash.finalize();
        let finalized = finalized.to_hex().to_lowercase().to_string();
        Ok(finalized)
    }
}
