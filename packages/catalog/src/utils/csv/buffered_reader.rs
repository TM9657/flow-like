use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::object_store::buffered::BufReader;
use flow_like_types::{
    async_trait,
    json::{json, to_value},
};
use futures::StreamExt;

use crate::data::path::FlowPath;

#[crate::register_node]
#[derive(Default)]
pub struct BufferedCsvReaderNode {}

impl BufferedCsvReaderNode {
    pub fn new() -> Self {
        BufferedCsvReaderNode {}
    }
}

#[async_trait]
impl NodeLogic for BufferedCsvReaderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "csv_buffered_reader",
            "Buffered CSV Reader",
            "Stream Read a CSV File",
            "Utils/CSV",
        );

        // node.add_icon("/flow/icons/bool.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("csv", "CSV", "CSV Path", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "chunk_size",
            "Chunk Size",
            "Chunk Size for Buffered Read",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10_000)));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Delimiter for CSV",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_output_pin(
            "for_chunk",
            "For Chunk",
            "Executes for each chunk",
            VariableType::Execution,
        );

        node.add_output_pin("chunk", "Chunk", "Chunk", VariableType::Struct)
            .set_value_type(flow_like::flow::pin::ValueType::Array);

        node.add_output_pin("exec_done", "Done", "Done", VariableType::Execution);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_done").await?;
        context.activate_exec_pin("for_chunk").await?;
        let exec_item = context.get_pin_by_name("for_chunk").await?;
        let value = context.get_pin_by_name("chunk").await?;

        let delimiter: String = context.evaluate_pin("delimiter").await?;
        let delimiter = delimiter.as_bytes()[0];
        let csv_path: FlowPath = context.evaluate_pin("csv").await?;
        let store = csv_path.to_runtime(context).await?;
        let location = store.path.clone();
        let get_request = store.store.as_generic().get(&location).await?;
        let reader = BufReader::new(store.store.as_generic(), &get_request.meta);

        let mut rdr = csv_async::AsyncReaderBuilder::new()
            .has_headers(true)
            .buffer_capacity(32 * 1024 * 1024)
            .delimiter(delimiter)
            .create_reader(reader);

        let chunk_size: u64 = context.evaluate_pin("chunk_size").await?;
        let headers = rdr.byte_headers().await?.clone();
        let headers = headers
            .iter()
            .map(|h| {
                let lossy_header = String::from_utf8_lossy(h);
                lossy_header.to_string()
            })
            .collect::<Vec<String>>();

        let mut records = rdr.byte_records();
        let mut chunk = Vec::with_capacity(chunk_size as usize);
        let flow = exec_item.lock().await.get_connected_nodes().await;

        while let Some(element) = records.next().await {
            let record = match element {
                Ok(record) => record,
                Err(e) => {
                    println!("Error reading record: {:?}", e);
                    continue;
                }
            };
            let json_obj =
                headers
                    .iter()
                    .zip(record.iter())
                    .fold(json!({}), |mut acc, (header, value)| {
                        let lossy_value = String::from_utf8_lossy(value);
                        acc[header] = json!(lossy_value.to_string());
                        acc
                    });
            chunk.push(json_obj);
            if chunk.len() as u64 == chunk_size {
                value.lock().await.set_value(to_value(&chunk)?).await;
                chunk = Vec::with_capacity(chunk_size as usize);
                for node in &flow {
                    let mut sub_context = context.create_sub_context(node).await;
                    let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;
                    sub_context.end_trace();
                    context.push_sub_context(&mut sub_context);

                    if run.is_err() {
                        let error = run.err().unwrap();
                        context.log_message(&format!("Error: {:?}", error), LogLevel::Error);
                    }
                }
            }
        }

        if !chunk.is_empty() {
            value.lock().await.set_value(to_value(&chunk)?).await;
            for node in &flow {
                let mut sub_context = context.create_sub_context(node).await;
                let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;
                sub_context.end_trace();
                context.push_sub_context(&mut sub_context);

                if run.is_err() {
                    let error = run.err().unwrap();
                    context.log_message(&format!("Error: {:?}", error), LogLevel::Error);
                }
            }
        }

        context.activate_exec_pin("exec_done").await?;
        context.deactivate_exec_pin("for_chunk").await?;

        return Ok(());
    }
}

// async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
//     context.deactivate_exec_pin("exec_done").await?;
//     context.activate_exec_pin("for_chunk").await?;

//     let csv_path: FlowPath = context.evaluate_pin("csv").await?;
//     let store = csv_path.to_runtime(context).await?;
//     let location = store.path.clone();
//     let csv_bytes = store
//     .store
//     .as_generic()
//     .get(&location)
//     .await?
//     .bytes().await?;

//     let cursor = std::io::Cursor::new(csv_bytes);
//     let mut rdr = csv::ReaderBuilder::new()
//         .has_headers(true)
//         .from_reader(cursor);

//     let chunk_size: u64 = context.evaluate_pin("chunk_size").await?;;
//     let headers = rdr.headers().iter().map(|h| {
//         let bytes = h.as_byte_record();
//         let lossy_header = String::from_utf8_lossy(bytes.as_slice());
//         lossy_header.to_string()
//     }).collect::<Vec<String>>();

//     let records = rdr.byte_records();
//     let mut chunk = Vec::with_capacity(chunk_size as usize);
//     for element in records{
//         let record = match element {
//             Ok(record) => record,
//             Err(e) => {
//                 continue;
//             }
//         };
//         let json_obj= headers.iter().zip(record.iter()).fold(json!({}), |mut acc, (header, value)| {
//             let lossy_value = String::from_utf8_lossy(value);
//             acc[header] = json!(lossy_value.to_string());
//             acc
//         });
//         chunk.push(json_obj);
//         if chunk.len() as u64 == chunk_size {
//             println!("Chunk: {:?}", chunk.len());
//             chunk = Vec::with_capacity(chunk_size as usize);
//         }
//     }

//     return Ok(());
// }
