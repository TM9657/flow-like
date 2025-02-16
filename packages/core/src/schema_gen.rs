use crate::{
    bit::{
        Bit, BitModelPreference, BitPack, BitProvider, EmbeddingModelParameters,
        ImageEmbeddingModelParameters, LLMParameters, VLMParameters,
    },
    flow::{board::Board, execution::Run, node::Node, pin::Pin, variable::Variable},
    hub::Hub,
    models::{history::History, response::Response, response_chunk::ResponseChunk},
    profile::Profile,
    utils::file::FileMetadata,
    vault::Vault,
};
use anyhow::Result;
use schemars::{schema_for, JsonSchema};
use serde::Serialize;
use serde_json::to_string_pretty;
use std::{
    fs::write,
    path::{Path, PathBuf},
};

fn save_schema<T: ?Sized + Serialize>(schema: &T, path: &PathBuf) -> Result<()> {
    let schema_str = to_string_pretty(schema)?;
    write(path, schema_str)?;
    Ok(())
}

fn generate_and_save_schema<T: Serialize + JsonSchema>(base_path: &Path, path: &str) -> Result<()> {
    let schema = schema_for!(T);
    let full_path = base_path.join(path);

    if !full_path.parent().unwrap().exists() {
        std::fs::create_dir_all(full_path.parent().unwrap())?;
    }

    save_schema(&schema, &full_path)
}
pub fn generate_schema(base_path: PathBuf) -> anyhow::Result<()> {
    generate_and_save_schema::<History>(&base_path, "llm/history.json")?;
    generate_and_save_schema::<Response>(&base_path, "llm/response.json")?;
    generate_and_save_schema::<ResponseChunk>(&base_path, "llm/response-chunk.json")?;

    generate_and_save_schema::<EmbeddingModelParameters>(
        &base_path,
        "bit/bit/embedding-model-parameters.json",
    )?;
    generate_and_save_schema::<ImageEmbeddingModelParameters>(
        &base_path,
        "bit/bit/image-embedding-model-parameters.json",
    )?;
    generate_and_save_schema::<VLMParameters>(&base_path, "bit/bit/vlm-parameters.json")?;
    generate_and_save_schema::<LLMParameters>(&base_path, "bit/bit/llm-parameters.json")?;
    generate_and_save_schema::<BitProvider>(&base_path, "bit/bit/provider.json")?;

    generate_and_save_schema::<Bit>(&base_path, "bit/bit.json")?;
    generate_and_save_schema::<BitModelPreference>(&base_path, "bit/preferences.json")?;
    generate_and_save_schema::<BitPack>(&base_path, "bit/bit-pack.json")?;

    generate_and_save_schema::<Board>(&base_path, "flow/board.json")?;
    generate_and_save_schema::<Node>(&base_path, "flow/node.json")?;
    generate_and_save_schema::<Pin>(&base_path, "flow/pin.json")?;
    generate_and_save_schema::<Variable>(&base_path, "flow/variable.json")?;
    generate_and_save_schema::<Run>(&base_path, "flow/run.json")?;

    generate_and_save_schema::<Profile>(&base_path, "profile/profile.json")?;

    generate_and_save_schema::<Hub>(&base_path, "hub/hub.json")?;

    generate_and_save_schema::<Vault>(&base_path, "vault/vault.json")?;

    generate_and_save_schema::<FileMetadata>(&base_path, "files/file-metadata.json")?;

    Ok(())
}
