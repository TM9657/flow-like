use object_store::path::Path;

#[derive(Clone, Debug)]
pub enum PathChange {
    Edited {
        path: Path,
        old_hash: String,
        new_hash: String,
    },
    Removed {
        path: Path,
        hash: String,
    },
    Added {
        path: Path,
        hash: String,
    },
    Renamed {
        old_path: Path,
        new_path: Path,
        hash: String,
    },
    Moved {
        old_path: Path,
        new_path: Path,
        hash: String,
    },
}
