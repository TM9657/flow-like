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
use flow_like_model_provider::llm::ModelLogic;
use flow_like_storage::Path;
use flow_like_types::image::ImageReader;
use flow_like_types::{Bytes, async_trait, json::json};
use markitdown::{ConversionOptions, Document, ExtractedImage, MarkItDown, create_llm_client};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

        let completion_handle = model.completion_model_handle(None).await?;
        let llm_client = create_llm_client(completion_handle);

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
