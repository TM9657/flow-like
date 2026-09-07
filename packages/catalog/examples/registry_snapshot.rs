//! Export the product catalog and the exact fingerprint used for compiled boards.
//!
//! Run each profile separately so Cargo does not merge their feature sets:
//! `cargo run -p flow-like-catalog --example registry_snapshot --no-default-features --features server-local-metadata -- /tmp/native-api.json`
//! `cargo run -p flow-like-catalog --example registry_snapshot --no-default-features --features server-local-ml -- /tmp/native-executor.json`
//! Add `--overlay nodes.json` to overlay a JSON array of node definitions before hashing.
//! Omit the output path to write JSON to stdout. This reads node definitions without
//! initializing execution, loading WASM packages, or calling host services.

use flow_like::state::FlowNodeRegistryInner;
use flow_like::{
    flow::execution::context::ExecutionContext,
    flow::node::{Node, NodeLogic},
};
use flow_like_types::async_trait;
use serde::Serialize;
use std::{
    error::Error,
    fs::File,
    io::{self, BufWriter, Write},
    sync::Arc,
};

#[derive(Serialize)]
struct RegistrySnapshot {
    registry_fingerprint: String,
    node_count: usize,
    nodes: Vec<NodeSnapshot>,
}

#[derive(Serialize)]
struct NodeSnapshot {
    name: String,
    version: Option<u32>,
    semantic_hash: String,
    node: Node,
}

struct MetadataNode(Node);

#[async_trait]
impl NodeLogic for MetadataNode {
    fn get_node(&self) -> Node {
        self.0.clone()
    }

    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "Snapshot nodes contain metadata only"
        ))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let mut output = None;
    let mut overlay = None;
    while let Some(arg) = args.next() {
        if arg == "--overlay" && overlay.is_none() {
            overlay = Some(args.next().ok_or("--overlay requires a JSON file path")?);
        } else if output.is_none() {
            output = Some(arg);
        } else {
            return Err("Usage: registry_snapshot [output.json] [--overlay nodes.json]".into());
        }
    }

    let catalog = Arc::new(flow_like_catalog::get_catalog());
    let mut registry = FlowNodeRegistryInner::prepare(&catalog);
    if let Some(path) = overlay {
        let nodes: Vec<Node> = serde_json::from_reader(File::open(path)?)?;
        for node in nodes {
            registry.insert(node.clone(), Arc::new(MetadataNode(node)));
        }
    }
    let registry_fingerprint = registry
        .fingerprint()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut nodes: Vec<_> = registry
        .registry
        .values()
        .map(|(node, _)| NodeSnapshot {
            name: node.name.clone(),
            version: node.version,
            semantic_hash: format!("{:016x}", node.semantic_hash()),
            node: node.clone(),
        })
        .collect();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    let snapshot = RegistrySnapshot {
        registry_fingerprint,
        node_count: nodes.len(),
        nodes,
    };

    let mut writer: Box<dyn Write> = match output {
        Some(path) => Box::new(BufWriter::new(File::create(path)?)),
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };
    serde_json::to_writer_pretty(&mut writer, &snapshot)?;
    writeln!(writer)?;
    writer.flush()?;
    eprintln!(
        "Exported {} nodes; registry fingerprint {}",
        snapshot.node_count, snapshot.registry_fingerprint
    );
    Ok(())
}
