use super::*;

#[test]
fn extra_bin_dirs_contains_common_locations() {
    let dirs = extra_bin_dirs();
    assert!(!dirs.is_empty(), "extra_bin_dirs should not be empty");

    let home = dirs_next::home_dir().expect("test requires a home directory");
    assert!(dirs.contains(&home.join(".local/bin")));
    assert!(dirs.contains(&home.join(".asdf/shims")));

    let paths_str: Vec<String> = dirs.iter().map(|d| d.display().to_string()).collect();
    let has_homebrew = paths_str.iter().any(|p| p.contains("homebrew"));
    let has_usr_local = paths_str.iter().any(|p| p.contains("/usr/local/bin"));
    assert!(
        has_homebrew || has_usr_local,
        "Should include /opt/homebrew/bin or /usr/local/bin. Got: {:?}",
        paths_str
    );

    #[cfg(target_os = "linux")]
    assert!(dirs.contains(&PathBuf::from("/home/linuxbrew/.linuxbrew/bin")));
}

#[test]
fn augmented_path_includes_existing_dirs() {
    let path = augmented_path();
    assert!(!path.is_empty(), "augmented_path should not be empty");
    // Must contain original PATH
    let current = std::env::var("PATH").unwrap_or_default();
    assert!(
        path.contains(&current),
        "augmented PATH should contain original PATH"
    );
}

#[test]
fn executable_lookup_searches_supplied_path() -> std::io::Result<()> {
    let temp_dir = std::env::temp_dir().join(format!(
        "flowpilot-executable-lookup-test-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let executable_name = if cfg!(windows) { "codex.exe" } else { "codex" };
    let executable = temp_dir.join(executable_name);
    std::fs::write(&executable, b"#!/bin/sh\nexit 0\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
    }

    let path_value = std::env::join_paths([temp_dir.as_path()])
        .expect("test path should join")
        .to_string_lossy()
        .into_owned();

    assert_eq!(
        find_executable_in_path("codex", &path_value).as_deref(),
        Some(executable.as_path())
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn codex_ide_extension_candidate_dirs_find_extension_bundled_binary() -> std::io::Result<()> {
    let temp_home = std::env::temp_dir().join(format!(
        "flowpilot-codex-extension-test-{}",
        uuid::Uuid::new_v4()
    ));
    let codex_dir = temp_home.join(".vscode/extensions/openai.chatgpt-test/bin/macos-aarch64");
    std::fs::create_dir_all(codex_dir.join("codex-path"))?;
    let executable_name = if cfg!(windows) { "codex.exe" } else { "codex" };
    let executable = codex_dir.join(executable_name);
    std::fs::write(&executable, b"#!/bin/sh\nexit 0\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
    }

    let dirs = codex_ide_extension_candidate_dirs(&temp_home);
    assert!(
        dirs.iter().any(|dir| dir == &codex_dir),
        "expected extension binary directory in candidates: {:?}",
        dirs
    );
    assert!(
        dirs.iter().any(|dir| dir == &codex_dir.join("codex-path")),
        "expected bundled Codex PATH helper directory in candidates: {:?}",
        dirs
    );

    let _ = std::fs::remove_dir_all(&temp_home);
    Ok(())
}

#[test]
fn claude_ide_extension_binaries_prefers_newest_version() -> std::io::Result<()> {
    let temp_home = std::env::temp_dir().join(format!(
        "flowpilot-claude-ext-test-{}",
        uuid::Uuid::new_v4()
    ));
    let binary_name = if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    };
    let make = |version: &str| -> std::io::Result<PathBuf> {
        let dir = temp_home
            .join(".vscode/extensions")
            .join(format!("anthropic.claude-code-{version}-darwin-arm64"))
            .join("resources/native-binary");
        std::fs::create_dir_all(&dir)?;
        let executable = dir.join(binary_name);
        std::fs::write(&executable, b"#!/bin/sh\nexit 0\n")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(executable)
    };
    let _older = make("2.1.9")?;
    let newest = make("2.1.204")?;
    // An unrelated extension must not be mistaken for the Claude Code CLI.
    std::fs::create_dir_all(temp_home.join(".vscode/extensions/some.other-ext"))?;

    let binaries = claude_ide_extension_binaries(&temp_home);
    assert_eq!(
        binaries.first(),
        Some(&newest),
        "newest extension version must resolve first: {:?}",
        binaries
    );

    let _ = std::fs::remove_dir_all(&temp_home);
    Ok(())
}

#[test]
fn codex_npm_native_package_layout_resolves_like_official_sdk() -> std::io::Result<()> {
    let Some((target, platform_package)) = codex_target() else {
        return Ok(());
    };
    let temp_root = std::env::temp_dir().join(format!(
        "flowpilot-codex-npm-package-test-{}",
        uuid::Uuid::new_v4()
    ));
    let vendor_target = temp_root
        .join("node_modules")
        .join(platform_package)
        .join("vendor")
        .join(target);
    let bin_dir = vendor_target.join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    std::fs::create_dir_all(vendor_target.join("codex-path"))?;
    std::fs::write(vendor_target.join("codex-package.json"), b"{}")?;
    let executable = bin_dir.join(codex_binary_name());
    std::fs::write(&executable, b"#!/bin/sh\nexit 0\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
    }

    let resolution = find_codex_packaged_cli_under_root(
        &temp_root.join("node_modules"),
        CliResolutionSource::CodexNpmPackage,
    )
    .expect("official @openai/codex native package layout should resolve");
    assert_eq!(resolution.executable, executable);
    assert_eq!(resolution.source, CliResolutionSource::CodexNpmPackage);
    assert!(
        resolution
            .path_dirs
            .iter()
            .any(|dir| dir == &vendor_target.join("codex-path")),
        "expected codex-path helper dir in resolution: {:?}",
        resolution.path_dirs
    );

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[test]
fn augmented_path_has_node_accessible() {
    let path = augmented_path();
    let found_node = path.split(':').any(|dir| {
        let candidate = std::path::Path::new(dir).join("node");
        candidate.exists()
    });
    assert!(
        found_node,
        "augmented PATH should include a directory containing `node`. PATH = {}",
        path
    );
}

#[test]
fn find_copilot_cli_resolves() {
    let cli_path = find_copilot_cli_path();
    assert!(
        cli_path.is_some(),
        "find_copilot_cli_path() returned None — the `copilot` CLI binary is not installed or not on PATH. \
             Searched in: {:?}",
        extra_bin_dirs()
            .iter()
            .filter(|d| d.exists())
            .collect::<Vec<_>>()
    );
    if let Some(ref p) = cli_path {
        assert!(
            p.exists(),
            "resolved copilot CLI path does not exist: {:?}",
            p
        );
    }
}
