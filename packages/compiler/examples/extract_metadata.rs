//! Read node metadata with the same limits used by the compilation worker.
//!
//! cargo run -p flow-like-compiler --example extract_metadata -- path/to/node.wasm

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args_os().skip(1).collect();
    if paths.is_empty() {
        return Err("provide one or more WASM artifact paths".into());
    }

    let mut failed = false;
    for path in paths {
        let path = std::path::PathBuf::from(path);
        let bytes = tokio::fs::read(&path).await?;
        let wasm_hash = blake3::hash(&bytes).to_hex().to_string();
        let result = match flow_like_compiler::extract_nodes(&bytes).await {
            Ok(nodes) => serde_json::json!({
                "path": path,
                "wasm_hash": wasm_hash,
                "node_count": nodes.len(),
                "nodes": nodes,
            }),
            Err(error) => {
                failed = true;
                serde_json::json!({
                    "path": path,
                    "wasm_hash": wasm_hash,
                    "error": error.to_string(),
                })
            }
        };
        println!("{}", serde_json::to_string(&result)?);
    }
    if failed {
        return Err("metadata extraction failed for one or more artifacts".into());
    }
    Ok(())
}
