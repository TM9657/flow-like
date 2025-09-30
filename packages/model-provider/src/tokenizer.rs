use std::sync::Arc;

#[cfg(feature = "local-ml")]
use fastembed::TokenizerFiles;
use tokenizers::{AddedToken, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

#[cfg(feature = "local-ml")]
pub fn load_tokenizer_from_file(
    tokenizer_files: Arc<TokenizerFiles>,
    max_length: usize,
) -> flow_like_types::Result<Tokenizer> {
    let base_error_message =
        "Error building TokenizerFiles for UserDefinedEmbeddingModel. Could not read {} file.";

    // Serialize each tokenizer file
    let config: flow_like_types::Value =
        flow_like_types::json::from_slice(&tokenizer_files.config_file).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                base_error_message.replace("{}", "config.json"),
            )
        })?;
    let special_tokens_map: flow_like_types::Value = flow_like_types::json::from_slice(
        &tokenizer_files.special_tokens_map_file,
    )
    .map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            base_error_message.replace("{}", "special_tokens_map.json"),
        )
    })?;
    let tokenizer_config: flow_like_types::Value = flow_like_types::json::from_slice(
        &tokenizer_files.tokenizer_config_file,
    )
    .map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            base_error_message.replace("{}", "tokenizer_config.json"),
        )
    })?;
    let mut tokenizer: tokenizers::Tokenizer = tokenizers::Tokenizer::from_bytes(
        tokenizer_files.tokenizer_file.clone(),
    )
    .map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            base_error_message.replace("{}", "tokenizer.json"),
        )
    })?;

    //For BGEBaseSmall, the model_max_length value is set to 1000000000000000019884624838656. Which fits in a f64
    let model_max_length = tokenizer_config["model_max_length"]
        .as_f64()
        .expect("Error reading model_max_length from tokenizer_config.json")
        as f32;
    let max_length = max_length.min(model_max_length as usize);
    let pad_id = config["pad_token_id"].as_u64().unwrap_or(0) as u32;
    let pad_token = tokenizer_config["pad_token"]
        .as_str()
        .expect("Error reading pad_token from tokenier_config.json")
        .into();

    let mut tokenizer = tokenizer
        .with_padding(Some(PaddingParams {
            // TODO: the user should able to choose the padding strategy
            strategy: PaddingStrategy::BatchLongest,
            pad_token,
            pad_id,
            ..Default::default()
        }))
        .with_truncation(Some(TruncationParams {
            max_length,
            ..Default::default()
        }))
        .map_err(flow_like_types::Error::msg)?
        .clone();
    if let flow_like_types::Value::Object(root_object) = special_tokens_map {
        for (_, value) in root_object.iter() {
            if value.is_string() {
                tokenizer.add_special_tokens(&[AddedToken {
                    content: value.as_str().unwrap().into(),
                    special: true,
                    ..Default::default()
                }]);
            } else if value.is_object() {
                tokenizer.add_special_tokens(&[AddedToken {
                    content: value["content"].as_str().unwrap().into(),
                    special: true,
                    single_word: value["single_word"].as_bool().unwrap(),
                    lstrip: value["lstrip"].as_bool().unwrap(),
                    rstrip: value["rstrip"].as_bool().unwrap(),
                    normalized: value["normalized"].as_bool().unwrap(),
                }]);
            }
        }
    }
    Ok(tokenizer.into())
}
