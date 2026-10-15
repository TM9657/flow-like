use super::{CifxClient, CifxConfig, CifxDriverMode};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command};

const LIBRARY_PATH: &str = "FLOW_LIKE_CIFX_LIBRARY_PATH";
const LIBRARY_SHA256: &str = "FLOW_LIKE_CIFX_LIBRARY_SHA256";
const PROBE_CASE: &str = "FLOW_LIKE_CIFX_TEST_CASE";
const PROBE_REQUEST: &str = "FLOW_LIKE_CIFX_TEST_REQUEST";
const PROBE_MARKER: &str = "FLOW_LIKE_CIFX_TEST_MARKER";

#[test]
fn native_initializers_require_operator_approval() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("initializer.c");
    let library = directory.path().join(if cfg!(target_os = "macos") {
        "initializer.dylib"
    } else {
        "initializer.so"
    });
    std::fs::write(
        &source,
        r#"
#include <stdio.h>
#include <stdlib.h>

__attribute__((constructor)) static void record_load(void) {
    const char *path = getenv("FLOW_LIKE_CIFX_TEST_MARKER");
    if (path == NULL) abort();
    FILE *marker = fopen(path, "w");
    if (marker == NULL) abort();
    fputs("loaded\n", marker);
    fclose(marker);
}
"#,
    )
    .unwrap();
    let output = Command::new("cc")
        .arg(if cfg!(target_os = "macos") {
            "-dynamiclib"
        } else {
            "-shared"
        })
        .args(["-fPIC", "-std=c11", "-Wall", "-Werror"])
        .arg(&source)
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Native initializer fixture compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let digest = format!("{:x}", Sha256::digest(std::fs::read(&library).unwrap()));
    let unapproved = library.with_file_name("unapproved.so");
    std::fs::copy(&library, &unapproved).unwrap();

    for case in [
        "missing_approval",
        "missing_digest",
        "wrong_digest",
        "wrong_path",
        "network_path",
        "unc_path",
        "approved",
    ] {
        let marker = directory.path().join(format!("{case}.marker"));
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "cifx::loader_tests::native_initializer_probe",
                "--nocapture",
            ])
            .env_remove(LIBRARY_PATH)
            .env_remove(LIBRARY_SHA256)
            .env(PROBE_CASE, case)
            .env(PROBE_MARKER, &marker)
            .env(PROBE_REQUEST, &library);
        if case != "missing_approval" {
            child.env(LIBRARY_PATH, &library);
        }
        if !matches!(case, "missing_approval" | "missing_digest") {
            child.env(
                LIBRARY_SHA256,
                if case == "wrong_digest" {
                    "0".repeat(64)
                } else {
                    digest.clone()
                },
            );
        }
        match case {
            "wrong_path" => {
                child.env(PROBE_REQUEST, &unapproved);
            }
            "network_path" => {
                child.env(PROBE_REQUEST, "//attacker/share/driver.so");
            }
            "unc_path" => {
                child.env(PROBE_REQUEST, r"\\attacker\share\driver.dll");
            }
            _ => {}
        }
        let output = child.output().unwrap();
        assert!(
            output.status.success(),
            "Initializer probe {case} failed.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "The native initializer probe did not run for {case}"
        );
        assert_eq!(marker.exists(), case == "approved", "{case}");
    }
}

#[tokio::test]
async fn native_initializer_probe() {
    // Each child gets a fresh process policy without mutating the test runner's environment.
    let Ok(case) = std::env::var(PROBE_CASE) else {
        return;
    };
    let marker = PathBuf::from(std::env::var_os(PROBE_MARKER).unwrap());
    let config = CifxConfig {
        library_path: std::env::var(PROBE_REQUEST).unwrap(),
        driver_mode: CifxDriverMode::SystemDriver,
        board: "cifX0".into(),
        channel: 0,
        timeout_ms: 1000,
        expected_version_prefix: None,
        start_bus: false,
    };
    let error = match CifxClient::connect(config).await {
        Ok(_) => panic!("The initializer fixture has no cifX API and cannot connect"),
        Err(error) => error.to_string(),
    };
    let expected = match case.as_str() {
        "missing_approval" => "driver loading is disabled",
        "missing_digest" => "operator-approved FLOW_LIKE_CIFX_LIBRARY_SHA256",
        "wrong_digest" => "SHA-256 does not match",
        "wrong_path" | "network_path" => "must match the operator-configured",
        "unc_path" => "absolute installed library path",
        "approved" => "cifX API symbol is unavailable",
        other => panic!("Unknown native initializer probe case: {other}"),
    };
    assert!(error.contains(expected), "{case}: {error}");
    if case == "approved" {
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "loaded\n");
    } else {
        assert!(
            !marker.exists(),
            "Unapproved library ran its initializer: {case}"
        );
    }
}
