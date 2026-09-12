use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};

/// Directories the runtime keeps inside the bit store that are not artifacts:
/// the dependency tree cache and the MLX weight cache.
const RESERVED_DIRS: [&str; 2] = ["deps-cache", "mlx-models"];
const PARTIAL_SUFFIX: &str = ".download";

#[derive(Clone)]
pub struct Artifact {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub partial: bool,
}

#[derive(Clone)]
pub struct Entry {
    pub hash: String,
    pub path: PathBuf,
    pub artifacts: Vec<Artifact>,
}

impl Entry {
    pub fn size(&self) -> u64 {
        self.artifacts.iter().map(|artifact| artifact.size).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    pub fn artifact(&self, name: &str) -> Option<&Artifact> {
        self.artifacts
            .iter()
            .find(|artifact| !artifact.partial && artifact.name == name)
    }
}

pub fn scan(bit_dir: &Path) -> Result<Vec<Entry>> {
    if !bit_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for dir in fs::read_dir(bit_dir)
        .with_context(|| format!("could not read the bit store at {}", bit_dir.display()))?
    {
        let dir = dir?;
        if !dir.file_type()?.is_dir() {
            continue;
        }

        let hash = dir.file_name().to_string_lossy().to_string();
        if RESERVED_DIRS.contains(&hash.as_str()) {
            continue;
        }

        let path = dir.path();
        let mut artifacts = Vec::new();
        collect(&path, &path, &mut artifacts)?;
        artifacts.sort_by(|left, right| left.name.cmp(&right.name));
        entries.push(Entry {
            hash,
            path,
            artifacts,
        });
    }

    entries.sort_by(|left, right| left.hash.cmp(&right.hash));
    Ok(entries)
}

pub fn find<'a>(entries: &'a [Entry], hash: &str) -> Option<&'a Entry> {
    entries.iter().find(|entry| entry.hash == hash)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<Artifact>) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("could not read the directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect(root, &path, out)?;
            continue;
        }

        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        out.push(Artifact {
            partial: name.ends_with(PARTIAL_SUFFIX),
            size: entry.metadata()?.len(),
            name,
            path,
        });
    }

    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    EmptyDir,
    StalePartial,
    ZeroByte,
    Unfinished,
}

impl IssueKind {
    pub fn label(self) -> &'static str {
        match self {
            IssueKind::EmptyDir => "empty",
            IssueKind::StalePartial => "stale partial",
            IssueKind::ZeroByte => "zero byte",
            IssueKind::Unfinished => "unfinished",
        }
    }
}

pub struct Issue {
    pub hash: String,
    pub kind: IssueKind,
    pub detail: String,
    pub bytes: u64,
    /// What `doctor --fix` deletes. An unfinished download resumes, so it is
    /// reported without a removal.
    pub removable: Option<PathBuf>,
}

pub fn issues(entries: &[Entry]) -> Vec<Issue> {
    let mut issues = Vec::new();

    for entry in entries {
        if entry.is_empty() {
            issues.push(Issue {
                hash: entry.hash.clone(),
                kind: IssueKind::EmptyDir,
                detail: "holds no artifact, the download never produced a file".to_string(),
                bytes: 0,
                removable: Some(entry.path.clone()),
            });
            continue;
        }

        for artifact in &entry.artifacts {
            if artifact.partial {
                let completed = artifact.name.trim_end_matches(PARTIAL_SUFFIX);
                let superseded = entry.artifact(completed).is_some();

                issues.push(Issue {
                    hash: entry.hash.clone(),
                    kind: if superseded {
                        IssueKind::StalePartial
                    } else {
                        IssueKind::Unfinished
                    },
                    detail: artifact.name.clone(),
                    bytes: artifact.size,
                    removable: superseded.then(|| artifact.path.clone()),
                });
                continue;
            }

            if artifact.size == 0 {
                issues.push(Issue {
                    hash: entry.hash.clone(),
                    kind: IssueKind::ZeroByte,
                    detail: artifact.name.clone(),
                    bytes: 0,
                    removable: Some(artifact.path.clone()),
                });
            }
        }
    }

    issues
}

pub fn remove(path: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("could not read {}", path.display()))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .with_context(|| format!("could not remove {}", path.display()))
}
