use std::path::PathBuf;

pub fn dir_size(path: &PathBuf) -> flow_like_types::Result<u64> {
    let size = fs_extra::dir::get_size(path)?;
    Ok(size)
}
