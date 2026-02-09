use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{
    JsonSchema, async_trait,
    json::{Deserialize, Serialize, json},
};

use crate::data::path::FlowPath;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TdmsChannelInfo {
    pub name: String,
    pub group: String,
    pub data_type: String,
    pub properties: Vec<TdmsPropertyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TdmsPropertyInfo {
    pub name: String,
    pub data_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TdmsGroupInfo {
    pub name: String,
    pub channels: Vec<TdmsChannelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TdmsFileMetadata {
    pub groups: Vec<TdmsGroupInfo>,
    pub segment_count: u64,
}

#[crate::register_node]
#[derive(Default)]
pub struct TdmsMetadataNode {}

impl TdmsMetadataNode {
    pub fn new() -> Self {
        TdmsMetadataNode {}
    }
}

#[cfg(feature = "execute")]
fn format_data_type(dt: &tdms::data_type::TdmsDataType) -> String {
    format!("{:?}", dt)
}

#[cfg(feature = "execute")]
fn format_value(val: &tdms::data_type::TDMSValue) -> String {
    format!("{:?}", val.value)
}

#[async_trait]
impl NodeLogic for TdmsMetadataNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "tdms_metadata",
            "TDMS Metadata",
            "Extracts metadata (groups, channels, properties) from a LabVIEW TDMS file.",
            "Data/TDMS",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);
        node.add_input_pin(
            "tdms_path",
            "TDMS File",
            "Path to the TDMS file",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Done",
            "Finished extracting metadata",
            VariableType::Execution,
        );
        node.add_output_pin(
            "metadata",
            "Metadata",
            "TDMS file metadata struct",
            VariableType::Struct,
        )
        .set_schema::<TdmsFileMetadata>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let tdms_path: FlowPath = context.evaluate_pin("tdms_path").await?;
        let bytes = tdms_path.get(context, false).await?;

        let tmp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(tmp_file.path(), &bytes)?;

        let file = tdms::TDMSFile::from_path(tmp_file.path())
            .map_err(|e| flow_like_types::anyhow!("Failed to parse TDMS file: {:?}", e))?;

        let group_names = file.groups();
        let mut groups = Vec::with_capacity(group_names.len());

        for group_name in &group_names {
            let channels_map = file.channels(group_name);
            let mut channels = Vec::with_capacity(channels_map.len());

            for (_path, channel) in &channels_map {
                let properties: Vec<TdmsPropertyInfo> = channel
                    .properties
                    .iter()
                    .map(|p| TdmsPropertyInfo {
                        name: p.name.clone(),
                        data_type: format_data_type(&p.data_type),
                        value: format_value(&p.value),
                    })
                    .collect();

                channels.push(TdmsChannelInfo {
                    name: channel.path.clone(),
                    group: channel.group_path.clone(),
                    data_type: format_data_type(&channel.data_type),
                    properties,
                });
            }

            groups.push(TdmsGroupInfo {
                name: group_name.clone(),
                channels,
            });
        }

        let metadata = TdmsFileMetadata {
            segment_count: file.segments.len() as u64,
            groups,
        };

        context.set_pin_value("metadata", json!(metadata)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "TDMS parsing requires the 'execute' feature"
        ))
    }
}
