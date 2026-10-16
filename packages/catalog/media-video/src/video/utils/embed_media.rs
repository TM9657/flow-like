use super::*;
use flow_like_catalog_embedding::CachedEmbeddingModel;
use flow_like_model_provider::embedding::interface::{
    EmbeddingBatch, EmbeddingOptions, EmbeddingPurpose,
};
#[cfg(feature = "execute")]
use {
    super::prepare_embedding_media::{
        EmbeddingMediaKind, decode_embedding_media, embedding_media_kind,
    },
    flow_like_model_provider::embedding::interface::{
        EmbeddingDescriptor, EmbeddingError, EmbeddingFile, EmbeddingInput, EmbeddingModality,
        EmbeddingPart, EmbeddingRequest,
    },
    flow_like_storage::object_store::ObjectStoreExt,
};

fn media_embedding_node(video: bool) -> Node {
    let (name, label, method) = if video {
        ("embed_video", "Embed Video", "embedVideo")
    } else {
        ("embed_audio", "Embed Audio", "embedAudio")
    };
    let mut node = Node::new(
        name,
        label,
        "Embeds a file using its extension to select text, image, audio, video, or PDF input",
        "AI/Embedding",
    );
    node.set_flowscript_name("ai.embedding", method);
    node.set_version(1);
    node.set_long_running(true);
    node.add_icon("/flow/icons/bot-invoke.svg");
    node.set_scores(
        NodeScores::new()
            .set_privacy(6)
            .set_security(6)
            .set_performance(7)
            .set_governance(6)
            .set_reliability(7)
            .set_cost(6)
            .build(),
    );
    add_exec_pins(&mut node);
    add_flow_path_input(
        &mut node,
        "source",
        "Source",
        "A media, PDF, or UTF-8 text file; its extension selects the embedding modality",
    );
    node.add_input_pin(
        "model",
        "Model",
        "A loaded embedding model that supports the detected file modality",
        VariableType::Struct,
    )
    .set_schema::<CachedEmbeddingModel>()
    .set_options(PinOptions::new().set_enforce_schema(true).build());
    node.add_input_pin(
        "purpose",
        "Purpose",
        "The embedding task supported by the model",
        VariableType::String,
    )
    .set_schema::<EmbeddingPurpose>()
    .set_default_value(Some(json!(EmbeddingPurpose::Document)))
    .set_options(
        PinOptions::new()
            .set_valid_values(vec![
                "query".into(),
                "document".into(),
                "similarity".into(),
                "classification".into(),
                "clustering".into(),
                "code_query".into(),
            ])
            .build(),
    );
    node.add_input_pin(
        "options",
        "Options",
        "Optional output dimensions supported by the model",
        VariableType::Struct,
    )
    .set_schema::<EmbeddingOptions>()
    .set_default_value(Some(json!(EmbeddingOptions::default())))
    .set_options(PinOptions::new().set_enforce_schema(true).build());
    if video {
        node.add_input_pin(
            "max_frames",
            "Max Frames",
            "Maximum uniformly sampled frames for local models; hosted models sample frames on the server",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(16)));
        node.add_input_pin(
            "include_audio",
            "Include Audio",
            "Include a video's audio track; missing audio returns an error",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));
    }
    node.add_output_pin(
        "vector",
        "Vector",
        "The file embedding",
        VariableType::Float,
    )
    .set_value_type(ValueType::Array);
    node.add_output_pin(
        "result",
        "Result",
        "The vector with its embedding space, model revision, and usage",
        VariableType::Struct,
    )
    .set_schema::<EmbeddingBatch>()
    .set_options(PinOptions::new().set_enforce_schema(true).build());
    node
}

#[cfg(feature = "execute")]
fn validate_media_request(
    descriptor: &EmbeddingDescriptor,
    kind: EmbeddingMediaKind,
    purpose: EmbeddingPurpose,
    options: &EmbeddingOptions,
) -> flow_like_types::Result<()> {
    descriptor.validate(&EmbeddingRequest {
        items: vec![],
        purpose,
        options: options.clone(),
    })?;
    let required = match kind {
        EmbeddingMediaKind::Text => vec![EmbeddingModality::Text],
        EmbeddingMediaKind::Image => vec![EmbeddingModality::Image],
        EmbeddingMediaKind::Audio => vec![EmbeddingModality::Audio],
        EmbeddingMediaKind::File => vec![EmbeddingModality::File],
        EmbeddingMediaKind::Video {
            include_audio: true,
            ..
        } => vec![EmbeddingModality::Audio, EmbeddingModality::Video],
        EmbeddingMediaKind::Video { .. } => vec![EmbeddingModality::Video],
    };
    for modality in &required {
        if !descriptor.modalities.contains(modality) {
            return Err(EmbeddingError::UnsupportedModality(*modality).into());
        }
    }
    if required.len() > 1
        && !descriptor.joint_combinations.iter().any(|allowed| {
            let mut allowed = allowed.clone();
            allowed.sort();
            allowed.dedup();
            allowed == required
        })
    {
        return Err(EmbeddingError::UnsupportedCombination(required).into());
    }
    Ok(())
}

#[cfg(feature = "execute")]
async fn run_media_embedding(
    context: &mut ExecutionContext,
    video: bool,
) -> flow_like_types::Result<()> {
    context.deactivate_exec_pin("exec_out").await?;
    let handle: CachedEmbeddingModel = context.evaluate_pin("model").await?;
    let model = handle.resolve_model(context).await?;
    let source: FlowPath = context.evaluate_pin("source").await?;
    let purpose: EmbeddingPurpose = context.evaluate_pin("purpose").await?;
    let options: EmbeddingOptions = context.evaluate_pin("options").await?;
    let mut kind = embedding_media_kind(&source.object_path(), 16, false)?;
    if video && matches!(kind, EmbeddingMediaKind::Video { .. }) {
        let max_frames: i64 = context.evaluate_pin("max_frames").await?;
        if !(1..=1024).contains(&max_frames) {
            return Err(flow_like_types::anyhow!(
                "Max Frames must be between 1 and 1024"
            ));
        }
        kind = EmbeddingMediaKind::Video {
            max_frames: max_frames as usize,
            include_audio: context.evaluate_pin("include_audio").await?,
        };
    }
    validate_media_request(model.descriptor(), kind, purpose, &options)?;
    let (store, path) = flow_path_object(context, &source).await?;
    let input = if model.supports_encoded_media() && kind != EmbeddingMediaKind::Text {
        let typed_sources = model.requires_typed_media_sources();
        encoded_media_input_with_types(store.as_ref(), &path, kind, typed_sources).await?
    } else {
        decode_embedding_media(store.as_ref(), &path, kind)
            .await?
            .into_input()?
    };
    let dimensions = options
        .dimensions
        .unwrap_or(model.descriptor().space.dimensions);
    let request = EmbeddingRequest {
        items: vec![input],
        purpose,
        options,
    };
    model.descriptor().validate(&request)?;
    let result = model.embed(request).await?;
    if result.embeddings.len() != 1
        || dimensions == 0
        || result.space.dimensions != dimensions
        || result.embeddings[0].len() != result.space.dimensions
        || result.embeddings[0].iter().any(|value| !value.is_finite())
    {
        return Err(flow_like_types::anyhow!(
            "Model must return one finite vector with its declared dimensions for the file"
        ));
    }
    context
        .set_pin_value("vector", json!(result.embeddings[0]))
        .await?;
    context.set_pin_value("result", json!(result)).await?;
    context.activate_exec_pin("exec_out").await?;
    Ok(())
}

#[cfg(feature = "execute")]
pub(super) async fn encoded_media_input(
    store: &dyn ObjectStore,
    path: &ObjectPath,
    kind: EmbeddingMediaKind,
) -> flow_like_types::Result<EmbeddingInput> {
    encoded_media_input_with_types(store, path, kind, false).await
}

#[cfg(feature = "execute")]
async fn encoded_media_input_with_types(
    store: &dyn ObjectStore,
    path: &ObjectPath,
    kind: EmbeddingMediaKind,
    typed_sources: bool,
) -> flow_like_types::Result<EmbeddingInput> {
    use flow_like_types::{
        base64::{Engine, engine::general_purpose::STANDARD},
        futures::StreamExt,
    };

    const MAX_MEDIA_BYTES: usize = 100 * 1024 * 1024;
    let extension = path.extension().unwrap_or_default().to_ascii_lowercase();
    let mime = match (kind, extension.as_str()) {
        (EmbeddingMediaKind::Image, "png") => Some("image/png"),
        (EmbeddingMediaKind::Image, "jpg" | "jpeg" | "jpe") => Some("image/jpeg"),
        (EmbeddingMediaKind::Audio, "wav" | "wave") => Some("audio/wav"),
        (EmbeddingMediaKind::Audio, "mp3") => Some("audio/mpeg"),
        (EmbeddingMediaKind::Video { .. }, "mp4" | "m4v") => Some("video/mp4"),
        (EmbeddingMediaKind::Video { .. }, "mov") => Some("video/quicktime"),
        (EmbeddingMediaKind::File, "pdf") => Some("application/pdf"),
        _ => None,
    };
    if typed_sources
        && mime.is_none()
        && !matches!(kind, EmbeddingMediaKind::Image | EmbeddingMediaKind::Audio)
    {
        return Err(flow_like_types::anyhow!(
            "This hosted model requires MP4 or MOV video"
        ));
    }
    let object = store.get(path).await?;
    if object.meta.size > MAX_MEDIA_BYTES as u64 {
        return Err(flow_like_types::anyhow!(
            "Hosted embedding media exceeds 100 MiB"
        ));
    }
    let mut stream = object.into_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len().saturating_add(chunk.len()) > MAX_MEDIA_BYTES {
            return Err(flow_like_types::anyhow!(
                "Hosted embedding media exceeds 100 MiB"
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    if typed_sources && mime.is_none() {
        return super::prepare_embedding_media::decode_hosted_image_or_audio(
            Bytes::from(bytes),
            path.clone(),
            kind,
        )
        .await;
    }
    let modality = match kind {
        EmbeddingMediaKind::Image => EmbeddingModality::Image,
        EmbeddingMediaKind::Audio => EmbeddingModality::Audio,
        EmbeddingMediaKind::Video { .. } => EmbeddingModality::Video,
        EmbeddingMediaKind::File => {
            return Ok(EmbeddingInput {
                parts: vec![EmbeddingPart::File(EmbeddingFile {
                    source: format!("data:application/pdf;base64,{}", STANDARD.encode(&bytes)),
                    mime_type: "application/pdf".into(),
                    filename: path.filename().map(str::to_owned),
                })],
                title: None,
            });
        }
        EmbeddingMediaKind::Text => {
            return Err(flow_like_types::anyhow!("Text must be decoded as UTF-8"));
        }
    };
    let mut parts = vec![EmbeddingPart::EncodedMedia {
        modality,
        source: if typed_sources {
            format!(
                "data:{};base64,{}",
                mime.expect("typed media MIME checked"),
                STANDARD.encode(&bytes)
            )
        } else {
            STANDARD.encode(&bytes)
        },
    }];
    if matches!(
        kind,
        EmbeddingMediaKind::Video {
            include_audio: true,
            ..
        }
    ) {
        // Keep the soundtrack on the same timeline and use the bounded video bytes.
        let audio = super::prepare_embedding_media::decode_hosted_video_audio(
            Bytes::from(bytes),
            path.clone(),
        )
        .await?
        .into_audio()?;
        parts.push(EmbeddingPart::Audio(audio));
    }
    Ok(EmbeddingInput { parts, title: None })
}

#[cfg(not(feature = "execute"))]
async fn run_media_embedding(
    _context: &mut ExecutionContext,
    _video: bool,
) -> flow_like_types::Result<()> {
    Err(execute_feature_error())
}

#[crate::register_node]
#[derive(Default)]
pub struct EmbedAudioNode;

impl EmbedAudioNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for EmbedAudioNode {
    fn get_node(&self) -> Node {
        media_embedding_node(false)
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        run_media_embedding(context, false).await
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct EmbedVideoNode;

impl EmbedVideoNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for EmbedVideoNode {
    fn get_node(&self) -> Node {
        media_embedding_node(true)
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        run_media_embedding(context, true).await
    }
}

#[cfg(all(test, feature = "execute"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn typed_sources_keep_mime_and_pdf_metadata_without_changing_internal_sources() {
        let store = flow_like_storage::object_store::memory::InMemory::new();
        let image = ObjectPath::from("photo.jpg");
        store
            .put(&image, Bytes::from_static(b"image-bytes").into())
            .await
            .unwrap();
        let internal = encoded_media_input(&store, &image, EmbeddingMediaKind::Image)
            .await
            .unwrap();
        let hosted =
            encoded_media_input_with_types(&store, &image, EmbeddingMediaKind::Image, true)
                .await
                .unwrap();
        let EmbeddingPart::EncodedMedia {
            source: internal, ..
        } = &internal.parts[0]
        else {
            panic!("expected image");
        };
        let EmbeddingPart::EncodedMedia { source: hosted, .. } = &hosted.parts[0] else {
            panic!("expected image");
        };
        assert_eq!(hosted, &format!("data:image/jpeg;base64,{internal}"));
        let bitmap = ObjectPath::from("photo.bmp");
        let mut encoded = std::io::Cursor::new(Vec::new());
        flow_like_types::image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut encoded, flow_like_types::image::ImageFormat::Bmp)
            .unwrap();
        store
            .put(&bitmap, Bytes::from(encoded.into_inner()).into())
            .await
            .unwrap();
        let converted =
            encoded_media_input_with_types(&store, &bitmap, EmbeddingMediaKind::Image, true)
                .await
                .unwrap();
        let EmbeddingPart::Image(image) = &converted.parts[0] else {
            panic!("expected decoded bitmap");
        };
        assert_eq!((image.width(), image.height()), (2, 3));
        let pdf = ObjectPath::from("docs/report.pdf");
        store
            .put(&pdf, Bytes::from_static(b"%PDF-1.7").into())
            .await
            .unwrap();
        let input = encoded_media_input_with_types(&store, &pdf, EmbeddingMediaKind::File, true)
            .await
            .unwrap();
        let EmbeddingPart::File(file) = &input.parts[0] else {
            panic!("expected PDF");
        };
        assert!(file.source.starts_with("data:application/pdf;base64,"));
        assert_eq!(file.mime_type, "application/pdf");
        assert_eq!(file.filename.as_deref(), Some("report.pdf"));
        assert!(
            decode_embedding_media(&store, &pdf, EmbeddingMediaKind::File)
                .await
                .is_err()
        );
    }
}
