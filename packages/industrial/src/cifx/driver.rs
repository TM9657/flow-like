use crate::{Error, Result, require};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    sync::LazyLock,
};

// This policy is process configuration, never a deserializable workflow input.
#[derive(Clone)]
pub(super) struct DriverPolicy {
    configured_path: PathBuf,
    installation_directory: PathBuf,
    library_path: PathBuf,
    sha256: [u8; 32],
}

#[derive(Debug)]
pub(super) struct VerifiedDriver {
    pub path: PathBuf,
    // On Windows, keep the file open without write/delete sharing through the load.
    _file: File,
}

impl DriverPolicy {
    pub(super) fn from_environment() -> Result<Self> {
        static POLICY: LazyLock<std::result::Result<DriverPolicy, String>> = LazyLock::new(|| {
            DriverPolicy::from_settings(
                std::env::var_os("FLOW_LIKE_CIFX_LIBRARY_PATH").map(PathBuf::from),
                std::env::var("FLOW_LIKE_CIFX_LIBRARY_SHA256").ok(),
            )
            .map_err(|error| error.to_string())
        });
        POLICY
            .as_ref()
            .cloned()
            .map_err(|error| Error::Invalid(error.clone()))
    }

    pub(super) fn from_settings(path: Option<PathBuf>, digest: Option<String>) -> Result<Self> {
        let path = path.ok_or_else(|| Error::Invalid(
            "cifX driver loading is disabled; the operator must set FLOW_LIKE_CIFX_LIBRARY_PATH and FLOW_LIKE_CIFX_LIBRARY_SHA256 before starting the process".into(),
        ))?;
        let digest = digest.ok_or_else(|| {
            Error::Invalid(
                "cifX driver loading requires an operator-approved FLOW_LIKE_CIFX_LIBRARY_SHA256"
                    .into(),
            )
        })?;
        require(
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "FLOW_LIKE_CIFX_LIBRARY_SHA256 must contain exactly 64 hexadecimal characters",
        )?;
        let mut sha256 = [0; 32];
        for (index, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16)
                .map_err(|_| Error::Invalid("Invalid cifX library SHA-256".into()))?;
        }
        validate_local_path(&path)?;
        let installation_directory = path
            .parent()
            .ok_or_else(|| {
                Error::Invalid("cifX library requires an installation directory".into())
            })?
            .canonicalize()?;
        validate_local_path(&installation_directory)?;
        let library_path = path.canonicalize()?;
        validate_local_path(&library_path)?;
        require(
            library_path.starts_with(&installation_directory),
            "cifX library must remain within its operator-configured installation directory",
        )?;
        Ok(Self {
            configured_path: path,
            installation_directory,
            library_path,
            sha256,
        })
    }

    pub(super) fn verify(&self, requested_path: &str) -> Result<VerifiedDriver> {
        // Reject workflow-selected paths before resolving or opening them, including remote paths.
        require(
            Path::new(requested_path).as_os_str() == self.configured_path.as_os_str(),
            "cifX library_path must match the operator-configured FLOW_LIKE_CIFX_LIBRARY_PATH",
        )?;
        validate_local_path(&self.configured_path)?;
        let path = self.configured_path.canonicalize()?;
        validate_local_path(&path)?;
        require(
            path == self.library_path && path.starts_with(&self.installation_directory),
            "The approved cifX library path changed; restart after operator review",
        )?;
        require(path.is_file(), "cifX library path must point to a file")?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ);
        }
        let mut file = options.open(&path)?;
        require(
            file.metadata()?.is_file(),
            "cifX library must be a regular file",
        )?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        loop {
            let length = file.read(&mut buffer)?;
            if length == 0 {
                break;
            }
            hash.update(&buffer[..length]);
        }
        let actual_sha256: [u8; 32] = hash.finalize().into();
        require(
            actual_sha256 == self.sha256,
            "cifX library SHA-256 does not match the operator-approved digest",
        )?;
        // The operator must protect this installation directory and native dependencies from
        // untrusted writes. A digest cannot authenticate dependencies or prevent Unix path races.
        Ok(VerifiedDriver { path, _file: file })
    }
}

fn validate_local_path(path: &Path) -> Result<()> {
    let bytes = path.as_os_str().as_encoded_bytes();
    require(
        path.is_absolute() && bytes.len() <= 4096 && !bytes.contains(&0),
        "cifX requires an absolute local installed library path without NUL",
    )?;
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let drive = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => return Err(Error::Invalid("cifX rejects UNC and device paths".into())),
            },
            _ => return Err(Error::Invalid("cifX requires a local drive path".into())),
        };
        let root = [u16::from(drive), b':' as u16, b'\\' as u16, 0];
        // DRIVE_FIXED excludes mapped network drives as well as removable media.
        require(
            unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) } == 3,
            "cifX requires an installed library on a local fixed drive",
        )?;
    }
    #[cfg(not(windows))]
    require(
        !bytes.starts_with(b"//") && !bytes.starts_with(b"\\\\"),
        "cifX rejects network and device path syntax",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_requires_both_operator_settings_and_a_valid_digest() {
        assert!(DriverPolicy::from_settings(None, None).is_err());
        assert!(DriverPolicy::from_settings(None, Some("0".repeat(64))).is_err());
        let path = PathBuf::from("driver");
        assert!(DriverPolicy::from_settings(Some(path.clone()), None).is_err());
        for hash in [
            String::new(),
            "0".repeat(63),
            "z".repeat(64),
            "é".repeat(32),
        ] {
            assert!(DriverPolicy::from_settings(Some(path.clone()), Some(hash)).is_err());
        }
    }

    #[test]
    fn network_device_relative_and_nul_paths_are_rejected() {
        for path in [
            "driver.dll",
            "C:driver.dll",
            "/tmp/driver\0.so",
            "//attacker/share/driver.dll",
            r"\\attacker\share\driver.dll",
            r"\\?\UNC\attacker\share\driver.dll",
            r"\\.\pipe\driver.dll",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1\driver.dll",
        ] {
            assert!(validate_local_path(Path::new(path)).is_err(), "{path:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn policy_pins_the_file_digest_and_installation_directory() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let installed = directory.path().join("installed");
        std::fs::create_dir(&installed).unwrap();
        let path = installed.join("driver.so");
        std::fs::write(&path, b"approved driver").unwrap();
        let digest = format!("{:x}", Sha256::digest(b"approved driver"));
        let policy = DriverPolicy::from_settings(Some(path.clone()), Some(digest.clone())).unwrap();
        assert!(policy.verify(path.to_str().unwrap()).is_ok());
        let alias = installed.join(".").join("driver.so");
        assert!(policy.verify(alias.to_str().unwrap()).is_err());
        assert!(
            policy
                .verify("//attacker/share/driver.so")
                .unwrap_err()
                .to_string()
                .contains("must match")
        );

        let outside = directory.path().join("other.so");
        std::fs::write(&outside, b"approved driver").unwrap();
        assert!(policy.verify(outside.to_str().unwrap()).is_err());
        std::fs::write(&path, b"replaced driver").unwrap();
        assert!(
            policy
                .verify(path.to_str().unwrap())
                .unwrap_err()
                .to_string()
                .contains("SHA-256")
        );
        std::fs::remove_file(&path).unwrap();
        symlink(&outside, &path).unwrap();
        assert!(policy.verify(path.to_str().unwrap()).is_err());
        assert!(DriverPolicy::from_settings(Some(path.clone()), Some(digest.clone())).is_err());

        let sibling = installed.join("driver-v2.so");
        std::fs::write(&sibling, b"approved driver").unwrap();
        std::fs::remove_file(&path).unwrap();
        symlink(&sibling, &path).unwrap();
        let policy = DriverPolicy::from_settings(Some(path.clone()), Some(digest)).unwrap();
        assert!(policy.verify(path.to_str().unwrap()).is_ok());
        let another = installed.join("driver-v3.so");
        std::fs::write(&another, b"approved driver").unwrap();
        std::fs::remove_file(&path).unwrap();
        symlink(&another, &path).unwrap();
        assert!(policy.verify(path.to_str().unwrap()).is_err());
    }
}
