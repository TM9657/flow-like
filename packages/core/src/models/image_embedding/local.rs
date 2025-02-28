use super::ImageEmbeddingModelLogic;
use crate::{
    bit::{Bit, BitTypes},
    models::{embedding::EmbeddingModelLogic, embedding_factory::EmbeddingFactory},
    state::{FlowLikeState, FlowLikeStore},
};
use anyhow::Result;
use async_trait::async_trait;
use fastembed::{ImageEmbedding, ImageInitOptionsUserDefined, UserDefinedImageEmbeddingModel};
use std::sync::Arc;
use text_splitter::{MarkdownSplitter, TextSplitter};

use tokio::sync::Mutex;

pub struct LocalImageEmbeddingModel {
    pub bit: Bit,
    image_embedding_model: fastembed::ImageEmbedding,
    text_model: Arc<dyn EmbeddingModelLogic>,
}

impl LocalImageEmbeddingModel {
    pub async fn new(
        bit: &Bit,
        app_state: Arc<Mutex<FlowLikeState>>,
        factory: &mut EmbeddingFactory,
    ) -> anyhow::Result<Arc<Self>> {
        let bit_store = FlowLikeState::bit_store(&app_state).await?;

        let bit_store = match bit_store {
            FlowLikeStore::Local(store) => store,
            _ => return Err(anyhow::anyhow!("Only local store supported")),
        };

        let pack = bit.pack(app_state.clone()).await?;
        pack.download(app_state.clone()).await?;
        let embedding_model = pack
            .bits
            .iter()
            .find(|b| b.bit_type == BitTypes::Embedding)
            .ok_or(anyhow::anyhow!("Embedding model not found."))?;
        let preprocessor_bit = pack
            .bits
            .iter()
            .find(|b| b.bit_type == BitTypes::PreprocessorConfig)
            .ok_or(anyhow::anyhow!("Preprocessor bit not found."))?;
        let text_model = factory
            .build_text(embedding_model, app_state.clone())
            .await?;

        let model_path = bit
            .to_path(&bit_store)
            .ok_or(anyhow::anyhow!("No model path"))?;
        let loaded_model = std::fs::read(model_path)?;

        let preprocessor_path = preprocessor_bit
            .to_path(&bit_store)
            .ok_or(anyhow::anyhow!("No model path"))?;
        let loaded_preprocessor = std::fs::read(preprocessor_path)?;

        let user_embedding_model =
            UserDefinedImageEmbeddingModel::new(loaded_model, loaded_preprocessor);
        let init_options = ImageInitOptionsUserDefined::new();

        let loaded_model =
            ImageEmbedding::try_new_from_user_defined(user_embedding_model, init_options)?;

        let default_return_model = LocalImageEmbeddingModel {
            bit: bit.clone(),
            image_embedding_model: loaded_model,
            text_model,
        };

        Ok(Arc::new(default_return_model))
    }
}

#[async_trait]
impl ImageEmbeddingModelLogic for LocalImageEmbeddingModel {
    async fn get_splitter(
        &self,
    ) -> anyhow::Result<(
        TextSplitter<tokenizers::Tokenizer>,
        MarkdownSplitter<tokenizers::Tokenizer>,
    )> {
        return self.text_model.get_splitter().await;
    }

    async fn text_embed_query(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>> {
        return self.text_model.text_embed_query(texts).await;
    }

    async fn text_embed_document(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>> {
        return self.text_model.text_embed_document(texts).await;
    }

    async fn image_embed(&self, image_paths: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let embeddings = match self.image_embedding_model.embed(image_paths, None) {
            Ok(embeddings) => embeddings,
            Err(e) => {
                println!("Error embedding image: {}", e);
                return Err(anyhow::anyhow!("Error embedding image"));
            }
        };

        Ok(embeddings)
    }
}
