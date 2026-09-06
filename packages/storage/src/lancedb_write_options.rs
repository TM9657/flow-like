use lance::dataset::{WriteMode, WriteParams};
use lance_file::version::LanceFileVersion;

/// Build default LanceDB write options for new datasets and overwrites.
///
/// Request V2.2 storage and append mode, preserving Flow-Like's write preferences.
/// The dataset's existing format remains readable, and appends must not recreate it.
pub fn default_write_options() -> lancedb::table::WriteOptions {
    lancedb::table::WriteOptions {
        lance_write_params: Some(WriteParams {
            data_storage_version: Some(LanceFileVersion::V2_2),
            mode: WriteMode::Append,
            ..Default::default()
        }),
    }
}
