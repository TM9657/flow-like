use crate::data::datafusion::session::DataFusionSession;
use crate::data::path::FlowPath;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    variable::VariableType,
};
use flow_like_storage::datafusion::{
    common::TableReference,
    datasource::{
        file_format::{csv::CsvFormat, json::JsonFormat, parquet::ParquetFormat},
        listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl},
    },
};
use flow_like_types::{async_trait, json::json, reqwest::Url};
use std::sync::Arc;

fn build_store_url(store_ref: &str, path: &str) -> String {
    format!("flowlike://{}/{}", store_ref, path.trim_start_matches('/'))
}

#[crate::register_node]
#[derive(Default)]
pub struct MountStoreParquetNode {}

impl MountStoreParquetNode {
    pub fn new() -> Self {
        MountStoreParquetNode {}
    }
}

#[async_trait]
impl NodeLogic for MountStoreParquetNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_mount_parquet",
            "Mount Parquet",
            "Mount Parquet files from a FlowPath prefix into a DataFusion session as a queryable table",
            "Data/DataFusion",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "DataFusion session to mount the table into",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "path",
            "Path",
            "FlowPath to Parquet files (can be a directory prefix or single file)",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register the table as in the DataFusion catalog",
            VariableType::String,
        );

        node.add_input_pin(
            "file_extension",
            "File Extension",
            "File extension filter (e.g., 'parquet', 'parquet.gz')",
            VariableType::String,
        )
        .set_default_value(Some(json!("parquet")));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table mounted successfully",
            VariableType::Execution,
        );

        node.scores = Some(NodeScores {
            privacy: 10,
            security: 10,
            performance: 9,
            governance: 9,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let file_extension: String = context.evaluate_pin("file_extension").await?;

        let cached_session = session.load(context).await?;
        let store = path.to_store(context).await?;
        let object_store = store.as_generic();

        let url_str = build_store_url(&path.store_ref, &path.path);
        let url = Url::parse(&url_str)?;
        let table_path = ListingTableUrl::parse(&url_str)?;

        cached_session.ctx.register_object_store(&url, object_store);

        let format = ParquetFormat::default();
        let listing_options =
            ListingOptions::new(Arc::new(format)).with_file_extension(&file_extension);

        let config = ListingTableConfig::new(table_path)
            .with_listing_options(listing_options)
            .infer_schema(&cached_session.ctx.state())
            .await?;

        let table = ListingTable::try_new(config)?;

        cached_session
            .ctx
            .register_table(TableReference::bare(table_name), Arc::new(table))?;

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct MountStoreCsvNode {}

impl MountStoreCsvNode {
    pub fn new() -> Self {
        MountStoreCsvNode {}
    }
}

#[async_trait]
impl NodeLogic for MountStoreCsvNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_mount_csv",
            "Mount CSV",
            "Mount CSV files from a FlowPath into a DataFusion session as a queryable table",
            "Data/DataFusion",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "DataFusion session to mount the table into",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "path",
            "Path",
            "FlowPath to CSV files (can be a directory prefix or single file)",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register the table as in the DataFusion catalog",
            VariableType::String,
        );

        node.add_input_pin(
            "has_header",
            "Has Header",
            "Whether the CSV files have a header row",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Column delimiter character",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_input_pin(
            "file_extension",
            "File Extension",
            "File extension filter",
            VariableType::String,
        )
        .set_default_value(Some(json!("csv")));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table mounted successfully",
            VariableType::Execution,
        );

        node.scores = Some(NodeScores {
            privacy: 10,
            security: 10,
            performance: 8,
            governance: 9,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let has_header: bool = context.evaluate_pin("has_header").await?;
        let delimiter: String = context.evaluate_pin("delimiter").await?;
        let file_extension: String = context.evaluate_pin("file_extension").await?;

        let cached_session = session.load(context).await?;
        let store = path.to_store(context).await?;
        let object_store = store.as_generic();

        let url_str = build_store_url(&path.store_ref, &path.path);
        let url = Url::parse(&url_str)?;
        let table_path = ListingTableUrl::parse(&url_str)?;

        cached_session.ctx.register_object_store(&url, object_store);

        let delimiter_byte = delimiter.as_bytes().first().copied().unwrap_or(b',');
        let format = CsvFormat::default()
            .with_has_header(has_header)
            .with_delimiter(delimiter_byte);

        let listing_options =
            ListingOptions::new(Arc::new(format)).with_file_extension(&file_extension);

        let config = ListingTableConfig::new(table_path)
            .with_listing_options(listing_options)
            .infer_schema(&cached_session.ctx.state())
            .await?;

        let table = ListingTable::try_new(config)?;

        cached_session
            .ctx
            .register_table(TableReference::bare(table_name), Arc::new(table))?;

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct MountStoreJsonNode {}

impl MountStoreJsonNode {
    pub fn new() -> Self {
        MountStoreJsonNode {}
    }
}

#[async_trait]
impl NodeLogic for MountStoreJsonNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_mount_json",
            "Mount JSON",
            "Mount JSON (newline-delimited) files from a FlowPath into a DataFusion session as a queryable table",
            "Data/DataFusion",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin(
            "session",
            "Session",
            "DataFusion session to mount the table into",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "path",
            "Path",
            "FlowPath to JSON files (can be a directory prefix or single file)",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register the table as in the DataFusion catalog",
            VariableType::String,
        );

        node.add_input_pin(
            "file_extension",
            "File Extension",
            "File extension filter",
            VariableType::String,
        )
        .set_default_value(Some(json!("json")));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table mounted successfully",
            VariableType::Execution,
        );

        node.scores = Some(NodeScores {
            privacy: 10,
            security: 10,
            performance: 8,
            governance: 9,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let file_extension: String = context.evaluate_pin("file_extension").await?;

        let cached_session = session.load(context).await?;
        let store = path.to_store(context).await?;
        let object_store = store.as_generic();

        let url_str = build_store_url(&path.store_ref, &path.path);
        let url = Url::parse(&url_str)?;
        let table_path = ListingTableUrl::parse(&url_str)?;

        cached_session.ctx.register_object_store(&url, object_store);

        let format = JsonFormat::default();
        let listing_options =
            ListingOptions::new(Arc::new(format)).with_file_extension(&file_extension);

        let config = ListingTableConfig::new(table_path)
            .with_listing_options(listing_options)
            .infer_schema(&cached_session.ctx.state())
            .await?;

        let table = ListingTable::try_new(config)?;

        cached_session
            .ctx
            .register_table(TableReference::bare(table_name), Arc::new(table))?;

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like::flow::pin::PinType;
    use flow_like::flow::variable::VariableType;

    #[test]
    fn test_build_store_url_simple() {
        let url = build_store_url("my_store", "path/to/file.parquet");
        assert_eq!(url, "flowlike://my_store/path/to/file.parquet");
    }

    #[test]
    fn test_build_store_url_with_leading_slash() {
        let url = build_store_url("my_store", "/path/to/file.parquet");
        assert_eq!(url, "flowlike://my_store/path/to/file.parquet");
    }

    #[test]
    fn test_build_store_url_empty_path() {
        let url = build_store_url("my_store", "");
        assert_eq!(url, "flowlike://my_store/");
    }

    #[test]
    fn test_build_store_url_multiple_leading_slashes() {
        let url = build_store_url("my_store", "///path/to/file");
        assert_eq!(url, "flowlike://my_store/path/to/file");
    }

    #[test]
    fn test_mount_parquet_node_structure() {
        let node_logic = MountStoreParquetNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_mount_parquet");
        assert_eq!(node.friendly_name, "Mount Parquet");
        assert_eq!(node.category, "Data/DataFusion");
    }

    #[test]
    fn test_mount_parquet_node_input_pins() {
        let node_logic = MountStoreParquetNode::new();
        let node = node_logic.get_node();

        let input_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Input).collect();

        assert!(input_pins.iter().any(|p| p.name == "exec_in" && p.data_type == VariableType::Execution));
        assert!(input_pins.iter().any(|p| p.name == "session" && p.data_type == VariableType::Struct));
        assert!(input_pins.iter().any(|p| p.name == "path" && p.data_type == VariableType::Struct));
        assert!(input_pins.iter().any(|p| p.name == "table_name" && p.data_type == VariableType::String));
        assert!(input_pins.iter().any(|p| p.name == "file_extension" && p.data_type == VariableType::String));
    }

    #[test]
    fn test_mount_parquet_node_default_extension() {
        let node_logic = MountStoreParquetNode::new();
        let node = node_logic.get_node();

        let ext_pin = node.pins.values().find(|p| p.name == "file_extension");
        assert!(ext_pin.is_some());
        let default_value = ext_pin.unwrap().default_value.as_ref();
        assert!(default_value.is_some());
    }

    #[test]
    fn test_mount_csv_node_structure() {
        let node_logic = MountStoreCsvNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_mount_csv");
        assert_eq!(node.friendly_name, "Mount CSV");
    }

    #[test]
    fn test_mount_csv_node_csv_specific_pins() {
        let node_logic = MountStoreCsvNode::new();
        let node = node_logic.get_node();

        let input_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Input).collect();

        let has_header_pin = input_pins.iter().find(|p| p.name == "has_header");
        assert!(has_header_pin.is_some());
        assert_eq!(has_header_pin.unwrap().data_type, VariableType::Boolean);

        let delimiter_pin = input_pins.iter().find(|p| p.name == "delimiter");
        assert!(delimiter_pin.is_some());
        assert_eq!(delimiter_pin.unwrap().data_type, VariableType::String);
    }

    #[test]
    fn test_mount_json_node_structure() {
        let node_logic = MountStoreJsonNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_mount_json");
        assert_eq!(node.friendly_name, "Mount JSON");
    }

    #[test]
    fn test_mount_json_node_pins() {
        let node_logic = MountStoreJsonNode::new();
        let node = node_logic.get_node();

        let input_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Input).collect();
        let output_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Output).collect();

        assert!(input_pins.iter().any(|p| p.name == "exec_in"));
        assert!(input_pins.iter().any(|p| p.name == "session"));
        assert!(input_pins.iter().any(|p| p.name == "path"));
        assert!(input_pins.iter().any(|p| p.name == "table_name"));
        assert!(input_pins.iter().any(|p| p.name == "file_extension"));
        assert!(output_pins.iter().any(|p| p.name == "exec_out"));
    }

    #[test]
    fn test_all_mount_nodes_have_scores() {
        let parquet_node = MountStoreParquetNode::new().get_node();
        let csv_node = MountStoreCsvNode::new().get_node();
        let json_node = MountStoreJsonNode::new().get_node();

        assert!(parquet_node.scores.is_some());
        assert!(csv_node.scores.is_some());
        assert!(json_node.scores.is_some());
    }
}
