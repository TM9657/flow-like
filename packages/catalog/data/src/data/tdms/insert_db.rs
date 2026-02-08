use flow_like::flow::{
    execution::{LogLevel, context::ExecutionContext},
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
#[cfg(feature = "execute")]
use flow_like_storage::databases::vector::VectorStore;
use flow_like_types::{async_trait, json::json};

use crate::data::db::vector::NodeDBConnection;
use crate::data::path::FlowPath;

#[crate::register_node]
#[derive(Default)]
pub struct BatchInsertTdmsLocalDatabaseNode {}

impl BatchInsertTdmsLocalDatabaseNode {
    pub fn new() -> Self {
        BatchInsertTdmsLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for BatchInsertTdmsLocalDatabaseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "tdms_insert_local_db",
            "Batch Insert (TDMS)",
            "Reads a LabVIEW TDMS file and batch-inserts its channel data as rows into a vector database. Each row is a JSON object with channel names as keys.",
            "Data/Database/Insert",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);
        node.add_input_pin(
            "database",
            "Database",
            "Database Connection Reference",
            VariableType::Struct,
        )
        .set_schema::<NodeDBConnection>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "tdms_path",
            "TDMS File",
            "Path to the TDMS file",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "chunk_size",
            "Chunk Size",
            "Chunk Size for Buffered Insert",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10_000)));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Finished inserting TDMS data",
            VariableType::Execution,
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context).await?.db.clone();
        let mut database = database.write().await;
        let chunk_size: u64 = context.evaluate_pin("chunk_size").await?;

        let tdms_path: FlowPath = context.evaluate_pin("tdms_path").await?;
        let bytes = tdms_path.get(context, false).await?;

        let tmp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(tmp_file.path(), &bytes)?;

        let file = tdms::TDMSFile::from_path(tmp_file.path())
            .map_err(|e| flow_like_types::anyhow!("Failed to parse TDMS file: {:?}", e))?;

        let group_names = file.groups();

        for group_name in &group_names {
            let channels_map = file.channels(group_name);

            let channel_data: Vec<(String, Vec<String>)> = channels_map
                .iter()
                .filter_map(|(_path, channel)| {
                    let name = channel.path.clone();
                    let values = extract_channel_as_strings(&file, channel);
                    values.map(|v| (name, v))
                })
                .collect();

            if channel_data.is_empty() {
                continue;
            }

            let row_count = channel_data.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
            let mut chunk = Vec::with_capacity(chunk_size as usize);

            for row_idx in 0..row_count {
                let mut row = json!({});
                row["_group"] = json!(group_name);

                for (col_name, col_values) in &channel_data {
                    if row_idx < col_values.len() {
                        row[col_name] = json!(col_values[row_idx]);
                    }
                }

                chunk.push(row);

                if chunk.len() as u64 == chunk_size {
                    if let Err(e) = database.insert(chunk.clone()).await {
                        context.log_message(
                            &format!("Error inserting TDMS chunk: {:?}", e),
                            LogLevel::Error,
                        );
                    }
                    chunk = Vec::with_capacity(chunk_size as usize);
                }
            }

            if !chunk.is_empty() {
                if let Err(e) = database.insert(chunk).await {
                    context.log_message(
                        &format!("Error inserting final TDMS chunk: {:?}", e),
                        LogLevel::Error,
                    );
                }
            }
        }

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "TDMS processing requires the 'execute' feature"
        ))
    }
}

#[cfg(feature = "execute")]
fn extract_channel_as_strings<'a>(
    file: &'a tdms::TDMSFile<'a>,
    channel: &'a tdms::segment::Channel,
) -> Option<Vec<String>> {
    use tdms::data_type::TdmsDataType;

    match channel.data_type {
        TdmsDataType::DoubleFloat(_) => file
            .channel_data_double_float(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::SingleFloat(_) => file
            .channel_data_single_float(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::I8(_) => file
            .channel_data_i8(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::I16(_) => file
            .channel_data_i16(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::I32(_) => file
            .channel_data_i32(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::I64(_) => file
            .channel_data_i64(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::U8(_) => file
            .channel_data_u8(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::U16(_) => file
            .channel_data_u16(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::U32(_) => file
            .channel_data_u32(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::U64(_) => file
            .channel_data_u64(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::Boolean(_) => file
            .channel_data_bool(channel)
            .ok()
            .map(|iter| iter.map(|v| v.to_string()).collect()),
        TdmsDataType::String => file
            .channel_data_string(channel)
            .ok()
            .map(|iter| iter.collect()),
        TdmsDataType::TimeStamp(_) => file
            .channel_data_timestamp(channel)
            .ok()
            .map(|iter| iter.map(|v| format!("{:?}", v)).collect()),
        _ => None,
    }
}
