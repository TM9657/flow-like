use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub struct StagedFrontend {
    pub wasm_paths: Vec<String>,
    pub wasm_bytes: u64,
}

fn is_wasm_payload(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.ends_with(".wasm")
        || name.ends_with(".wasm.gz")
        || name.rsplit_once(".wasm.gz.").is_some_and(|(_, part)| {
            !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn files_under(root: &Path) -> io::Result<Vec<PathBuf>> {
    fn collect(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_dir() {
                collect(root, &path, files)?;
            } else if kind.is_file() {
                files.push(path.strip_prefix(root).unwrap().to_owned());
            } else {
                return Err(io::Error::other(format!(
                    "frontend assets must be regular files: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn sync_files(source: &Path, destination: &Path, files: &[PathBuf]) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for stale in files_under(destination)? {
        if files.binary_search(&stale).is_err() {
            fs::remove_file(destination.join(stale))?;
        }
    }
    for relative in files {
        let input = source.join(relative);
        let output = destination.join(relative);
        // Leave identical files untouched so Cargo and Xcode can reuse them.
        let contents = fs::read(&input)?;
        if fs::read(&output).ok().as_deref() != Some(contents.as_slice()) {
            if output.is_dir() {
                fs::remove_dir_all(&output)?;
            }
            fs::create_dir_all(output.parent().unwrap())?;
            fs::write(output, contents)?;
        }
    }
    Ok(())
}

/// Keep WASM payloads in the signed app bundle, outside the Mach-O __TEXT limit.
pub fn stage(source: &Path, embedded: &Path, resources: &Path) -> io::Result<StagedFrontend> {
    let files = files_under(source)?;
    let (wasm, frontend): (Vec<_>, Vec<_>) = files.into_iter().partition(|p| is_wasm_payload(p));
    let mut wasm_paths = Vec::with_capacity(wasm.len());
    let mut wasm_bytes = 0;
    for path in &wasm {
        let relative = path.to_str().ok_or_else(|| {
            io::Error::other(format!(
                "frontend asset path is not UTF-8: {}",
                path.display()
            ))
        })?;
        if relative.contains('\\') || relative.chars().any(char::is_control) {
            return Err(io::Error::other(format!(
                "invalid WASM asset path: {relative}"
            )));
        }
        wasm_paths.push(relative.to_owned());
        wasm_bytes += fs::metadata(source.join(path))?.len();
    }
    sync_files(source, resources, &wasm)?;
    sync_files(source, embedded, &frontend)?;
    Ok(StagedFrontend {
        wasm_paths,
        wasm_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "flow-like-ios-frontend-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("out/craft/pdfcraft")).unwrap();
            Self(root)
        }
        fn stage(&self) -> StagedFrontend {
            stage(
                &self.0.join("out"),
                &self.0.join("embedded"),
                &self.0.join("resources"),
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn separates_raw_and_compressed_wasm_without_changing_urls_or_bytes() {
        let fixture = Fixture::new();
        for (path, bytes) in [
            ("index.html", b"<html>app</html>".as_slice()),
            ("craft/pdfcraft/host.js", b"fetch('./craft.wasm.gz.000')"),
            ("craft/pdfcraft/craft.wasm.gz.000", b"gzip chunk"),
            ("craft/pdfcraft/craft.wasm.gz.001", b"second chunk"),
            ("module.wasm", b"\0asm"),
            ("module.wasm.gz", b"compressed wasm"),
            ("module.wasm.map", b"source map"),
        ] {
            fs::write(fixture.0.join("out").join(path), bytes).unwrap();
        }
        let staged = fixture.stage();
        assert_eq!(
            staged.wasm_paths,
            [
                "craft/pdfcraft/craft.wasm.gz.000",
                "craft/pdfcraft/craft.wasm.gz.001",
                "module.wasm",
                "module.wasm.gz",
            ]
        );
        assert_eq!(staged.wasm_bytes, 41);
        for path in staged.wasm_paths {
            assert!(!fixture.0.join("embedded").join(&path).exists());
            assert_eq!(
                fs::read(fixture.0.join("resources").join(&path)).unwrap(),
                fs::read(fixture.0.join("out").join(path)).unwrap()
            );
        }
        assert!(fixture.0.join("embedded/craft/pdfcraft/host.js").exists());
        assert!(fixture.0.join("embedded/module.wasm.map").exists());
        assert!(fixture.0.join("out/module.wasm").exists());
    }

    #[test]
    fn refreshes_changed_assets_removes_stale_files_and_restores_deleted_resources() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("out/module.wasm"), b"old").unwrap();
        fs::write(fixture.0.join("out/old.html"), b"old").unwrap();
        fs::create_dir_all(fixture.0.join("out/route")).unwrap();
        fs::write(fixture.0.join("out/route/index.html"), b"old route").unwrap();
        fixture.stage();
        fs::remove_file(fixture.0.join("out/old.html")).unwrap();
        fs::remove_dir_all(fixture.0.join("out/route")).unwrap();
        fs::write(fixture.0.join("out/route"), b"new route").unwrap();
        fs::write(fixture.0.join("out/module.wasm"), b"new").unwrap();
        fs::write(fixture.0.join("resources/obsolete.wasm"), b"stale").unwrap();
        fixture.stage();
        assert_eq!(
            fs::read(fixture.0.join("resources/module.wasm")).unwrap(),
            b"new"
        );
        assert!(!fixture.0.join("resources/obsolete.wasm").exists());
        assert!(!fixture.0.join("embedded/old.html").exists());
        assert_eq!(
            fs::read(fixture.0.join("embedded/route")).unwrap(),
            b"new route"
        );
        let modified = fs::metadata(fixture.0.join("embedded"))
            .unwrap()
            .modified()
            .unwrap();
        fixture.stage();
        assert_eq!(
            fs::metadata(fixture.0.join("embedded"))
                .unwrap()
                .modified()
                .unwrap(),
            modified
        );
        fs::remove_dir_all(fixture.0.join("resources")).unwrap();
        fixture.stage();
        assert!(fixture.0.join("resources/module.wasm").exists());
    }
}
