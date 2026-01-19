use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct Attachment {
    pub filename: Option<String>,
    pub content_type: String,
    pub data: Vec<u8>,
}
