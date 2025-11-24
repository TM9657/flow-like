use crate::bit::Bit;

pub mod embedding;
pub mod embedding_factory;
pub mod image_embedding;
pub mod llm;
pub mod local_utils;

pub trait ModelMeta: Send + Sync {
    fn get_bit(&self) -> Bit;
}
