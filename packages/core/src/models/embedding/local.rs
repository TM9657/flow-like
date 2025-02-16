use super::EmbeddingModelLogic;
use crate::{
    bit::{Bit, BitPack, BitTypes, Pooling},
    state::FlowLikeState,
    utils::{local_object_store::LocalObjectStore, tokenizer::load_tokenizer_from_file},
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fastembed::{InitOptionsUserDefined, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel};
use std::sync::Arc;
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};
use tokio::sync::Mutex;

pub struct LocalEmbeddingModel {
    pub bit: Bit,
    embedding_model: fastembed::TextEmbedding,
    tokenizer_files: TokenizerFiles,
}

impl LocalEmbeddingModel {
    pub async fn new(bit: &Bit, app_state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<Arc<Self>> {
        let bit_store = app_state
            .lock()
            .await
            .config
            .read()
            .await
            .local_store
            .clone()
            .ok_or(anyhow::anyhow!("No local store"))?;
        let pack = bit.pack(app_state.clone()).await;
        pack.download(app_state.clone()).await?;

        let model_path = bit
            .to_path(&bit_store)
            .ok_or(anyhow::anyhow!("No model path"))?;
        let loaded_model = std::fs::read(model_path)?;
        let loaded_tokenizer = load_tokenizer(&pack, &bit_store).await?;

        let mut pooling = fastembed::Pooling::Mean;

        let params = bit
            .try_to_embedding()
            .ok_or(anyhow::anyhow!("Not an Embedding Model"))?;

        if params.pooling == Pooling::CLS {
            pooling = fastembed::Pooling::Cls;
        }

        let user_embedding_model =
            UserDefinedEmbeddingModel::new(loaded_model, loaded_tokenizer.clone())
                .with_pooling(pooling);
        let init_options =
            InitOptionsUserDefined::new().with_max_length(params.input_length as usize);

        let loaded_model =
            TextEmbedding::try_new_from_user_defined(user_embedding_model, init_options)?;

        let default_return_model = LocalEmbeddingModel {
            bit: bit.clone(),
            embedding_model: loaded_model,
            tokenizer_files: loaded_tokenizer,
        };

        Ok(Arc::new(default_return_model))
    }
}

#[async_trait]
impl EmbeddingModelLogic for LocalEmbeddingModel {
    async fn get_splitter(
        &self,
    ) -> anyhow::Result<(
        TextSplitter<tokenizers::Tokenizer>,
        MarkdownSplitter<tokenizers::Tokenizer>,
    )> {
        let params = self
            .bit
            .try_to_embedding()
            .ok_or(anyhow::anyhow!("Not an Embedding Model"))?;
        let max_tokens = params.input_length as usize;

        let tokenizer = load_tokenizer_from_file(self.tokenizer_files.clone(), max_tokens)?;
        let config_md = ChunkConfig::new(max_tokens)
            .with_sizer(tokenizer.clone())
            .with_overlap(20)?;

        let config = ChunkConfig::new(max_tokens)
            .with_sizer(tokenizer)
            .with_overlap(20)?;

        return Ok((TextSplitter::new(config), MarkdownSplitter::new(config_md)));
    }

    async fn text_embed_query(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>> {
        let params = match self.bit.try_to_embedding() {
            Some(params) => params,
            None => {
                println!("Error getting embedding params");
                return Err(anyhow::anyhow!("Error getting embedding params"));
            }
        };

        let prefixed_array = texts
            .iter()
            .map(|text| format!("{}{}", params.prefix.query, text))
            .collect::<Vec<String>>();

        let embeddings = match self.embedding_model.embed(prefixed_array.to_vec(), None) {
            Ok(embeddings) => embeddings,
            Err(e) => {
                println!("Error embedding text: {}", e);
                return Err(anyhow::anyhow!("Error embedding text"));
            }
        };
        Ok(embeddings)
    }

    async fn text_embed_document(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>> {
        let params = match self.bit.try_to_embedding() {
            Some(params) => params,
            None => {
                println!("Error getting embedding params");
                return Err(anyhow::anyhow!("Error getting embedding params"));
            }
        };

        let prefixed_array = texts
            .iter()
            .map(|text| format!("{}{}", params.prefix.paragraph, text))
            .collect::<Vec<String>>();
        let embeddings = match self.embedding_model.embed(prefixed_array, None) {
            Ok(embeddings) => embeddings,
            Err(e) => {
                println!("Error embedding text: {}", e);
                return Err(anyhow::anyhow!("Error embedding text"));
            }
        };
        Ok(embeddings)
    }
}

async fn load_tokenizer(
    pack: &BitPack,
    model_path: &Arc<LocalObjectStore>,
) -> Result<TokenizerFiles> {
    let config_bit = pack.bits.iter().find(|b| b.bit_type == BitTypes::Config);
    let tokenizer_bit = pack.bits.iter().find(|b| b.bit_type == BitTypes::Tokenizer);
    let tokenizer_config_bit = pack
        .bits
        .iter()
        .find(|b| b.bit_type == BitTypes::TokenizerConfig);
    let special_tokens_bit = pack
        .bits
        .iter()
        .find(|b| b.bit_type == BitTypes::SpecialTokensMap);

    if config_bit.is_none()
        || tokenizer_bit.is_none()
        || tokenizer_config_bit.is_none()
        || special_tokens_bit.is_none()
    {
        return Err(anyhow::anyhow!("Error loading tokenizer files"));
    }

    let config_bit = config_bit
        .ok_or(anyhow!("Config Bit not found"))?
        .to_path(model_path)
        .ok_or(anyhow::anyhow!("Config Bit Path not Found"))?;
    let tokenizer_bit = tokenizer_bit
        .ok_or(anyhow!("Tokenizer Bit not found"))?
        .to_path(model_path)
        .ok_or(anyhow::anyhow!("Tokenizer Bit Path not Found"))?;
    let tokenizer_config_bit = tokenizer_config_bit
        .ok_or(anyhow!("Tokenizer Config Bit now found"))?
        .to_path(model_path)
        .ok_or(anyhow::anyhow!("Tokenizer Config Bit Path not Found"))?;
    let special_tokens_bit = special_tokens_bit
        .ok_or(anyhow!("Special Tokens Bit not found"))?
        .to_path(model_path)
        .ok_or(anyhow::anyhow!("Special Token Bit Path not Found"))?;

    Ok(TokenizerFiles {
        tokenizer_file: std::fs::read(tokenizer_bit).unwrap_or(Vec::new()),
        config_file: std::fs::read(config_bit).unwrap_or(Vec::new()),
        special_tokens_map_file: std::fs::read(special_tokens_bit).unwrap_or(Vec::new()),
        tokenizer_config_file: std::fs::read(tokenizer_config_bit).unwrap_or(Vec::new()),
    })
}
