use flow_like::{
    bit::Bit,
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
};
use flow_like_catalog_core::{FlowPath, NodeImage};
#[cfg(feature = "execute")]
use flow_like_storage::Path;
#[cfg(feature = "execute")]
use flow_like_types::Bytes;
#[cfg(feature = "execute")]
use flow_like_types::image::ImageReader;
use flow_like_types::{async_trait, json::json};
#[cfg(feature = "execute")]
use markitdown::{ConversionOptions, Document, ExtractedImage, MarkItDown, create_llm_client};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "execute")]
use std::io::Cursor;

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

#[cfg(feature = "execute")]
/// Converts a markitdown Document to DocumentPages, caching images
async fn document_to_pages(
    document: &Document,
    context: &mut ExecutionContext,
) -> flow_like_types::Result<Vec<DocumentPage>> {
    let mut pages = Vec::with_capacity(document.pages.len());

    for page in &document.pages {
        let content = page.to_markdown();
        let mut images = Vec::new();

        for extracted in page.images() {
            if let Some(node_image) = extracted_image_to_node_image(extracted, context).await? {
                images.push(node_image);
            }
        }

        pages.push(DocumentPage::new(page.page_number, content, images));
    }

    Ok(pages)
}

#[cfg(feature = "execute")]
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
    fn get_node(&self) -> Node {
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

    #[cfg(feature = "execute")]
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

        let options = ConversionOptions::default()
            .with_extension(&extension)
            .with_image_context_path(file.path)
            .with_images(extract_images);

        // Run the heavy document conversion in a cancellable task
        let result = context
            .run_cancellable(async move {
                let md = MarkItDown::new();
                md.convert_bytes(bytes, Some(options)).await
            })
            .await?
            .map_err(|e| flow_like_types::anyhow!("Failed to extract document: {}", e))?;

        let pages = document_to_pages(&result, context).await?;

        context.set_pin_value("pages", json!(pages)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Document processing requires the 'execute' feature"
        ))
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
    fn get_node(&self) -> Node {
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

    #[cfg(feature = "execute")]
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

        let _file_start = std::time::Instant::now();
        let file_buffer = file.get(context, false).await?;
        let bytes = Bytes::from(file_buffer);

        let model_factory = context.app_state.model_factory.clone();
        let _model_start = std::time::Instant::now();
        let model = model_factory
            .lock()
            .await
            .build(&model_bit, context.app_state.clone(), context.token.clone())
            .await?;

        let _llm_start = std::time::Instant::now();
        let completion_handle = model.completion_model_handle(None).await?;
        let llm_client = create_llm_client(completion_handle);

        let md = MarkItDown::new();
        let file_path_clone = file.path.clone();
        let mut options = ConversionOptions::default()
            .with_extension(&extension)
            .with_image_context_path(file.path)
            .with_images(extract_images)
            .with_llm(llm_client);
        options.url = Some(file_path_clone);

        let _convert_start = std::time::Instant::now();
        let result = md
            .convert_bytes(bytes, Some(options))
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to extract document with AI: {}", e))?;

        let _pages_start = std::time::Instant::now();
        let pages = document_to_pages(&result, context).await?;

        context.set_pin_value("pages", json!(pages)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Processing requires the 'execute' feature"
        ))
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
    fn get_node(&self) -> Node {
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
        .set_schema::<DocumentPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    #[cfg(feature = "execute")]
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
                .with_image_context_path(file.path)
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

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Processing requires the 'execute' feature"
        ))
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
    fn get_node(&self) -> Node {
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
        .set_schema::<DocumentPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    #[cfg(feature = "execute")]
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

        let completion_handle = model.completion_model_handle(None).await?;
        let llm_client = create_llm_client(completion_handle);

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
                .with_image_context_path(file.path.clone())
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

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Processing requires the 'execute' feature"
        ))
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
    fn get_node(&self) -> Node {
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

/// Detail level for document summarization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum SummaryDetailLevel {
    Low,
    Medium,
    High,
}

impl Default for SummaryDetailLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl SummaryDetailLevel {
    #[allow(dead_code)]
    fn target_ratio(&self) -> f32 {
        match self {
            Self::Low => 0.05,
            Self::Medium => 0.15,
            Self::High => 0.30,
        }
    }

    fn system_prompt(&self, include_toc: bool) -> String {
        let detail_instruction = match self {
            Self::Low => {
                "Create a very concise summary capturing only the most essential points. Focus on the main thesis, key conclusions, and critical takeaways. Omit details, examples, and supporting evidence."
            }
            Self::Medium => {
                "Create a balanced summary that covers main topics, key arguments, and important details. Include significant examples and supporting points while maintaining brevity."
            }
            Self::High => {
                "Create a comprehensive summary preserving most important information, including main topics, key arguments, supporting evidence, examples, and nuances. Maintain logical flow and relationships between concepts."
            }
        };

        let toc_instruction = if include_toc {
            "\n\nInclude a table of contents at the beginning with page references where each topic can be found. Format as:\n## Table of Contents\n- [Topic Name](#topic) (Pages X-Y)\n"
        } else {
            ""
        };

        format!(
            "You are a document summarization expert. {detail_instruction}{toc_instruction}\n\n\
            Guidelines:\n\
            - Preserve key terminology and domain-specific language\n\
            - Maintain factual accuracy\n\
            - Use clear, professional language\n\
            - Structure the summary logically\n\
            - Extract and highlight keywords that characterize the document's focus areas"
        )
    }
}

/// Result of document summarization containing the summary text and extracted keywords
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DocumentSummary {
    pub summary: String,
    pub keywords: Vec<String>,
    pub page_references: Vec<PageReference>,
}

/// Reference to content location within the document
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PageReference {
    pub topic: String,
    pub pages: Vec<u32>,
}

/// A content section with semantic grouping across pages
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContentSection {
    pub title: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub page_references: Vec<u32>,
}

#[cfg(feature = "execute")]
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

#[cfg(feature = "execute")]
fn chunk_pages_for_context(pages: &[DocumentPage], max_tokens: usize) -> Vec<Vec<&DocumentPage>> {
    let mut chunks = Vec::new();
    let mut current_chunk = Vec::new();
    let mut current_tokens = 0;

    for page in pages {
        let page_tokens = estimate_tokens(&page.content);
        if current_tokens + page_tokens > max_tokens && !current_chunk.is_empty() {
            chunks.push(current_chunk);
            current_chunk = Vec::new();
            current_tokens = 0;
        }
        current_chunk.push(page);
        current_tokens += page_tokens;
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

#[cfg(feature = "execute")]
fn format_pages_for_prompt(pages: &[&DocumentPage]) -> String {
    pages
        .iter()
        .map(|p| format!("[Page {}]\n{}", p.page_number, p.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

#[cfg(feature = "execute")]
async fn invoke_model_simple(
    context: &ExecutionContext,
    model_bit: &Bit,
    system_prompt: &str,
    user_prompt: &str,
) -> flow_like_types::Result<String> {
    invoke_model_simple_standalone(
        &context.app_state,
        &context.token,
        model_bit,
        system_prompt,
        user_prompt,
    )
    .await
}

#[cfg(feature = "execute")]
async fn invoke_model_simple_standalone(
    app_state: &std::sync::Arc<flow_like::state::FlowLikeState>,
    access_token: &Option<String>,
    model_bit: &Bit,
    system_prompt: &str,
    user_prompt: &str,
) -> flow_like_types::Result<String> {
    use flow_like_model_provider::history::{History, HistoryMessage, Role};

    let model_factory = app_state.model_factory.clone();
    let model = model_factory
        .lock()
        .await
        .build(model_bit, app_state.clone(), access_token.clone())
        .await?;

    let model_name = model_bit
        .meta
        .get("name")
        .map(|m| m.name.clone())
        .unwrap_or_else(|| model_bit.id.clone());

    let mut history = History::new(model_name, vec![]);
    history.set_system_prompt(system_prompt.to_string());
    history.push_message(HistoryMessage::from_string(Role::User, user_prompt));

    let response = model.invoke(&history, None).await?;

    response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .ok_or_else(|| flow_like_types::anyhow!("No response from model"))
}

#[crate::register_node]
#[derive(Default)]
pub struct SummarizeDocumentNode {}

impl SummarizeDocumentNode {
    pub fn new() -> Self {
        SummarizeDocumentNode {}
    }
}

#[async_trait]
impl NodeLogic for SummarizeDocumentNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "ai_processing_summarize_document",
            "Summarize Document",
            "Creates an intelligent summary of document pages using AI with configurable detail levels. Handles long documents via iterative summarization.",
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
            "Execution trigger to start summarization.",
            VariableType::Execution,
        );

        node.add_input_pin(
            "pages",
            "Pages",
            "Document pages to summarize.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<DocumentPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "model",
            "Model",
            "AI model to use for summarization.",
            VariableType::Struct,
        )
        .set_schema::<Bit>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "detail_level",
            "Detail Level",
            "Summary detail level: Low (very concise), Medium (balanced), High (comprehensive).",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "Low".to_string(),
                    "Medium".to_string(),
                    "High".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json!("Medium")));

        node.add_input_pin(
            "include_toc",
            "Include TOC",
            "Whether to include a table of contents with page references.",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "max_context_tokens",
            "Max Context Tokens",
            "Maximum tokens per summarization chunk (adjust based on model context window).",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(8000)));

        node.add_input_pin(
            "parallel_requests",
            "Parallel Requests",
            "Number of chunks to process in parallel. Set to 0 or chunks count to process all at once.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(4)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution output after summarization completes.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "summary",
            "Summary",
            "The generated document summary.",
            VariableType::Struct,
        )
        .set_schema::<DocumentSummary>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let pages: Vec<DocumentPage> = context.evaluate_pin("pages").await?;
        let model_bit: Bit = context.evaluate_pin("model").await?;
        let detail_str: String = context.evaluate_pin("detail_level").await?;
        let include_toc: bool = context.evaluate_pin("include_toc").await?;
        let max_context_tokens: i64 = context.evaluate_pin("max_context_tokens").await?;
        let parallel_requests: i64 = context.evaluate_pin("parallel_requests").await?;

        let detail_level = match detail_str.as_str() {
            "Low" => SummaryDetailLevel::Low,
            "High" => SummaryDetailLevel::High,
            _ => SummaryDetailLevel::Medium,
        };

        if pages.is_empty() {
            let empty_summary = DocumentSummary {
                summary: String::new(),
                keywords: vec![],
                page_references: vec![],
            };
            context
                .set_pin_value("summary", json!(empty_summary))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        let system_prompt = detail_level.system_prompt(include_toc);
        let chunks = chunk_pages_for_context(&pages, max_context_tokens as usize);
        let num_chunks = chunks.len();

        let concurrency = if parallel_requests <= 0 {
            num_chunks
        } else {
            (parallel_requests as usize).min(num_chunks)
        };

        let chunk_summaries: Vec<(String, Vec<u32>)> = if concurrency <= 1 {
            // Sequential processing - no merge needed for single chunk
            let mut results = Vec::with_capacity(num_chunks);
            for chunk in &chunks {
                let content = format_pages_for_prompt(chunk);
                let page_numbers: Vec<u32> = chunk.iter().map(|p| p.page_number).collect();
                let page_range = page_numbers
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let user_prompt = format!(
                    "Summarize the following document section (pages: {}).\n\n\
                    Provide:\n\
                    1. A structured summary\n\
                    2. Key keywords (comma-separated, prefixed with 'Keywords:')\n\
                    3. Important topics with their page numbers (if applicable)\n\n\
                    Content:\n{}",
                    page_range, content
                );

                let result =
                    invoke_model_simple(context, &model_bit, &system_prompt, &user_prompt).await?;
                results.push((result, page_numbers));
            }
            results
        } else {
            // Parallel processing
            use futures::stream::{self, StreamExt};
            use std::sync::Arc;

            let model_bit = Arc::new(model_bit.clone());
            let system_prompt = Arc::new(system_prompt.clone());
            let app_state = context.app_state.clone();
            let token = context.token.clone();

            let tasks: Vec<_> = chunks
                .iter()
                .map(|chunk| {
                    let content = format_pages_for_prompt(chunk);
                    let page_numbers: Vec<u32> = chunk.iter().map(|p| p.page_number).collect();
                    let page_range = page_numbers
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");

                    let user_prompt = format!(
                        "Summarize the following document section (pages: {}).\n\n\
                        Provide:\n\
                        1. A structured summary\n\
                        2. Key keywords (comma-separated, prefixed with 'Keywords:')\n\
                        3. Important topics with their page numbers (if applicable)\n\n\
                        Content:\n{}",
                        page_range, content
                    );

                    let model_bit = Arc::clone(&model_bit);
                    let system_prompt = Arc::clone(&system_prompt);
                    let app_state = app_state.clone();
                    let token = token.clone();

                    async move {
                        let result = invoke_model_simple_standalone(
                            &app_state,
                            &token,
                            &model_bit,
                            &system_prompt,
                            &user_prompt,
                        )
                        .await?;
                        Ok::<_, flow_like_types::Error>((result, page_numbers))
                    }
                })
                .collect();

            stream::iter(tasks)
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
        };

        let final_summary = if chunk_summaries.len() == 1 {
            chunk_summaries[0].0.clone()
        } else {
            let combined_summaries = chunk_summaries
                .iter()
                .enumerate()
                .map(|(i, (s, pages))| format!("[Section {} - Pages {:?}]\n{}", i + 1, pages, s))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n");

            let merge_prompt = format!(
                "Merge these section summaries into a single coherent document summary.\n\
                Maintain the same detail level and include all important keywords.\n\
                If a table of contents is present, consolidate it.\n\n\
                Section Summaries:\n{}",
                combined_summaries
            );

            invoke_model_simple(context, &model_bit, &system_prompt, &merge_prompt).await?
        };

        let (summary_text, keywords, page_refs) = parse_summary_response(&final_summary);

        let result = DocumentSummary {
            summary: summary_text,
            keywords,
            page_references: page_refs,
        };

        context.set_pin_value("summary", json!(result)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Document summarization requires the 'execute' feature"
        ))
    }
}

fn parse_summary_response(response: &str) -> (String, Vec<String>, Vec<PageReference>) {
    let mut keywords = Vec::new();
    let mut page_refs = Vec::new();
    let mut summary_lines = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("keywords:") {
            let kw_str = trimmed
                .strip_prefix("keywords:")
                .or_else(|| trimmed.strip_prefix("Keywords:"))
                .unwrap_or("");
            keywords.extend(
                kw_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            );
        } else if let Some(page_ref) = parse_page_reference_line(trimmed) {
            page_refs.push(page_ref);
        } else {
            summary_lines.push(line);
        }
    }

    (
        summary_lines.join("\n").trim().to_string(),
        keywords,
        page_refs,
    )
}

fn parse_page_reference_line(line: &str) -> Option<PageReference> {
    if !line.contains("Page") && !line.contains("page") {
        return None;
    }

    let page_pattern = regex::Regex::new(r"[Pp]ages?\s*(\d+(?:\s*[-,]\s*\d+)*)").ok()?;
    let captures = page_pattern.captures(line)?;
    let page_str = captures.get(1)?.as_str();

    let pages: Vec<u32> = page_str
        .split(|c| c == ',' || c == '-')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if pages.is_empty() {
        return None;
    }

    let topic = line
        .split(|c| c == '(' || c == '[')
        .next()
        .unwrap_or(line)
        .trim()
        .trim_start_matches('-')
        .trim()
        .to_string();

    Some(PageReference { topic, pages })
}

#[crate::register_node]
#[derive(Default)]
pub struct ExtractContentSectionsNode {}

impl ExtractContentSectionsNode {
    pub fn new() -> Self {
        ExtractContentSectionsNode {}
    }
}

#[async_trait]
impl NodeLogic for ExtractContentSectionsNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "ai_processing_extract_content_sections",
            "Extract Content Sections",
            "Intelligently segments document into thematic sections with summaries, tracking content across non-contiguous pages. Ideal for large document corpora.",
            "AI/Processing",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(5)
                .set_security(5)
                .set_performance(5)
                .set_governance(5)
                .set_reliability(7)
                .set_cost(3)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Execution trigger to start section extraction.",
            VariableType::Execution,
        );

        node.add_input_pin(
            "pages",
            "Pages",
            "Document pages to segment into sections.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<DocumentPage>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "model",
            "Model",
            "AI model for semantic analysis.",
            VariableType::Struct,
        )
        .set_schema::<Bit>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "max_context_tokens",
            "Max Context Tokens",
            "Maximum tokens per analysis chunk.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(8000)));

        node.add_input_pin(
            "parallel_requests",
            "Parallel Requests",
            "Number of chunks to process in parallel. Set to 0 or chunks count to process all at once.",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(4)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution output after extraction completes.",
            VariableType::Execution,
        );

        node.add_output_pin(
            "sections",
            "Sections",
            "Array of thematic content sections with cross-page tracking.",
            VariableType::Struct,
        )
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_schema::<ContentSection>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let pages: Vec<DocumentPage> = context.evaluate_pin("pages").await?;
        let model_bit: Bit = context.evaluate_pin("model").await?;
        let max_context_tokens: i64 = context.evaluate_pin("max_context_tokens").await?;
        let parallel_requests: i64 = context.evaluate_pin("parallel_requests").await?;

        if pages.is_empty() {
            context
                .set_pin_value("sections", json!(Vec::<ContentSection>::new()))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        let system_prompt = r#"You are a document analysis expert specializing in semantic segmentation.
Your task is to identify distinct thematic sections within document content, even when topics span non-contiguous pages.

For each distinct topic or theme you identify, provide:
1. SECTION_TITLE: A concise, descriptive title
2. SECTION_SUMMARY: A brief summary of that topic's content
3. SECTION_KEYWORDS: Comma-separated keywords specific to this section
4. SECTION_PAGES: Page numbers where this topic appears

Format each section exactly as:
===SECTION===
TITLE: [title]
SUMMARY: [summary]
KEYWORDS: [keyword1, keyword2, ...]
PAGES: [page numbers]
===END_SECTION===

Group related content even if it appears on different pages. Focus on semantic coherence, not page order.
Extract specific, domain-relevant keywords that would help identify this content in a large corpus."#;

        let chunks = chunk_pages_for_context(&pages, max_context_tokens as usize);
        let num_chunks = chunks.len();

        let concurrency = if parallel_requests <= 0 {
            num_chunks
        } else {
            (parallel_requests as usize).min(num_chunks)
        };

        let all_raw_sections: Vec<ContentSection> = if concurrency <= 1 {
            // Sequential processing - no cross-chunk merge needed for single chunk
            let mut results = Vec::new();
            for chunk in &chunks {
                let content = format_pages_for_prompt(chunk);
                let user_prompt = format!(
                    "Analyze the following document content and extract thematic sections:\n\n{}",
                    content
                );

                let response =
                    invoke_model_simple(context, &model_bit, system_prompt, &user_prompt).await?;
                results.extend(parse_sections_response(&response));
            }
            results
        } else {
            // Parallel processing
            use futures::stream::{self, StreamExt};
            use std::sync::Arc;

            let model_bit = Arc::new(model_bit.clone());
            let system_prompt = Arc::new(system_prompt.to_string());
            let app_state = context.app_state.clone();
            let token = context.token.clone();

            let tasks: Vec<_> = chunks
                .iter()
                .map(|chunk| {
                    let content = format_pages_for_prompt(chunk);
                    let user_prompt = format!(
                        "Analyze the following document content and extract thematic sections:\n\n{}",
                        content
                    );

                    let model_bit = Arc::clone(&model_bit);
                    let system_prompt = Arc::clone(&system_prompt);
                    let app_state = app_state.clone();
                    let token = token.clone();

                    async move {
                        let response = invoke_model_simple_standalone(
                            &app_state,
                            &token,
                            &model_bit,
                            &system_prompt,
                            &user_prompt,
                        )
                        .await?;
                        Ok::<_, flow_like_types::Error>(parse_sections_response(&response))
                    }
                })
                .collect();

            let chunk_results: Vec<Vec<ContentSection>> = stream::iter(tasks)
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;

            chunk_results.into_iter().flatten().collect()
        };

        // Only merge if we have multiple chunks AND parallel processing was used
        let sections = if concurrency > 1 && num_chunks > 1 && !all_raw_sections.is_empty() {
            use std::sync::Arc;
            merge_related_sections(context, &Arc::new(model_bit), all_raw_sections).await?
        } else {
            all_raw_sections
        };

        context.set_pin_value("sections", json!(sections)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Content section extraction requires the 'execute' feature"
        ))
    }
}

fn parse_sections_response(response: &str) -> Vec<ContentSection> {
    let mut sections = Vec::new();
    let section_pattern = regex::Regex::new(r"===SECTION===(.*?)===END_SECTION===").ok();

    if let Some(pattern) = section_pattern {
        for cap in pattern.captures_iter(response) {
            if let Some(content) = cap.get(1) {
                if let Some(section) = parse_single_section(content.as_str()) {
                    sections.push(section);
                }
            }
        }
    }

    if sections.is_empty() {
        if let Some(section) = parse_single_section(response) {
            sections.push(section);
        }
    }

    sections
}

fn parse_single_section(content: &str) -> Option<ContentSection> {
    let mut title = String::new();
    let mut summary = String::new();
    let mut keywords = Vec::new();
    let mut pages = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(t) = trimmed.strip_prefix("TITLE:") {
            title = t.trim().to_string();
        } else if let Some(s) = trimmed.strip_prefix("SUMMARY:") {
            summary = s.trim().to_string();
        } else if let Some(k) = trimmed.strip_prefix("KEYWORDS:") {
            keywords = k
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(p) = trimmed.strip_prefix("PAGES:") {
            pages = p
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|s| s.trim().parse().ok())
                .collect();
        }
    }

    if title.is_empty() && summary.is_empty() {
        return None;
    }

    Some(ContentSection {
        title: if title.is_empty() {
            "Untitled Section".to_string()
        } else {
            title
        },
        summary,
        keywords,
        page_references: pages,
    })
}

#[cfg(feature = "execute")]
async fn merge_related_sections(
    context: &ExecutionContext,
    model_bit: &Bit,
    sections: Vec<ContentSection>,
) -> flow_like_types::Result<Vec<ContentSection>> {
    if sections.len() <= 1 {
        return Ok(sections);
    }

    let sections_json = sections
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{{\"id\": {}, \"title\": \"{}\", \"keywords\": {:?}, \"pages\": {:?}}}",
                i, s.title, s.keywords, s.page_references
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    let system_prompt = r#"You are a document analysis expert. Given a list of content sections, identify which sections should be merged because they cover the same topic appearing on different pages.

Return a JSON array of merge groups. Each group contains the IDs of sections that should be merged together.
Only group sections that are clearly about the same specific topic.

Example output:
[[0, 3, 7], [1, 5], [2], [4], [6]]

This means: sections 0, 3, 7 should be merged; sections 1, 5 should be merged; others remain separate."#;

    let user_prompt = format!(
        "Analyze these sections and identify merge groups:\n{}",
        sections_json
    );

    let response = invoke_model_simple(context, model_bit, system_prompt, &user_prompt).await?;

    let merge_groups = parse_merge_groups(&response, sections.len());

    let mut merged = Vec::new();
    let mut used = vec![false; sections.len()];

    for group in merge_groups {
        if group.is_empty() {
            continue;
        }

        let mut combined_title = String::new();
        let mut combined_summary = String::new();
        let mut combined_keywords = Vec::new();
        let mut combined_pages = Vec::new();

        for &idx in &group {
            if idx < sections.len() && !used[idx] {
                used[idx] = true;
                let s = &sections[idx];

                if combined_title.is_empty() {
                    combined_title = s.title.clone();
                }

                if !combined_summary.is_empty() {
                    combined_summary.push_str(" ");
                }
                combined_summary.push_str(&s.summary);

                for kw in &s.keywords {
                    if !combined_keywords.contains(kw) {
                        combined_keywords.push(kw.clone());
                    }
                }

                combined_pages.extend(&s.page_references);
            }
        }

        if !combined_title.is_empty() {
            combined_pages.sort();
            combined_pages.dedup();

            merged.push(ContentSection {
                title: combined_title,
                summary: combined_summary,
                keywords: combined_keywords,
                page_references: combined_pages,
            });
        }
    }

    for (i, section) in sections.into_iter().enumerate() {
        if !used[i] {
            merged.push(section);
        }
    }

    Ok(merged)
}

#[cfg(feature = "execute")]
fn parse_merge_groups(response: &str, max_idx: usize) -> Vec<Vec<usize>> {
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(groups) = flow_like_types::json::from_str::<Vec<Vec<usize>>>(cleaned) {
        return groups
            .into_iter()
            .map(|g| g.into_iter().filter(|&i| i < max_idx).collect())
            .collect();
    }

    let bracket_pattern = regex::Regex::new(r"\[([^\[\]]+)\]").ok();
    if let Some(pattern) = bracket_pattern {
        let mut groups = Vec::new();
        for cap in pattern.captures_iter(cleaned) {
            if let Some(inner) = cap.get(1) {
                let indices: Vec<usize> = inner
                    .as_str()
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .filter(|&i| i < max_idx)
                    .collect();
                if !indices.is_empty() {
                    groups.push(indices);
                }
            }
        }
        if !groups.is_empty() {
            return groups;
        }
    }

    (0..max_idx).map(|i| vec![i]).collect()
}
