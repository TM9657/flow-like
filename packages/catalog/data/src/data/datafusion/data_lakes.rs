use crate::data::datafusion::session::DataFusionSession;
use crate::data::path::FlowPath;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, reqwest::Url};

fn build_store_url(store_ref: &str, path: &str) -> String {
    format!("flowlike://{}/{}", store_ref, path.trim_start_matches('/'))
}

// ============================================================================
// Delta Lake Support
// ============================================================================

/// Register a Delta Lake table in DataFusion using FlowPath
#[crate::register_node]
#[derive(Default)]
pub struct RegisterDeltaTableNode {}

impl RegisterDeltaTableNode {
    pub fn new() -> Self {
        RegisterDeltaTableNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterDeltaTableNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_delta",
            "Register Delta Table",
            "Register a Delta Lake table in DataFusion using a FlowPath. Requires the 'delta' feature.",
            "Data/DataFusion/Lakes",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin("session", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_input_pin("path", "Path", "FlowPath to the Delta table directory", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin("table_name", "Table Name", "Name to register in DataFusion", VariableType::String);

        node.add_input_pin("version", "Version", "Specific version to load (-1 for latest)", VariableType::Integer)
            .set_default_value(Some(json!(-1)));

        node.add_output_pin("exec_out", "Done", "Table registered", VariableType::Execution);

        node.add_output_pin("session_out", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_output_pin("table_version", "Table Version", "Actual version loaded", VariableType::Integer);

        node.add_output_pin("num_files", "Num Files", "Number of data files in the table", VariableType::Integer);

        node.scores = Some(NodeScores {
            privacy: 8,
            security: 8,
            performance: 9,
            governance: 9,
            reliability: 9,
            cost: 9,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        #[cfg(feature = "delta")]
        {
            use flow_like_storage::deltalake::DeltaTableBuilder;

            context.deactivate_exec_pin("exec_out").await?;

            let session: DataFusionSession = context.evaluate_pin("session").await?;
            let path: FlowPath = context.evaluate_pin("path").await?;
            let table_name: String = context.evaluate_pin("table_name").await?;
            let version: i64 = context.evaluate_pin("version").await.unwrap_or(-1);

            let cached_session = session.load(context).await?;
            let store = path.to_store(context).await?;
            let object_store = store.as_generic();

            let url_str = build_store_url(&path.store_ref, &path.path);
            let url = Url::parse(&url_str)?;

            let mut builder = DeltaTableBuilder::from_uri(&url_str)
                .with_storage_backend(object_store, url);

            if version >= 0 {
                builder = builder.with_version(version);
            }

            let delta_table = builder.load().await
                .map_err(|e| flow_like_types::anyhow!("Failed to open Delta table: {}", e))?;

            let actual_version = delta_table.version();
            let num_files = delta_table.get_files_count() as i64;

            cached_session.ctx.register_table(&table_name, std::sync::Arc::new(delta_table))?;

            context.set_pin_value("session_out", json!(session)).await?;
            context.set_pin_value("table_version", json!(actual_version)).await?;
            context.set_pin_value("num_files", json!(num_files)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }

        #[cfg(not(feature = "delta"))]
        {
            Err(flow_like_types::anyhow!(
                "Delta Lake support not enabled. Rebuild with the 'delta' feature flag."
            ))
        }
    }
}

/// Time travel to a specific Delta table version using FlowPath
#[crate::register_node]
#[derive(Default)]
pub struct DeltaTimeTravelNode {}

impl DeltaTimeTravelNode {
    pub fn new() -> Self {
        DeltaTimeTravelNode {}
    }
}

#[async_trait]
impl NodeLogic for DeltaTimeTravelNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_delta_time_travel",
            "Delta Time Travel",
            "Load a specific version or timestamp of a Delta table for point-in-time queries.",
            "Data/DataFusion/Lakes",
        );
        node.add_icon("/flow/icons/clock.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin("session", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_input_pin("path", "Path", "FlowPath to the Delta table directory", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin("table_name", "Table Name", "Name to register in DataFusion", VariableType::String);

        node.add_input_pin("travel_mode", "Travel Mode", "Mode: 'version' or 'timestamp'", VariableType::String)
            .set_default_value(Some(json!("version")));

        node.add_input_pin("version", "Version", "Version number (when mode is 'version')", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_input_pin("timestamp", "Timestamp", "ISO 8601 timestamp (when mode is 'timestamp')", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Done", "Table registered", VariableType::Execution);

        node.add_output_pin("session_out", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_output_pin("loaded_version", "Loaded Version", "Actual version loaded", VariableType::Integer);

        node.scores = Some(NodeScores {
            privacy: 8,
            security: 8,
            performance: 8,
            governance: 10,
            reliability: 9,
            cost: 8,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        #[cfg(feature = "delta")]
        {
            use flow_like_storage::deltalake::DeltaTableBuilder;

            context.deactivate_exec_pin("exec_out").await?;

            let session: DataFusionSession = context.evaluate_pin("session").await?;
            let path: FlowPath = context.evaluate_pin("path").await?;
            let table_name: String = context.evaluate_pin("table_name").await?;
            let travel_mode: String = context.evaluate_pin("travel_mode").await?;
            let version: i64 = context.evaluate_pin("version").await.unwrap_or(0);
            let timestamp: String = context.evaluate_pin("timestamp").await.unwrap_or_default();

            let cached_session = session.load(context).await?;
            let store = path.to_store(context).await?;
            let object_store = store.as_generic();

            let url_str = build_store_url(&path.store_ref, &path.path);
            let url = Url::parse(&url_str)?;

            let builder = match travel_mode.to_lowercase().as_str() {
                "version" => {
                    DeltaTableBuilder::from_uri(&url_str)
                        .with_storage_backend(object_store, url)
                        .with_version(version)
                }
                "timestamp" => {
                    let dt = chrono::DateTime::parse_from_rfc3339(&timestamp)
                        .map_err(|e| flow_like_types::anyhow!("Invalid timestamp format: {}", e))?;
                    DeltaTableBuilder::from_uri(&url_str)
                        .with_storage_backend(object_store, url)
                        .with_timestamp(dt.to_utc())
                }
                _ => {
                    return Err(flow_like_types::anyhow!("Invalid travel mode: {}. Use 'version' or 'timestamp'", travel_mode));
                }
            };

            let delta_table = builder.load().await
                .map_err(|e| flow_like_types::anyhow!("Failed to load Delta table: {}", e))?;

            let loaded_version = delta_table.version();
            cached_session.ctx.register_table(&table_name, std::sync::Arc::new(delta_table))?;

            context.set_pin_value("session_out", json!(session)).await?;
            context.set_pin_value("loaded_version", json!(loaded_version)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }

        #[cfg(not(feature = "delta"))]
        {
            Err(flow_like_types::anyhow!(
                "Delta Lake support not enabled. Rebuild with the 'delta' feature flag."
            ))
        }
    }
}

/// Get Delta table history and metadata using FlowPath
#[crate::register_node]
#[derive(Default)]
pub struct DeltaTableInfoNode {}

impl DeltaTableInfoNode {
    pub fn new() -> Self {
        DeltaTableInfoNode {}
    }
}

#[async_trait]
impl NodeLogic for DeltaTableInfoNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_delta_info",
            "Delta Table Info",
            "Get metadata and history information about a Delta table.",
            "Data/DataFusion/Lakes",
        );
        node.add_icon("/flow/icons/info.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin("path", "Path", "FlowPath to the Delta table directory", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin("history_limit", "History Limit", "Max number of history entries to return", VariableType::Integer)
            .set_default_value(Some(json!(10)));

        node.add_output_pin("exec_out", "Done", "Info retrieved", VariableType::Execution);

        node.add_output_pin("current_version", "Current Version", "Latest version number", VariableType::Integer);

        node.add_output_pin("num_files", "Num Files", "Number of data files", VariableType::Integer);

        node.add_output_pin("schema", "Schema", "Table schema as JSON", VariableType::Generic);

        node.add_output_pin("history", "History", "Version history as JSON array", VariableType::Generic);

        node.add_output_pin("partitions", "Partitions", "Partition columns", VariableType::Generic);

        node.scores = Some(NodeScores {
            privacy: 10,
            security: 10,
            performance: 10,
            governance: 10,
            reliability: 10,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        #[cfg(feature = "delta")]
        {
            use flow_like_storage::deltalake::DeltaTableBuilder;

            context.deactivate_exec_pin("exec_out").await?;

            let path: FlowPath = context.evaluate_pin("path").await?;
            let history_limit: i64 = context.evaluate_pin("history_limit").await.unwrap_or(10);

            let store = path.to_store(context).await?;
            let object_store = store.as_generic();

            let url_str = build_store_url(&path.store_ref, &path.path);
            let url = Url::parse(&url_str)?;

            let delta_table = DeltaTableBuilder::from_uri(&url_str)
                .with_storage_backend(object_store, url)
                .load()
                .await
                .map_err(|e| flow_like_types::anyhow!("Failed to open Delta table: {}", e))?;

            let current_version = delta_table.version();
            let num_files = delta_table.get_files_count() as i64;

            let schema = delta_table.schema()
                .map(|s| {
                    let fields: Vec<_> = s.fields().map(|f| json!({
                        "name": f.name(),
                        "type": format!("{:?}", f.data_type()),
                        "nullable": f.is_nullable(),
                    })).collect();
                    json!({ "fields": fields })
                })
                .unwrap_or(json!(null));

            let partitions: Vec<String> = delta_table.metadata()
                .map(|m| m.partition_columns().clone())
                .unwrap_or_default();

            let history = delta_table.history(Some(history_limit as usize)).await
                .map(|h| h.iter().map(|entry| json!({
                    "read_version": entry.read_version,
                    "timestamp": entry.timestamp,
                    "operation": entry.operation,
                    "user_id": entry.user_id,
                    "user_name": entry.user_name,
                })).collect::<Vec<_>>())
                .unwrap_or_default();

            context.set_pin_value("current_version", json!(current_version)).await?;
            context.set_pin_value("num_files", json!(num_files)).await?;
            context.set_pin_value("schema", schema).await?;
            context.set_pin_value("history", json!(history)).await?;
            context.set_pin_value("partitions", json!(partitions)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }

        #[cfg(not(feature = "delta"))]
        {
            Err(flow_like_types::anyhow!(
                "Delta Lake support not enabled. Rebuild with the 'delta' feature flag."
            ))
        }
    }
}

// ============================================================================
// Register Parquet with Hive Partitioning (works without extra deps)
// ============================================================================

/// Register partitioned Parquet files with Hive-style partitioning using FlowPath
#[crate::register_node]
#[derive(Default)]
pub struct RegisterHivePartitionedParquetNode {}

impl RegisterHivePartitionedParquetNode {
    pub fn new() -> Self {
        RegisterHivePartitionedParquetNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterHivePartitionedParquetNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_hive_parquet",
            "Register Hive Parquet",
            "Register Hive-partitioned Parquet files as a table in DataFusion using a FlowPath.",
            "Data/DataFusion/Lakes",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin("session", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_input_pin("path", "Path", "FlowPath to root directory of partitioned Parquet files", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin("table_name", "Table Name", "Name to register in DataFusion", VariableType::String);

        node.add_output_pin("exec_out", "Done", "Table registered", VariableType::Execution);

        node.add_output_pin("session_out", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.scores = Some(NodeScores {
            privacy: 9,
            security: 9,
            performance: 9,
            governance: 8,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use flow_like_storage::datafusion::datasource::listing::{
            ListingTable, ListingTableConfig, ListingTableUrl, ListingOptions,
        };
        use flow_like_storage::datafusion::datasource::file_format::parquet::ParquetFormat;
        use std::sync::Arc;

        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;

        let cached_session = session.load(context).await?;
        let store = path.to_store(context).await?;
        let object_store = store.as_generic();

        let url_str = build_store_url(&path.store_ref, &path.path);
        let url = Url::parse(&url_str)?;
        let table_path = ListingTableUrl::parse(&url_str)?;

        cached_session.ctx.register_object_store(&url, object_store);

        let parquet_format = ParquetFormat::default();
        let listing_options = ListingOptions::new(Arc::new(parquet_format))
            .with_file_extension(".parquet")
            .with_table_partition_cols(vec![])
            .with_collect_stat(true);

        let config = ListingTableConfig::new(table_path)
            .with_listing_options(listing_options)
            .infer_schema(&cached_session.ctx.state())
            .await?;

        let listing_table = ListingTable::try_new(config)?;
        cached_session.ctx.register_table(&table_name, Arc::new(listing_table))?;

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

/// Register partitioned JSON files as a table using FlowPath
#[crate::register_node]
#[derive(Default)]
pub struct RegisterPartitionedJsonNode {}

impl RegisterPartitionedJsonNode {
    pub fn new() -> Self {
        RegisterPartitionedJsonNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterPartitionedJsonNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_partitioned_json",
            "Register Partitioned JSON",
            "Register partitioned JSON/NDJSON files as a table in DataFusion using a FlowPath.",
            "Data/DataFusion/Lakes",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin("session", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_input_pin("path", "Path", "FlowPath to JSON files", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin("table_name", "Table Name", "Name to register", VariableType::String);

        node.add_input_pin("file_extension", "File Extension", "File extension to match", VariableType::String)
            .set_default_value(Some(json!(".json")));

        node.add_output_pin("exec_out", "Done", "Table registered", VariableType::Execution);

        node.add_output_pin("session_out", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.scores = Some(NodeScores {
            privacy: 9,
            security: 9,
            performance: 7,
            governance: 8,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use flow_like_storage::datafusion::datasource::listing::{
            ListingTable, ListingTableConfig, ListingTableUrl, ListingOptions,
        };
        use flow_like_storage::datafusion::datasource::file_format::json::JsonFormat;
        use std::sync::Arc;

        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let file_extension: String = context.evaluate_pin("file_extension").await.unwrap_or_else(|_| ".json".to_string());

        let cached_session = session.load(context).await?;
        let store = path.to_store(context).await?;
        let object_store = store.as_generic();

        let url_str = build_store_url(&path.store_ref, &path.path);
        let url = Url::parse(&url_str)?;
        let table_path = ListingTableUrl::parse(&url_str)?;

        cached_session.ctx.register_object_store(&url, object_store);

        let json_format = JsonFormat::default();
        let listing_options = ListingOptions::new(Arc::new(json_format))
            .with_file_extension(&file_extension)
            .with_collect_stat(true);

        let config = ListingTableConfig::new(table_path)
            .with_listing_options(listing_options)
            .infer_schema(&cached_session.ctx.state())
            .await?;

        let listing_table = ListingTable::try_new(config)?;
        cached_session.ctx.register_table(&table_name, Arc::new(listing_table))?;

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

// ============================================================================
// Write Delta Lake Table
// ============================================================================

/// Write query results to a Delta Lake table using FlowPath
#[crate::register_node]
#[derive(Default)]
pub struct WriteDeltaTableNode {}

impl WriteDeltaTableNode {
    pub fn new() -> Self {
        WriteDeltaTableNode {}
    }
}

#[async_trait]
impl NodeLogic for WriteDeltaTableNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_write_delta",
            "Write Delta Table",
            "Write query results to a new or existing Delta Lake table using FlowPath. Supports append, overwrite modes.",
            "Data/DataFusion/Lakes",
        );
        node.add_icon("/flow/icons/save.svg");

        node.add_input_pin("exec_in", "Input", "Trigger execution", VariableType::Execution);

        node.add_input_pin("session", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_input_pin("query", "Query", "SQL query to execute", VariableType::String);

        node.add_input_pin("path", "Path", "FlowPath for the Delta table directory", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin("mode", "Mode", "Write mode: append, overwrite, error, ignore", VariableType::String)
            .set_default_value(Some(json!("append")));

        node.add_input_pin("partition_by", "Partition By", "Columns to partition by (comma-separated)", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Done", "Write completed", VariableType::Execution);

        node.add_output_pin("session_out", "Session", "DataFusion session", VariableType::Struct)
            .set_schema::<DataFusionSession>();

        node.add_output_pin("rows_written", "Rows Written", "Number of rows written", VariableType::Integer);

        node.add_output_pin("new_version", "New Version", "Version number after write", VariableType::Integer);

        node.scores = Some(NodeScores {
            privacy: 7,
            security: 7,
            performance: 8,
            governance: 9,
            reliability: 9,
            cost: 8,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        #[cfg(feature = "delta")]
        {
            use flow_like_storage::deltalake::{DeltaOps, DeltaTable, DeltaTableBuilder};
            use flow_like_storage::deltalake::protocol::SaveMode;

            context.deactivate_exec_pin("exec_out").await?;

            let session: DataFusionSession = context.evaluate_pin("session").await?;
            let query: String = context.evaluate_pin("query").await?;
            let path: FlowPath = context.evaluate_pin("path").await?;
            let mode: String = context.evaluate_pin("mode").await.unwrap_or_else(|_| "append".to_string());
            let partition_by_str: String = context.evaluate_pin("partition_by").await.unwrap_or_default();

            let cached_session = session.load(context).await?;

            let df = cached_session.ctx.sql(&query).await?;
            let batches = df.collect().await?;

            if batches.is_empty() {
                context.set_pin_value("session_out", json!(session)).await?;
                context.set_pin_value("rows_written", json!(0)).await?;
                context.set_pin_value("new_version", json!(0)).await?;
                context.activate_exec_pin("exec_out").await?;
                return Ok(());
            }

            let total_rows: i64 = batches.iter().map(|b| b.num_rows() as i64).sum();

            let store = path.to_store(context).await?;
            let object_store = store.as_generic();

            let url_str = build_store_url(&path.store_ref, &path.path);
            let url = Url::parse(&url_str)?;

            let partition_cols: Vec<String> = partition_by_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let write_mode = match mode.to_lowercase().as_str() {
                "append" => SaveMode::Append,
                "overwrite" => SaveMode::Overwrite,
                "error" | "errorifexists" => SaveMode::ErrorIfExists,
                "ignore" => SaveMode::Ignore,
                _ => return Err(flow_like_types::anyhow!("Invalid write mode: {}", mode)),
            };

            let ops = match DeltaTableBuilder::from_uri(&url_str)
                .with_storage_backend(object_store.clone(), url.clone())
                .load()
                .await
            {
                Ok(table) => DeltaOps::from(table),
                Err(_) => {
                    let table = DeltaTableBuilder::from_uri(&url_str)
                        .with_storage_backend(object_store.clone(), url.clone())
                        .build()
                        .map_err(|e| flow_like_types::anyhow!("Failed to create Delta table: {}", e))?;
                    DeltaOps::from(table)
                }
            };

            let mut write_builder = ops.write(batches).with_save_mode(write_mode);

            if !partition_cols.is_empty() {
                write_builder = write_builder.with_partition_columns(partition_cols);
            }

            let table: DeltaTable = write_builder.await
                .map_err(|e| flow_like_types::anyhow!("Failed to write to Delta table: {}", e))?;

            let new_version = table.version();

            context.set_pin_value("session_out", json!(session)).await?;
            context.set_pin_value("rows_written", json!(total_rows)).await?;
            context.set_pin_value("new_version", json!(new_version)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }

        #[cfg(not(feature = "delta"))]
        {
            Err(flow_like_types::anyhow!(
                "Delta Lake support not enabled. Rebuild with the 'delta' feature flag."
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like::flow::pin::PinType;
    use flow_like::flow::variable::VariableType;

    #[test]
    fn test_build_store_url_simple() {
        let url = build_store_url("delta_store", "tables/sales");
        assert_eq!(url, "flowlike://delta_store/tables/sales");
    }

    #[test]
    fn test_build_store_url_with_leading_slash() {
        let url = build_store_url("delta_store", "/tables/sales");
        assert_eq!(url, "flowlike://delta_store/tables/sales");
    }

    #[test]
    fn test_build_store_url_deep_path() {
        let url = build_store_url("s3_bucket", "data/warehouse/tables/customers/_delta_log");
        assert_eq!(url, "flowlike://s3_bucket/data/warehouse/tables/customers/_delta_log");
    }

    #[test]
    fn test_register_delta_table_node_structure() {
        let node_logic = RegisterDeltaTableNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_register_delta");
        assert_eq!(node.friendly_name, "Register Delta Table");
        assert_eq!(node.category, "Data/DataFusion/Lakes");
    }

    #[test]
    fn test_register_delta_table_node_input_pins() {
        let node_logic = RegisterDeltaTableNode::new();
        let node = node_logic.get_node();

        let input_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Input).collect();

        assert!(input_pins.iter().any(|p| p.name == "exec_in" && p.data_type == VariableType::Execution));
        assert!(input_pins.iter().any(|p| p.name == "session" && p.data_type == VariableType::Struct));
        assert!(input_pins.iter().any(|p| p.name == "path" && p.data_type == VariableType::Struct));
        assert!(input_pins.iter().any(|p| p.name == "table_name" && p.data_type == VariableType::String));
        assert!(input_pins.iter().any(|p| p.name == "version" && p.data_type == VariableType::Integer));
    }

    #[test]
    fn test_register_delta_table_node_output_pins() {
        let node_logic = RegisterDeltaTableNode::new();
        let node = node_logic.get_node();

        let output_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Output).collect();

        assert!(output_pins.iter().any(|p| p.name == "exec_out"));
        assert!(output_pins.iter().any(|p| p.name == "session_out"));
        assert!(output_pins.iter().any(|p| p.name == "table_version"));
        assert!(output_pins.iter().any(|p| p.name == "num_files"));
    }

    #[test]
    fn test_delta_time_travel_node_structure() {
        let node_logic = DeltaTimeTravelNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_delta_time_travel");
        assert_eq!(node.friendly_name, "Delta Time Travel");
    }

    #[test]
    fn test_delta_time_travel_node_travel_pins() {
        let node_logic = DeltaTimeTravelNode::new();
        let node = node_logic.get_node();

        let input_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Input).collect();

        let travel_mode_pin = input_pins.iter().find(|p| p.name == "travel_mode");
        assert!(travel_mode_pin.is_some());
        assert_eq!(travel_mode_pin.unwrap().data_type, VariableType::String);

        let version_pin = input_pins.iter().find(|p| p.name == "version");
        assert!(version_pin.is_some());
        assert_eq!(version_pin.unwrap().data_type, VariableType::Integer);

        let timestamp_pin = input_pins.iter().find(|p| p.name == "timestamp");
        assert!(timestamp_pin.is_some());
        assert_eq!(timestamp_pin.unwrap().data_type, VariableType::String);
    }

    #[test]
    fn test_delta_table_info_node_structure() {
        let node_logic = DeltaTableInfoNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_delta_info");
        assert_eq!(node.friendly_name, "Delta Table Info");
    }

    #[test]
    fn test_delta_table_info_node_output_pins() {
        let node_logic = DeltaTableInfoNode::new();
        let node = node_logic.get_node();

        let output_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Output).collect();

        assert!(output_pins.iter().any(|p| p.name == "current_version"));
        assert!(output_pins.iter().any(|p| p.name == "num_files"));
        assert!(output_pins.iter().any(|p| p.name == "schema"));
        assert!(output_pins.iter().any(|p| p.name == "partitions"));
        assert!(output_pins.iter().any(|p| p.name == "history"));
    }

    #[test]
    fn test_register_hive_partitioned_parquet_node_structure() {
        let node_logic = RegisterHivePartitionedParquetNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_register_hive_parquet");
        assert_eq!(node.friendly_name, "Register Hive Parquet");
    }

    #[test]
    fn test_register_partitioned_json_node_structure() {
        let node_logic = RegisterPartitionedJsonNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_register_partitioned_json");
        assert_eq!(node.friendly_name, "Register Partitioned JSON");
    }

    #[test]
    fn test_write_delta_table_node_structure() {
        let node_logic = WriteDeltaTableNode::new();
        let node = node_logic.get_node();

        assert_eq!(node.name, "df_write_delta");
        assert_eq!(node.friendly_name, "Write Delta Table");
        assert_eq!(node.category, "Data/DataFusion/Lakes");
    }

    #[test]
    fn test_write_delta_table_node_input_pins() {
        let node_logic = WriteDeltaTableNode::new();
        let node = node_logic.get_node();

        let input_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Input).collect();

        assert!(input_pins.iter().any(|p| p.name == "query"));
        assert!(input_pins.iter().any(|p| p.name == "path"));
        assert!(input_pins.iter().any(|p| p.name == "mode"));
        assert!(input_pins.iter().any(|p| p.name == "partition_by"));
    }

    #[test]
    fn test_write_delta_table_node_output_pins() {
        let node_logic = WriteDeltaTableNode::new();
        let node = node_logic.get_node();

        let output_pins: Vec<_> = node.pins.values().filter(|p| p.pin_type == PinType::Output).collect();

        assert!(output_pins.iter().any(|p| p.name == "rows_written"));
        assert!(output_pins.iter().any(|p| p.name == "new_version"));
    }

    #[test]
    fn test_all_delta_nodes_have_scores() {
        let register_node = RegisterDeltaTableNode::new().get_node();
        let time_travel_node = DeltaTimeTravelNode::new().get_node();
        let info_node = DeltaTableInfoNode::new().get_node();
        let write_node = WriteDeltaTableNode::new().get_node();
        let hive_node = RegisterHivePartitionedParquetNode::new().get_node();
        let json_node = RegisterPartitionedJsonNode::new().get_node();

        assert!(register_node.scores.is_some());
        assert!(time_travel_node.scores.is_some());
        assert!(info_node.scores.is_some());
        assert!(write_node.scores.is_some());
        assert!(hive_node.scores.is_some());
        assert!(json_node.scores.is_some());
    }

    #[test]
    fn test_all_delta_nodes_use_flowpath() {
        let nodes = vec![
            RegisterDeltaTableNode::new().get_node(),
            DeltaTimeTravelNode::new().get_node(),
            DeltaTableInfoNode::new().get_node(),
            WriteDeltaTableNode::new().get_node(),
            RegisterHivePartitionedParquetNode::new().get_node(),
            RegisterPartitionedJsonNode::new().get_node(),
        ];

        for node in nodes {
            let path_pin = node.pins.values().find(|p| p.name == "path");
            assert!(path_pin.is_some(), "Node {} missing path pin", node.name);
            assert_eq!(
                path_pin.unwrap().data_type,
                VariableType::Struct,
                "Node {} path pin should be Struct type",
                node.name
            );
        }
    }
}
