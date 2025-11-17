use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};
use htmd::HtmlToMarkdownBuilder;

#[crate::register_node]
#[derive(Default)]
pub struct HTMLToMarkdownNode {}

impl HTMLToMarkdownNode {
    pub fn new() -> Self {
        HTMLToMarkdownNode {}
    }
}

#[async_trait]
impl NodeLogic for HTMLToMarkdownNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "utils_md_html_to_md",
            "HTML to Markdown",
            "Attempts to convert HTML to Markdown, removing unwanted tags",
            "Utils/Markdown",
        );

        node.add_icon("/flow/icons/web.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("html", "Html", "Html to Parse", VariableType::String);

        node.add_input_pin("skipped_tags", "Tags", "Tags to skip", VariableType::String)
            .set_value_type(flow_like::flow::pin::ValueType::Array)
            .set_default_value(Some(json!(["script", "style", "iframe",])));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Finished Parsing",
            VariableType::Execution,
        );

        node.add_output_pin(
            "markdown",
            "Markdown",
            "The parsed Markdown",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let html: String = context.evaluate_pin("html").await?;
        let skipped_tags: Vec<String> = context.evaluate_pin("skipped_tags").await?;
        let skipped_tags = skipped_tags
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();

        let converter = HtmlToMarkdownBuilder::new().skip_tags(skipped_tags).build();
        let markdown = converter.convert(&html)?;

        context.set_pin_value("markdown", json!(markdown)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
