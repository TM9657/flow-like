use crate::{
    app::App,
    bit::{
        Bit, BitModelPreference, BitPack, EmbeddingModelParameters, ImageEmbeddingModelParameters,
        LLMParameters, VLMParameters,
    },
    flow::{
        board::{
            Board,
            commands::{
                GenericCommand,
                comments::{
                    remove_comment::RemoveCommentCommand, upsert_comment::UpsertCommentCommand,
                },
                nodes::{
                    add_node::AddNodeCommand, copy_paste::CopyPasteCommand,
                    move_node::MoveNodeCommand, remove_node::RemoveNodeCommand,
                    update_node::UpdateNodeCommand,
                },
                pins::{
                    connect_pins::ConnectPinsCommand, disconnect_pins::DisconnectPinsCommand,
                    upsert_pin::UpsertPinCommand,
                },
                variables::{
                    remove_variable::RemoveVariableCommand, upsert_variable::UpsertVariableCommand,
                },
            },
        },
        execution::Run,
        node::Node,
        pin::Pin,
        variable::Variable,
    },
    hub::Hub,
    profile::Profile,
    utils::file::FileMetadata,
};
use flow_like_model_provider::{
    history::History, response::Response, response_chunk::ResponseChunk,
};
use flow_like_types::{Result, json::to_string_pretty};
use schemars::{JsonSchema, schema_for};
use serde::Serialize;
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
pub fn generate_schema(base_path: PathBuf) -> flow_like_types::Result<()> {
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

    generate_and_save_schema::<Bit>(&base_path, "bit/bit.json")?;
    generate_and_save_schema::<BitModelPreference>(&base_path, "bit/preferences.json")?;
    generate_and_save_schema::<BitPack>(&base_path, "bit/bit-pack.json")?;

    generate_and_save_schema::<Board>(&base_path, "flow/board.json")?;
    generate_and_save_schema::<GenericCommand>(&base_path, "flow/board/commands/generic.json")?;
    generate_and_save_schema::<RemoveCommentCommand>(
        &base_path,
        "flow/board/commands/remove-comment.json",
    )?;
    generate_and_save_schema::<UpsertCommentCommand>(
        &base_path,
        "flow/board/commands/upsert-comment.json",
    )?;
    generate_and_save_schema::<AddNodeCommand>(&base_path, "flow/board/commands/add-node.json")?;
    generate_and_save_schema::<CopyPasteCommand>(
        &base_path,
        "flow/board/commands/copy-paste.json",
    )?;
    generate_and_save_schema::<MoveNodeCommand>(&base_path, "flow/board/commands/move-node.json")?;
    generate_and_save_schema::<RemoveNodeCommand>(
        &base_path,
        "flow/board/commands/remove-node.json",
    )?;
    generate_and_save_schema::<UpdateNodeCommand>(
        &base_path,
        "flow/board/commands/update-node.json",
    )?;
    generate_and_save_schema::<DisconnectPinsCommand>(
        &base_path,
        "flow/board/commands/disconnect-pins.json",
    )?;
    generate_and_save_schema::<ConnectPinsCommand>(
        &base_path,
        "flow/board/commands/connect-pins.json",
    )?;
    generate_and_save_schema::<UpsertPinCommand>(
        &base_path,
        "flow/board/commands/upsert-pin.json",
    )?;
    generate_and_save_schema::<RemoveVariableCommand>(
        &base_path,
        "flow/board/commands/remove-variable.json",
    )?;
    generate_and_save_schema::<UpsertVariableCommand>(
        &base_path,
        "flow/board/commands/upsert-variable.json",
    )?;
    generate_and_save_schema::<Node>(&base_path, "flow/node.json")?;
    generate_and_save_schema::<Pin>(&base_path, "flow/pin.json")?;
    generate_and_save_schema::<Variable>(&base_path, "flow/variable.json")?;
    generate_and_save_schema::<Run>(&base_path, "flow/run.json")?;

    generate_and_save_schema::<Profile>(&base_path, "profile/profile.json")?;

    generate_and_save_schema::<Hub>(&base_path, "hub/hub.json")?;

    generate_and_save_schema::<App>(&base_path, "app/app.json")?;

    generate_and_save_schema::<FileMetadata>(&base_path, "files/file-metadata.json")?;

    Ok(())
}
