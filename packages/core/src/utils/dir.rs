use std::path::PathBuf;

pub fn dir_size(path: &PathBuf) -> anyhow::Result<u64> {
    let size = fs_extra::dir::get_size(path)?;
    Ok(size)
}
