//! Storage paths shared by credential issuers and runtime writers.

const MAX_STORAGE_PATH_SEGMENT_CHARS: usize = 80;
const STORAGE_PATH_SEGMENT_DIGEST_CHARS: usize = 12;

/// Keep safe identifiers readable; append a digest when rewriting could collide.
pub fn storage_path_segment(value: &str, fallback: &str) -> String {
    let mut sanitized = String::with_capacity(value.len().min(MAX_STORAGE_PATH_SEGMENT_CHARS));
    let mut lossy = value.chars().count() > MAX_STORAGE_PATH_SEGMENT_CHARS;
    for ch in value.chars().take(MAX_STORAGE_PATH_SEGMENT_CHARS) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
            lossy = true;
        }
    }

    let trimmed = sanitized.trim_matches(|ch| ch == '.' || ch == '_');
    lossy |= trimmed.len() != sanitized.len();

    if !lossy {
        return if trimmed.is_empty() {
            fallback.to_string()
        } else {
            trimmed.to_string()
        };
    }

    let digest = blake3::hash(value.as_bytes()).to_hex();
    let digest = &digest[..STORAGE_PATH_SEGMENT_DIGEST_CHARS];
    let base = if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    };
    let base: String = base
        .chars()
        .take(MAX_STORAGE_PATH_SEGMENT_CHARS - STORAGE_PATH_SEGMENT_DIGEST_CHARS - 1)
        .collect();
    let base = base.trim_end_matches(['.', '_']);
    let base = if base.is_empty() { fallback } else { base };
    format!("{base}-{digest}")
}

/// The scratch prefixes every scoped credential must authorise, built from the
/// same segments the writers use. Returned as a pair so no caller can sanitise
/// one and forget the other.
pub fn temporary_prefixes(sub: &str, app_id: &str) -> (String, String) {
    let app_segment = storage_path_segment(app_id, "app");
    (
        format!(
            "tmp/user/{}/apps/{}",
            storage_path_segment(sub, "user"),
            app_segment
        ),
        format!("tmp/global/apps/{}", app_segment),
    )
}

/// Device-local training ledgers live under this root of the Local app-storage store.
pub const TRAINING_LEDGER_ROOT: &str = "training-ledgers";

/// Published models live under this directory of an app or user model root.
pub const PUBLISHED_MODELS_DIR: &str = "models";

/// App-relative locations of the pre-device-local App ledger. It stays on the
/// device and is excluded from every copy path.
pub const LEGACY_TRAINING_LEDGER_PREFIXES: [&str; 3] = [
    "storage/inspection/jobs.sqlite",
    "storage/inspection/jobs.artifacts/",
    "storage/inspection/.training-locks/",
];

const LEGACY_LEDGER_DATABASE_SUFFIXES: [&str; 4] = ["", "-wal", "-shm", "-journal"];

pub fn training_ledger_app_dir(app_id: &str) -> String {
    format!(
        "{TRAINING_LEDGER_ROOT}/apps/{}",
        storage_path_segment(app_id, "app")
    )
}

/// `None` is the shared App ledger; `Some(sub)` is that user's private ledger.
pub fn training_ledger_dir(app_id: &str, user_sub: Option<&str>) -> String {
    let app_dir = training_ledger_app_dir(app_id);
    match user_sub {
        None => format!("{app_dir}/project"),
        Some(sub) => format!("{app_dir}/users/{}", storage_path_segment(sub, "user")),
    }
}

/// `app_relative` is relative to `apps/{app}/`. The database prefix matches
/// only the SQLite file and its `-wal`, `-shm` and `-journal` companions; the
/// directory prefixes match the directory and everything below it. ASCII case
/// is ignored, because macOS and Windows open case variants as the same file.
pub fn is_legacy_training_ledger_path(app_relative: &str) -> bool {
    let path = app_relative.trim_start_matches('/').to_ascii_lowercase();
    LEGACY_TRAINING_LEDGER_PREFIXES
        .iter()
        .any(|prefix| match prefix.strip_suffix('/') {
            Some(directory) => path == directory || path.starts_with(prefix),
            None => path
                .strip_prefix(prefix)
                .is_some_and(|rest| LEGACY_LEDGER_DATABASE_SUFFIXES.contains(&rest)),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_ledger_dirs_use_sanitized_segments() {
        assert_eq!(
            training_ledger_app_dir("app-1"),
            "training-ledgers/apps/app-1"
        );
        assert_eq!(
            training_ledger_dir("app-1", None),
            "training-ledgers/apps/app-1/project"
        );
        let private = training_ledger_dir("app-1", Some("google-oauth2|1"));
        let segment = private
            .strip_prefix("training-ledgers/apps/app-1/users/")
            .expect("private ledger lives under the app's users directory");
        let (base, digest) = segment
            .rsplit_once('-')
            .expect("a rewritten sub carries a digest suffix");
        assert_eq!(base, "google-oauth2_1");
        assert_eq!(digest.len(), STORAGE_PATH_SEGMENT_DIGEST_CHARS);
        assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(
            training_ledger_app_dir("../escape"),
            format!(
                "training-ledgers/apps/{}",
                storage_path_segment("../escape", "app")
            )
        );
        assert!(!training_ledger_app_dir("../escape").contains(".."));
    }

    #[test]
    fn legacy_ledger_predicate_matches_only_ledger_files() {
        for path in [
            "storage/inspection/jobs.sqlite",
            "storage/inspection/jobs.sqlite-wal",
            "storage/inspection/jobs.sqlite-shm",
            "storage/inspection/jobs.sqlite-journal",
            "storage/inspection/jobs.artifacts/x.blob",
            "storage/inspection/jobs.artifacts",
            "storage/inspection/.training-locks/a.lock",
            "/storage/inspection/jobs.sqlite",
        ] {
            assert!(is_legacy_training_ledger_path(path), "{path} should match");
        }
        for path in [
            "storage/inspection/pki/cert.der",
            "storage/models/artifacts/a/weights.bin",
            "storage/inspection/jobs.sqlite.bak.d",
            "storage/inspection/jobs.sqlite-walrus",
            "storage/inspection/jobs.artifactsx/y.blob",
            "storage/inspection",
            "inspection/jobs.sqlite",
            "",
        ] {
            assert!(
                !is_legacy_training_ledger_path(path),
                "{path} should not match"
            );
        }
    }

    #[test]
    fn legacy_ledger_predicate_matches_case_variants_of_ledger_files() {
        for path in [
            "storage/inspection/Jobs.sqlite",
            "storage/inspection/JOBS.SQLITE-WAL",
            "Storage/Inspection/jobs.sqlite-Journal",
            "storage/inspection/Jobs.Artifacts/x.blob",
            "storage/inspection/JOBS.ARTIFACTS",
            "STORAGE/INSPECTION/.Training-Locks/a.lock",
            "/Storage/inspection/jobs.SQLITE-shm",
        ] {
            assert!(is_legacy_training_ledger_path(path), "{path} should match");
        }
        for path in [
            "storage/inspection/PKI/cert.der",
            "storage/inspection/JOBS.SQLITE.bak",
            "storage/inspection/Jobs.Artifactsx/y.blob",
        ] {
            assert!(
                !is_legacy_training_ledger_path(path),
                "{path} should not match"
            );
        }
    }
}
