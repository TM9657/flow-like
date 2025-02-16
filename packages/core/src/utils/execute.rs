// Inspired by the Tauri project implementation
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tokio::process::{self, Command};

pub fn executable_path() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?;
    let parent = path.parent()?;
    Some(parent.to_path_buf())
}

fn side_car_path(command: &PathBuf) -> anyhow::Result<PathBuf> {
    let executable = executable_path().ok_or(anyhow::anyhow!("Could not get executable path"))?;
    #[cfg(windows)]
    return Ok(executable.join(&command).with_extension("exe"));
    #[cfg(not(windows))]
    return Ok(executable.join(command));
}

// TODO: replace the input with a Bit
pub async fn sidecar(command: &PathBuf) -> anyhow::Result<StdCommand> {
    let path = side_car_path(command)?;

    if !path.exists() {
        return Err(anyhow::anyhow!("Sidecar not found at path: {:?}", path));
    }

    if !path.is_file() {
        return Err(anyhow::anyhow!("Sidecar is not a file: {:?}", path));
    }

    let sidecar = StdCommand::new(path);
    Ok(sidecar)
}

//
pub async fn async_sidecar(command: &PathBuf) -> anyhow::Result<Command> {
    let path = side_car_path(command)?;

    if !path.exists() {
        return Err(anyhow::anyhow!("Sidecar not found at path: {:?}", path));
    }

    if !path.is_file() {
        return Err(anyhow::anyhow!("Sidecar is not a file: {:?}", path));
    }

    let sidecar = process::Command::new(path);
    Ok(sidecar)
}

// ==================== IDEAS ====================
// - Sidecar BIT function
