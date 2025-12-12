use crate::data::path::FlowPath;
use crate::image::NodeImage;
use flow_like::{
    bit::Bit,
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::{
    history::{Content, ContentType, History, HistoryMessage, ImageUrl, MessageContent, Role},
    llm::ModelLogic,
};
use flow_like_storage::Path;
use flow_like_types::image::ImageReader;
use flow_like_types::{Bytes, async_trait, base64::Engine, json::json, tokio::sync::RwLock};
use markitdown::{
    ConversionOptions, Document, ExtractedImage, MarkItDown,
    error::MarkitdownError,
    llm::{LlmClient, LlmConfig},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::Arc;

/// Represents a single page extracted from a document
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DocumentPage {
    pub page_number: u32,
    pub content: String,
    pub images: Vec<NodeImage>,
}

impl DocumentPage {
    pub fn new(page_number: u32, content: String, images: Vec<NodeImage>) -> Self {
        Self {
            page_number,
            content,
            images,
        }
    }
}

/// Combines an array of DocumentPages into a single markdown string
pub fn pages_to_markdown(pages: &[DocumentPage]) -> String {
    let mut md = String::new();

    for (i, page) in pages.iter().enumerate() {
        if pages.len() > 1 {
            md.push_str(&format!("\n---\n## Page {}\n\n", page.page_number));
        }
        md.push_str(&page.content);
        if i < pages.len() - 1 {
            md.push('\n');
        }
    }

    md
}

/// Converts a markitdown Document to DocumentPages, caching images
async fn document_to_pages(
    document: &Document,
    context: &mut ExecutionContext,
) -> flow_like_types::Result<Vec<DocumentPage>> {
    let mut pages = Vec::with_capacity(document.pages.len());
    println!(
        "[MARKITDOWN DEBUG] Processing {} pages to DocumentPages",
        document.pages.len()
    );

    for page in &document.pages {
        let content = page.to_markdown();
        let mut images = Vec::new();
        println!(
            "[MARKITDOWN DEBUG] Page {} has {} images",
            page.page_number,
            page.images().len()
        );
        println!(
            "[MARKITDOWN DEBUG] Page {} content length: {} bytes",
            page.page_number,
            content.len()
        );
        println!(
            "[MARKITDOWN DEBUG] Page {} content preview: {}",
            page.page_number,
            if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.clone()
            }
        );

        for extracted in page.images() {
            if let Some(node_image) = extracted_image_to_node_image(extracted, context).await? {
                images.push(node_image);
            }
        }

        pages.push(DocumentPage::new(page.page_number, content, images));
    }

    Ok(pages)
}

/// Converts a markitdown ExtractedImage to a NodeImage by decoding and caching
async fn extracted_image_to_node_image(
    extracted: &ExtractedImage,
    context: &mut ExecutionContext,
) -> flow_like_types::Result<Option<NodeImage>> {
    if extracted.data.is_empty() {
        return Ok(None);
    }

    let cursor = Cursor::new(extracted.data.as_ref());
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| flow_like_types::anyhow!("Failed to guess image format: {}", e))?;

    let dynamic_image = reader
        .decode()
        .map_err(|e| flow_like_types::anyhow!("Failed to decode image: {}", e))?;

    let node_image = NodeImage::new(context, dynamic_image).await;
    Ok(Some(node_image))
}

struct FlowLikeLlmClient {
    model: Arc<RwLock<Arc<dyn ModelLogic>>>,
    model_name: String,
    config: LlmConfig,
}

impl FlowLikeLlmClient {
    fn new(model: Arc<dyn ModelLogic>, model_name: String) -> Self {
        Self {
            model: Arc::new(RwLock::new(model)),
            model_name,
            config: LlmConfig::default(),
        }
    }

    fn build_image_data_url(&self, image_data: &[u8], mime_type: &str) -> String {
        let base64 = flow_like_types::base64::engine::general_purpose::STANDARD.encode(image_data);
        format!("data:{};base64,{}", mime_type, base64)
    }

    async fn invoke_with_image(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        image_data_url: &str,
    ) -> Result<String, MarkitdownError> {
        println!(
            "[MARKITDOWN DEBUG] invoke_with_image called with user_prompt: {}",
            user_prompt
        );
        let mut history = History::new(
            self.model_name.clone(),
            vec![
                HistoryMessage {
                    role: Role::System,
                    content: MessageContent::String(system_prompt.to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    annotations: None,
                },
                HistoryMessage {
                    role: Role::User,
                    content: MessageContent::Contents(vec![
                        Content::Image {
                            content_type: ContentType::ImageUrl,
                            image_url: ImageUrl {
                                url: image_data_url.to_string(),
                                detail: Some("auto".to_string()),
                            },
                        },
                        Content::Text {
                            content_type: ContentType::Text,
                            text: user_prompt.to_string(),
                        },
                    ]),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    annotations: None,
                },
            ],
        );
        history.temperature = Some(self.config.temperature as f32);

        println!(
            "[MARKITDOWN DEBUG] About to invoke model: {}",
            self.model_name
        );
        let model = self.model.read().await;
        model.transform_history(&mut history);

        println!(
            "[MARKITDOWN DEBUG] History after transform: messages={}",
            history.messages.len()
        );
        println!("[MARKITDOWN DEBUG] Calling model.invoke()...");
        println!(
            "[MARKITDOWN DEBUG] Image data URL length: {}",
            image_data_url.len()
        );
        let response = model
            .invoke(&history, None)
            .await
            .map_err(|e| {
                let error_msg = format!("{}", e);
                println!("[MARKITDOWN ERROR] Model invocation failed: {}", error_msg);
                println!("[MARKITDOWN ERROR] Model name: {}", self.model_name);

                if error_msg.contains("JsonError") || error_msg.contains("expected value at line 1") {
                    println!("[MARKITDOWN ERROR] CRITICAL: The model returned invalid JSON. This usually means:");
                    println!("[MARKITDOWN ERROR] 1. The model doesn't support vision/image inputs");
                    println!("[MARKITDOWN ERROR] 2. The model provider's API returned an error (check API quotas/limits)");
                    println!("[MARKITDOWN ERROR] 3. The image is too large ({} chars in base64)", image_data_url.len());
                    println!("[MARKITDOWN ERROR] Try using a vision-capable model like GPT-4o, Claude 3.5 Sonnet, or Gemini Pro Vision");
                }

                MarkitdownError::LlmError(error_msg)
            })?;

        println!("[MARKITDOWN DEBUG] Model invocation successful, extracting text...");
        println!(
            "[MARKITDOWN DEBUG] Response has {} choices",
            response.choices.len()
        );
        let text = response
            .choices
            .first()
            .and_then(|c| {
                println!(
                    "[MARKITDOWN DEBUG] First choice message content: {:?}",
                    c.message.content
                );
                c.message.content.clone()
            })
            .unwrap_or_default();

        println!(
            "[MARKITDOWN DEBUG] LLM response text (length: {}): {}",
            text.len(),
            text
        );

        Ok(text)
    }

    async fn invoke_text(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, MarkitdownError> {
        let mut history = History::new(
            self.model_name.clone(),
            vec![
                HistoryMessage {
                    role: Role::System,
                    content: MessageContent::String(system_prompt.to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    annotations: None,
                },
                HistoryMessage {
                    role: Role::User,
                    content: MessageContent::String(user_prompt.to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    annotations: None,
                },
            ],
        );
        history.temperature = Some(self.config.temperature as f32);

        let model = self.model.read().await;
        let response = model
            .invoke(&history, None)
            .await
            .map_err(|e| MarkitdownError::LlmError(e.to_string()))?;

        let text = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(text)
    }
}

#[async_trait]
impl LlmClient for FlowLikeLlmClient {
    async fn describe_image(
        &self,
        image_data: &[u8],
        mime_type: &str,
    ) -> Result<String, MarkitdownError> {
        println!(
            "[MARKITDOWN DEBUG] describe_image called with mime_type: {}, data size: {} bytes",
            mime_type,
            image_data.len()
        );
        let data_url = self.build_image_data_url(image_data, mime_type);
        self.invoke_with_image(
            &self.config.image_description_prompt,
            "Describe this image in detail.",
            &data_url,
        )
        .await
    }

    async fn describe_image_base64(
        &self,
        base64_data: &str,
        mime_type: &str,
    ) -> Result<String, MarkitdownError> {
        println!(
            "[MARKITDOWN DEBUG] describe_image_base64 called with mime_type: {}, base64 length: {}",
            mime_type,
            base64_data.len()
        );
        let data_url = format!("data:{};base64,{}", mime_type, base64_data);
        self.invoke_with_image(
            &self.config.image_description_prompt,
            "Describe this image in detail.",
            &data_url,
        )
        .await
    }

    async fn describe_images_batch(
        &self,
        images: &[(&[u8], &str)],
    ) -> Result<Vec<String>, MarkitdownError> {
        let mut results = Vec::with_capacity(images.len());
        for (data, mime_type) in images {
            let desc = self.describe_image(*data, *mime_type).await?;
            results.push(desc);
        }
        Ok(results)
    }

    async fn convert_page_image(
        &self,
        image_data: &[u8],
        mime_type: &str,
    ) -> Result<String, MarkitdownError> {
        println!(
            "[MARKITDOWN DEBUG] convert_page_image called with mime_type: {}, data size: {} bytes",
            mime_type,
            image_data.len()
        );
        let data_url = self.build_image_data_url(image_data, mime_type);
        self.invoke_with_image(
            &self.config.page_conversion_prompt,
            "Convert this document page to clean, well-structured markdown.",
            &data_url,
        )
        .await
    }

    async fn complete(&self, prompt: &str) -> Result<String, MarkitdownError> {
        self.invoke_text("You are a helpful assistant.", prompt)
            .await
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ExtractDocumentNode {}

impl ExtractDocumentNode {
    pub fn new() -> Self {
        ExtractDocumentNode {}
    }
}

#[async_trait]
impl NodeLogic for ExtractDocumentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_processing_extract_document",
            "Extract Document",
            "Extracts text and content from documents (PDF, DOCX, XLSX, images, etc.) and converts to markdown.",
            "AI/Processing",
        );
        node.add_icon("/flow/icons/file-text.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(8)
                .set_governance(10)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Execution trigger to start document extraction.",
            VariableType::Execution,
        );

        node.add_input_pin(
            "file",
            "File",
            "Document file to extract (PDF, DOCX, XLSX, images, etc.).",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "extract_images",
            "Extract Images",
            "Whether to extract and embed images from the document.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution output after extraction completes.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "pages",
            "Pages",
            "Extracted document pages with content and images.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<DocumentPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let file: FlowPath = context.evaluate_pin("file").await?;
        let extract_images: bool = context.evaluate_pin("extract_images").await?;

        let file_path = Path::from(file.path.clone());
        let extension = file_path
            .extension()
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        let file_buffer = file.get(context, false).await?;
        let bytes = Bytes::from(file_buffer);

        let md = MarkItDown::new();
        let options = ConversionOptions::default()
            .with_extension(&extension)
            .with_images(extract_images);

        let result = md
            .convert_bytes(bytes, Some(options))
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to extract document: {}", e))?;

        let pages = document_to_pages(&result, context).await?;

        context.set_pin_value("pages", json!(pages)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ExtractDocumentAiNode {}

impl ExtractDocumentAiNode {
    pub fn new() -> Self {
        ExtractDocumentAiNode {}
    }
}

#[async_trait]
impl NodeLogic for ExtractDocumentAiNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_processing_extract_document_ai",
            "AI Extract Document",
            "Extracts text and content from documents using AI for enhanced image descriptions and OCR.",
            "AI/Processing",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(5)
                .set_security(5)
                .set_performance(6)
                .set_governance(5)
                .set_reliability(7)
                .set_cost(4)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Execution trigger to start AI-powered document extraction.",
            VariableType::Execution,
        );

        node.add_input_pin(
            "file",
            "File",
            "Document file to extract (PDF, DOCX, XLSX, images, etc.).",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "model",
            "Model",
            "Vision-capable AI model for image analysis and OCR.",
            VariableType::Struct,
        )
        .set_schema::<Bit>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "extract_images",
            "Extract Images",
            "Whether to extract and embed images from the document.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution output after extraction completes.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "pages",
            "Pages",
            "Extracted document pages with AI-generated descriptions and images.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<DocumentPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let file: FlowPath = context.evaluate_pin("file").await?;
        let model_bit = context.evaluate_pin::<Bit>("model").await?;
        let extract_images: bool = context.evaluate_pin("extract_images").await?;

        let file_path = Path::from(file.path.clone());
        let extension = file_path
            .extension()
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        let file_buffer = file.get(context, false).await?;
        let bytes = Bytes::from(file_buffer);

        let model_factory = context.app_state.model_factory.clone();
        let model = model_factory
            .lock()
            .await
            .build(&model_bit, context.app_state.clone(), context.token.clone())
            .await?;

        let default_model = model
            .default_model()
            .await
            .unwrap_or_else(|| "gpt-4o".to_string());

        let llm_client: Arc<dyn LlmClient> = Arc::new(FlowLikeLlmClient::new(model, default_model));

        println!(
            "[MARKITDOWN DEBUG] Starting AI document extraction with extension: {}, extract_images: {}",
            extension, extract_images
        );
        let md = MarkItDown::new();
        let options = ConversionOptions::default()
            .with_extension(&extension)
            .with_images(extract_images)
            .with_llm(llm_client);

        println!(
            "[MARKITDOWN DEBUG] Calling convert_bytes with file size: {} bytes",
            bytes.len()
        );
        let result = md
            .convert_bytes(bytes, Some(options))
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to extract document with AI: {}", e))?;

        println!(
            "[MARKITDOWN DEBUG] Conversion complete, processing {} pages",
            result.pages.len()
        );
        let pages = document_to_pages(&result, context).await?;

        context.set_pin_value("pages", json!(pages)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ExtractDocumentsNode {}

impl ExtractDocumentsNode {
    pub fn new() -> Self {
        ExtractDocumentsNode {}
    }
}

#[async_trait]
impl NodeLogic for ExtractDocumentsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_processing_extract_documents",
            "Extract Documents",
            "Extracts text and content from multiple documents in parallel.",
            "AI/Processing",
        );
        node.add_icon("/flow/icons/files.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(9)
                .set_governance(10)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Execution trigger to start batch document extraction.",
            VariableType::Execution,
        );

        node.add_input_pin(
            "files",
            "Files",
            "Array of document files to extract.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "extract_images",
            "Extract Images",
            "Whether to extract and embed images from documents.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution output after all extractions complete.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "results",
            "Results",
            "Array of extracted document pages for each file.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<Vec<DocumentPage>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let files: Vec<FlowPath> = context.evaluate_pin("files").await?;
        let extract_images: bool = context.evaluate_pin("extract_images").await?;

        let md = MarkItDown::new();
        let mut all_results: Vec<Vec<DocumentPage>> = Vec::with_capacity(files.len());

        for file in files {
            let file_path = Path::from(file.path.clone());
            let extension = file_path
                .extension()
                .map(|e| format!(".{}", e))
                .unwrap_or_default();

            let file_buffer = file.get(context, false).await?;
            let bytes = Bytes::from(file_buffer);

            let options = ConversionOptions::default()
                .with_extension(&extension)
                .with_images(extract_images);

            let result = md
                .convert_bytes(bytes, Some(options))
                .await
                .map_err(|e| flow_like_types::anyhow!("Failed to extract document: {}", e))?;

            let pages = document_to_pages(&result, context).await?;
            all_results.push(pages);
        }

        context.set_pin_value("results", json!(all_results)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ExtractDocumentsAiNode {}

impl ExtractDocumentsAiNode {
    pub fn new() -> Self {
        ExtractDocumentsAiNode {}
    }
}

#[async_trait]
impl NodeLogic for ExtractDocumentsAiNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_processing_extract_documents_ai",
            "AI Extract Documents",
            "Extracts text and content from multiple documents using AI in parallel.",
            "AI/Processing",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(5)
                .set_security(5)
                .set_performance(7)
                .set_governance(5)
                .set_reliability(7)
                .set_cost(3)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Execution trigger to start AI-powered batch extraction.",
            VariableType::Execution,
        );

        node.add_input_pin(
            "files",
            "Files",
            "Array of document files to extract.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "model",
            "Model",
            "Vision-capable AI model for image analysis and OCR.",
            VariableType::Struct,
        )
        .set_schema::<Bit>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "extract_images",
            "Extract Images",
            "Whether to extract and embed images from documents.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution output after all extractions complete.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "results",
            "Results",
            "Array of extracted document pages with AI descriptions for each file.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<Vec<DocumentPage>>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let files: Vec<FlowPath> = context.evaluate_pin("files").await?;
        let model_bit = context.evaluate_pin::<Bit>("model").await?;
        let extract_images: bool = context.evaluate_pin("extract_images").await?;

        let model_factory = context.app_state.model_factory.clone();
        let model = model_factory
            .lock()
            .await
            .build(&model_bit, context.app_state.clone(), context.token.clone())
            .await?;

        let default_model = model
            .default_model()
            .await
            .unwrap_or_else(|| "gpt-4o".to_string());

        let llm_client: Arc<dyn LlmClient> = Arc::new(FlowLikeLlmClient::new(model, default_model));

        let md = MarkItDown::new();
        let mut all_results: Vec<Vec<DocumentPage>> = Vec::with_capacity(files.len());

        for file in files {
            let file_path = Path::from(file.path.clone());
            let extension = file_path
                .extension()
                .map(|e| format!(".{}", e))
                .unwrap_or_default();

            let file_buffer = file.get(context, false).await?;
            let bytes = Bytes::from(file_buffer);

            let options = ConversionOptions::default()
                .with_extension(&extension)
                .with_images(extract_images)
                .with_llm(llm_client.clone());

            let result = md.convert_bytes(bytes, Some(options)).await.map_err(|e| {
                flow_like_types::anyhow!("Failed to extract document with AI: {}", e)
            })?;

            let pages = document_to_pages(&result, context).await?;
            all_results.push(pages);
        }

        context.set_pin_value("results", json!(all_results)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct PagesToMarkdownNode {}

impl PagesToMarkdownNode {
    pub fn new() -> Self {
        PagesToMarkdownNode {}
    }
}

#[async_trait]
impl NodeLogic for PagesToMarkdownNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_processing_pages_to_markdown",
            "Pages to Markdown",
            "Combines an array of document pages into a single markdown string.",
            "AI/Processing",
        );
        node.add_icon("/flow/icons/file-text.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(10)
                .set_security(10)
                .set_performance(10)
                .set_governance(10)
                .set_reliability(10)
                .set_cost(10)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Execution trigger to combine pages.",
            VariableType::Execution,
        );

        node.add_input_pin(
            "pages",
            "Pages",
            "Array of document pages to combine.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<DocumentPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution output after combining pages.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "markdown",
            "Markdown",
            "Combined markdown content from all pages.",
            VariableType::String,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let pages: Vec<DocumentPage> = context.evaluate_pin("pages").await?;
        let markdown = pages_to_markdown(&pages);

        context.set_pin_value("markdown", json!(markdown)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
