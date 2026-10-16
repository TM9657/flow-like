//! OpenRouter's embedding content dialect is distinct from its chat dialect.

use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};

enum Part<'a> {
    Text(&'a str),
    Image(&'a str),
    Audio(&'a str),
    Video(&'a str),
    File { source: &'a str, mime_type: &'a str },
}

fn parts(input: &HostedEmbeddingInput) -> Vec<Part<'_>> {
    use HostedEmbeddingStructuredInput as Input;
    match input {
        HostedEmbeddingInput::Text(text)
        | HostedEmbeddingInput::Structured(Input::Text { text }) => vec![Part::Text(text)],
        HostedEmbeddingInput::Structured(Input::Image { image }) => vec![Part::Image(image)],
        HostedEmbeddingInput::Structured(Input::Audio { audio }) => vec![Part::Audio(audio)],
        HostedEmbeddingInput::Structured(Input::Video { video }) => vec![Part::Video(video)],
        HostedEmbeddingInput::Structured(Input::File {
            source, mime_type, ..
        }) => {
            vec![Part::File { source, mime_type }]
        }
        HostedEmbeddingInput::Structured(Input::Multimodal {
            text,
            image,
            audio,
            video,
        }) => {
            let mut parts = Vec::new();
            if let Some(text) = text {
                parts.push(Part::Text(text));
            }
            if let Some(image) = image {
                parts.push(Part::Image(image));
            }
            if let Some(audio) = audio {
                parts.push(Part::Audio(audio));
            }
            if let Some(video) = video {
                parts.push(Part::Video(video));
            }
            parts
        }
        HostedEmbeddingInput::Structured(Input::Content { parts }) => parts
            .iter()
            .map(|part| match part {
                HostedEmbeddingPart::Text { text } => Part::Text(text),
                HostedEmbeddingPart::Image { source } => Part::Image(source),
                HostedEmbeddingPart::Audio { source } => Part::Audio(source),
                HostedEmbeddingPart::Video { source } => Part::Video(source),
                HostedEmbeddingPart::File {
                    source, mime_type, ..
                } => Part::File { source, mime_type },
            })
            .collect(),
    }
}

pub(super) fn validate_request(
    provider: &EmbeddingModelProvider,
    config: &RemoteExecutionConfig,
    payload: &EmbedRequest,
) -> Result<(), ApiError> {
    let multimodal = nonempty(config.model_id.as_deref()) == Some("google/gemini-embedding-2");
    if multimodal && !(1..=8192).contains(&provider.input_length) {
        return Err(ApiError::bad_request(
            "Gemini Embedding 2 input length must be between 1 and 8192 tokens",
        ));
    }
    let mut media_bytes = 0_usize;
    for input in &payload.input {
        let parts = parts(input);
        if parts.is_empty() || parts.len() > 64 {
            return Err(ApiError::bad_request(
                "An OpenRouter embedding item requires between 1 and 64 content parts",
            ));
        }
        let mut counts = [0; 4];
        let mut text_bytes = 0_usize;
        let mut has_text = false;
        for part in &parts {
            if let Part::Text(text) = part {
                if text.is_empty() {
                    return Err(ApiError::bad_request(
                        "Embedding text parts must be nonempty",
                    ));
                }
                text_bytes = text_bytes.saturating_add(text.len());
                has_text = true;
                continue;
            }
            if !multimodal {
                return Err(ApiError::bad_request(
                    "OpenRouter media embeddings require google/gemini-embedding-2",
                ));
            }
            let (source, allowed, default, index): (&str, &[&str], &str, usize) = match part {
                Part::Image(source) => (source, &["image/png", "image/jpeg"], "image/png", 0),
                Part::Audio(source) => (
                    source,
                    &["audio/wav", "audio/x-wav", "audio/mpeg", "audio/mp3"],
                    "audio/wav",
                    1,
                ),
                Part::Video(source) => (source, &["video/mp4", "video/quicktime"], "video/mp4", 2),
                Part::File { source, mime_type } => {
                    if *mime_type != "application/pdf" {
                        return Err(ApiError::bad_request(
                            "Gemini Embedding 2 file inputs must use application/pdf",
                        ));
                    }
                    (source, &["application/pdf"], "application/pdf", 3)
                }
                Part::Text(_) => unreachable!(),
            };
            counts[index] += 1;
            media_bytes = media_bytes.saturating_add(validate_media(source)?);
            if is_media_url(source) {
                if index != 0 {
                    return Err(ApiError::bad_request(
                        "OpenRouter audio, video and PDF embeddings require inline base64 data; use a data URL instead of an HTTP URL",
                    ));
                }
                let url = Url::parse(source)
                    .map_err(|_| ApiError::bad_request("Invalid embedding image URL"))?;
                if !url.username().is_empty() || url.password().is_some() {
                    return Err(ApiError::bad_request(
                        "Embedding image URLs must not contain credentials",
                    ));
                }
            } else {
                let (mime, encoded) = inline_data(source, default);
                if !allowed.contains(&mime) {
                    return Err(ApiError::bad_request(
                        "Unsupported OpenRouter embedding media MIME type",
                    ));
                }
                // Validate the encoding before reserving credits or dispatching upstream.
                STANDARD.decode(encoded).map_err(|_| {
                    ApiError::bad_request("Embedding media contains invalid base64")
                })?;
            }
        }
        if counts[0] > 6 || counts[1..].iter().any(|count| *count > 1) {
            return Err(ApiError::bad_request(
                "Gemini Embedding 2 accepts up to six images and one audio, video or PDF file of each type per item",
            ));
        }
        if has_text && text_bytes.saturating_add(prefix(provider, payload).len()) > MAX_INPUT_BYTES
        {
            return Err(ApiError::bad_request(
                "Embedding text must be at most 100,000 bytes including the model prefix",
            ));
        }
    }
    if media_bytes > MAX_TOTAL_ENCODED_MEDIA_BYTES {
        return Err(ApiError::bad_request(
            "Embedding batch exceeds the 256 MiB inline media limit",
        ));
    }
    Ok(())
}

fn inline_data<'a>(source: &'a str, default_mime: &'a str) -> (&'a str, &'a str) {
    source
        .strip_prefix("data:")
        .and_then(|data| {
            let (header, encoded) = data.split_once(',')?;
            Some((header.strip_suffix(";base64")?, encoded))
        })
        .unwrap_or((default_mime, source))
}

fn inline_source(source: &str, default_mime: &str) -> serde_json::Value {
    let (mime, encoded) = inline_data(source, default_mime);
    let format = match mime {
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "video/quicktime" => "mov",
        "application/pdf" => "pdf",
        _ => "mp4",
    };
    serde_json::json!({"data": format!("data:{mime};base64,{encoded}"), "format": format})
}

#[derive(Deserialize)]
struct OpenRouterEmbeddingObject {
    index: Option<usize>,
    embedding: Vec<f32>,
}

pub(super) fn decode_response(
    bytes: &[u8],
    expected_count: usize,
    expected_dimensions: usize,
) -> Result<HostedEmbeddingResult, ApiError> {
    let response: UpstreamResponse<OpenRouterEmbeddingObject> = serde_json::from_slice(bytes)
        .map_err(|_| ApiError::internal("Embedding provider returned an invalid response"))?;
    let indexed = response
        .data
        .iter()
        .filter(|item| item.index.is_some())
        .count();
    if indexed != 0 && indexed != response.data.len() {
        return Err(ApiError::internal(
            "Embedding provider returned inconsistent vector indexes",
        ));
    }
    // OpenRouter permits omitted indexes. A wholly unindexed response keeps
    // array order; indexed responses use the shared duplicate and bounds checks.
    normalize_response(
        UpstreamResponse {
            data: response
                .data
                .into_iter()
                .enumerate()
                .map(|(position, item)| EmbeddingObject {
                    index: item.index.unwrap_or(position),
                    embedding: item.embedding,
                })
                .collect(),
            model: response.model,
            id: response.id,
            request_id: response.request_id,
            usage: response.usage,
        },
        expected_count,
        expected_dimensions,
    )
}

pub(super) fn request_body(
    provider: &EmbeddingModelProvider,
    config: &RemoteExecutionConfig,
    payload: &EmbedRequest,
) -> serde_json::Value {
    let inputs: Vec<_> = payload.input.iter().map(parts).collect();
    let has_media = inputs
        .iter()
        .flatten()
        .any(|part| !matches!(part, Part::Text(_)));
    let input: Vec<serde_json::Value> = inputs.into_iter().map(|parts| {
        let mut prefix_pending = true;
        let mut text = String::new();
        let content: Vec<_> = parts.into_iter().map(|part| match part {
            Part::Text(value) => {
                let value = if prefix_pending {
                    prefix_pending = false;
                    format!("{}{value}", prefix(provider, payload))
                } else { value.to_owned() };
                text.push_str(&value);
                serde_json::json!({"type": "text", "text": value})
            }
            Part::Image(source) => {
                let url = if is_media_url(source) || source.starts_with("data:") {
                    source.to_owned()
                } else { format!("data:image/png;base64,{source}") };
                serde_json::json!({"type": "image_url", "image_url": {"url": url}})
            }
            Part::Audio(source) => serde_json::json!({"type": "input_audio", "input_audio": inline_source(source, "audio/wav")}),
            Part::Video(source) => serde_json::json!({"type": "input_video", "input_video": inline_source(source, "video/mp4")}),
            Part::File { source, mime_type } => serde_json::json!({"type": "input_file", "input_file": inline_source(source, mime_type)}),
        }).collect();
        if has_media { serde_json::json!({"content": content}) } else { text.into() }
    }).collect();
    serde_json::json!({
        "model": nonempty(config.model_id.as_deref()),
        "input": input,
        "encoding_format": "float",
        "dimensions": provider.vector_length,
        "input_type": match payload.embed_type {
            EmbedType::Query => "search_query",
            EmbedType::Document => "search_document",
        },
    })
}
