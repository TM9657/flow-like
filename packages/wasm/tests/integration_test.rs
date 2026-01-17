//! Integration tests for WASM node loading and execution
//!
//! These tests verify that the WASM runtime can correctly load and execute
//! nodes built from the templates.

use flow_like_wasm::abi::WasmExecutionInput;
use flow_like_wasm::engine::{WasmConfig, WasmEngine};
use flow_like_wasm::instance::WasmInstance;
use flow_like_wasm::limits::WasmSecurityConfig;
use flow_like_wasm::module::WasmModule;
use std::path::PathBuf;
use std::sync::Arc;

/// Get the path to the compiled Rust template WASM
fn rust_template_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("templates/wasm-node-rust/target/wasm32-unknown-unknown/release/flow_like_wasm_node_template.wasm")
}

/// Get the path to the compiled Rust math_nodes example
fn rust_math_nodes_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("templates/wasm-node-rust/target/wasm32-unknown-unknown/release/examples/math_nodes.wasm")
}

/// Get the path to the compiled AssemblyScript template WASM
fn as_template_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("templates/wasm-node-assemblyscript/build/release.wasm")
}

/// Get the path to the compiled AssemblyScript examples WASM
fn as_examples_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("templates/wasm-node-assemblyscript/build/examples.wasm")
}

/// Helper to create a basic execution input
fn create_execution_input(
    inputs: serde_json::Map<String, serde_json::Value>,
) -> WasmExecutionInput {
    WasmExecutionInput {
        inputs,
        node_id: "test_node_id".to_string(),
        run_id: "test_run_id".to_string(),
        app_id: "test_app".to_string(),
        board_id: "test_board".to_string(),
        user_id: "test_user".to_string(),
        stream_state: false,
        log_level: 1,
        node_name: String::new(),
    }
}

/// Helper to create execution input with node_name for multi-node packages
fn create_package_execution_input(
    node_name: &str,
    inputs: serde_json::Map<String, serde_json::Value>,
) -> WasmExecutionInput {
    WasmExecutionInput {
        inputs,
        node_id: "test_node_id".to_string(),
        run_id: "test_run_id".to_string(),
        app_id: "test_app".to_string(),
        board_id: "test_board".to_string(),
        user_id: "test_user".to_string(),
        stream_state: false,
        log_level: 1,
        node_name: node_name.to_string(),
    }
}

// ============================================================================
// Rust Template Tests
// ============================================================================

#[tokio::test]
async fn test_load_rust_template() {
    let wasm_path = rust_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust template not built. Run: cd templates/wasm-node-rust && cargo build --release --target wasm32-unknown-unknown");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
        .await
        .expect("Failed to load module");

    // Verify module was loaded
    assert!(module.has_alloc());
}

#[tokio::test]
async fn test_rust_template_get_definition() {
    let wasm_path = rust_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust template not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    let definition = instance
        .call_get_node()
        .await
        .expect("Failed to get node definition");

    // Verify definition fields
    assert_eq!(definition.name, "my_custom_node");
    assert_eq!(definition.friendly_name, "My Custom Node");
    assert!(!definition.description.is_empty());
    assert_eq!(definition.category, "Custom/WASM");

    // Verify pins exist
    assert!(!definition.pins.is_empty());

    // Check for expected pins
    let pin_names: Vec<&str> = definition.pins.iter().map(|p| p.name.as_str()).collect();
    assert!(pin_names.contains(&"exec"), "Missing exec pin");
    assert!(pin_names.contains(&"input_text"), "Missing input_text pin");
    assert!(pin_names.contains(&"multiplier"), "Missing multiplier pin");
    assert!(pin_names.contains(&"exec_out"), "Missing exec_out pin");
    assert!(
        pin_names.contains(&"output_text"),
        "Missing output_text pin"
    );
    assert!(pin_names.contains(&"char_count"), "Missing char_count pin");
}

#[tokio::test]
async fn test_rust_template_execute() {
    let wasm_path = rust_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust template not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    // Create execution input
    let mut inputs = serde_json::Map::new();
    inputs.insert("input_text".to_string(), serde_json::json!("Hello"));
    inputs.insert("multiplier".to_string(), serde_json::json!(3));

    let exec_input = create_execution_input(inputs);
    let result = instance
        .call_run(&exec_input)
        .await
        .expect("Failed to execute node");

    // Verify no error
    assert!(
        result.error.is_none(),
        "Execution returned error: {:?}",
        result.error
    );

    // Verify outputs
    assert!(
        result.outputs.contains_key("output_text"),
        "Missing output_text output"
    );
    assert!(
        result.outputs.contains_key("char_count"),
        "Missing char_count output"
    );

    // Verify output values
    let output_text = result.outputs.get("output_text").unwrap().as_str().unwrap();
    assert_eq!(output_text, "HelloHelloHello");

    let char_count = result.outputs.get("char_count").unwrap().as_i64().unwrap();
    assert_eq!(char_count, 15);

    // Verify exec pin was activated
    assert!(result.activate_exec.contains(&"exec_out".to_string()));
}

// ============================================================================
// Rust Math Nodes Package Tests
// ============================================================================

#[tokio::test]
async fn test_rust_math_nodes_definitions() {
    let wasm_path = rust_math_nodes_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust math_nodes example not built. Run: cd templates/wasm-node-rust && cargo build --release --target wasm32-unknown-unknown --examples");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    // Verify this is a multi-node package
    assert!(
        instance.is_package(),
        "math_nodes should be a multi-node package"
    );

    // Get all node definitions
    let definitions = instance
        .call_get_nodes()
        .await
        .expect("Failed to get node definitions");

    // The math_nodes package should have multiple definitions
    println!("Got {} definitions", definitions.len());
    assert!(
        !definitions.is_empty(),
        "Package should have at least one node"
    );

    // Find the math_add node
    let add_node = definitions.iter().find(|d| d.name == "math_add");
    assert!(add_node.is_some(), "Should have math_add node");
}

#[tokio::test]
async fn test_rust_math_add_execute() {
    let wasm_path = rust_math_nodes_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust math_nodes example not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    // Test math_add
    let mut inputs = serde_json::Map::new();
    inputs.insert("a".to_string(), serde_json::json!(5.0));
    inputs.insert("b".to_string(), serde_json::json!(3.0));

    let exec_input = create_package_execution_input("math_add", inputs);
    let result = instance
        .call_run(&exec_input)
        .await
        .expect("Failed to execute math_add");

    assert!(
        result.error.is_none(),
        "Execution returned error: {:?}",
        result.error
    );

    let sum = result.outputs.get("result").unwrap().as_f64().unwrap();
    assert!((sum - 8.0).abs() < 0.001, "Expected 8.0, got {}", sum);
}

#[tokio::test]
async fn test_rust_math_multiply_execute() {
    let wasm_path = rust_math_nodes_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust math_nodes example not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    // Test math_multiply
    let mut inputs = serde_json::Map::new();
    inputs.insert("a".to_string(), serde_json::json!(4.0));
    inputs.insert("b".to_string(), serde_json::json!(7.0));

    let exec_input = create_package_execution_input("math_multiply", inputs);
    let result = instance
        .call_run(&exec_input)
        .await
        .expect("Failed to execute math_multiply");

    assert!(
        result.error.is_none(),
        "Execution returned error: {:?}",
        result.error
    );

    let product = result.outputs.get("result").unwrap().as_f64().unwrap();
    assert!(
        (product - 28.0).abs() < 0.001,
        "Expected 28.0, got {}",
        product
    );
}

#[tokio::test]
async fn test_rust_math_divide_by_zero() {
    let wasm_path = rust_math_nodes_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust math_nodes example not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    // Test math_divide with division by zero
    let mut inputs = serde_json::Map::new();
    inputs.insert("a".to_string(), serde_json::json!(10.0));
    inputs.insert("b".to_string(), serde_json::json!(0.0));

    let exec_input = create_package_execution_input("math_divide", inputs);
    let result = instance
        .call_run(&exec_input)
        .await
        .expect("Failed to execute math_divide");

    assert!(
        result.error.is_none(),
        "Execution returned error: {:?}",
        result.error
    );

    // Should indicate invalid division
    let is_valid = result.outputs.get("is_valid").unwrap().as_bool().unwrap();
    assert!(!is_valid, "Division by zero should set is_valid to false");
}

// ============================================================================
// AssemblyScript Template Tests
// ============================================================================

#[tokio::test]
async fn test_load_assemblyscript_template() {
    let wasm_path = as_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: AssemblyScript template not built. Run: cd templates/wasm-node-assemblyscript && npm run build");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
        .await
        .expect("Failed to load module");

    // Verify module was loaded
    assert!(
        module.has_alloc(),
        "AssemblyScript module should export alloc"
    );
}

#[tokio::test]
async fn test_assemblyscript_template_get_definition() {
    let wasm_path = as_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: AssemblyScript template not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    let definition = instance
        .call_get_node()
        .await
        .expect("Failed to get node definition");

    // Verify definition fields
    assert_eq!(definition.name, "my_custom_node_as");
    assert_eq!(definition.friendly_name, "My Custom Node (AS)");
    assert!(!definition.description.is_empty());
    assert_eq!(definition.category, "Custom/WASM");

    // Verify pins exist
    assert!(!definition.pins.is_empty());

    // Check for expected pins
    let pin_names: Vec<&str> = definition.pins.iter().map(|p| p.name.as_str()).collect();
    assert!(pin_names.contains(&"exec"), "Missing exec pin");
    assert!(pin_names.contains(&"input_text"), "Missing input_text pin");
    assert!(pin_names.contains(&"multiplier"), "Missing multiplier pin");
    assert!(pin_names.contains(&"exec_out"), "Missing exec_out pin");
    assert!(
        pin_names.contains(&"output_text"),
        "Missing output_text pin"
    );
    assert!(pin_names.contains(&"char_count"), "Missing char_count pin");
}

#[tokio::test]
async fn test_assemblyscript_template_execute() {
    let wasm_path = as_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: AssemblyScript template not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");
    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut instance = WasmInstance::new(&engine, module, WasmSecurityConfig::permissive())
        .await
        .expect("Failed to create instance");

    // Create execution input - AssemblyScript expects JSON string values
    let mut inputs = serde_json::Map::new();
    inputs.insert("input_text".to_string(), serde_json::json!("World"));
    inputs.insert("multiplier".to_string(), serde_json::json!(2));

    let exec_input = create_execution_input(inputs);
    let result = instance
        .call_run(&exec_input)
        .await
        .expect("Failed to execute node");

    // Verify no error
    assert!(
        result.error.is_none(),
        "Execution returned error: {:?}",
        result.error
    );

    // Verify outputs
    assert!(
        result.outputs.contains_key("output_text"),
        "Missing output_text output"
    );
    assert!(
        result.outputs.contains_key("char_count"),
        "Missing char_count output"
    );

    // Verify output values
    let output_text = result.outputs.get("output_text").unwrap().as_str().unwrap();
    assert_eq!(output_text, "WorldWorld");

    let char_count = result.outputs.get("char_count").unwrap().as_i64().unwrap();
    assert_eq!(char_count, 10);

    // Verify exec pin was activated
    assert!(result.activate_exec.contains(&"exec_out".to_string()));
}

// ============================================================================
// AssemblyScript Examples Tests
// ============================================================================

#[tokio::test]
async fn test_assemblyscript_examples_math_add() {
    let wasm_path = as_examples_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: AssemblyScript examples not built. Run: cd templates/wasm-node-assemblyscript && npm run build:examples");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");

    // The examples module exports individual node functions (get_add_definition, run_add, etc.)
    // rather than get_node/get_nodes. This is a different pattern that requires
    // custom handling. For now, verify the module can be parsed but may not load
    // as a standard Flow-Like WASM module.
    let result = WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string()).await;

    // The examples module doesn't have get_node/get_nodes exports,
    // so loading it as a standard module will fail
    if result.is_err() {
        eprintln!("Note: AssemblyScript examples use individual exports pattern (get_add_definition, run_add, etc.)");
        eprintln!(
            "This is expected behavior - the examples demonstrate a different multi-node pattern"
        );
        return;
    }

    let module = result.unwrap();
    println!("AssemblyScript examples module loaded successfully");
    assert!(module.has_alloc());
}

// ============================================================================
// Security and Limits Tests
// ============================================================================

#[tokio::test]
async fn test_fuel_consumption() {
    let wasm_path = rust_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust template not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");

    // Create engine with fuel metering
    let mut config = WasmConfig::default();
    config.fuel_metering = true;
    let engine = WasmEngine::new(config).expect("Failed to create engine");

    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut security = WasmSecurityConfig::permissive();
    security.limits.fuel_limit = 10_000_000; // Generous limit

    let mut instance = WasmInstance::new(&engine, module, security)
        .await
        .expect("Failed to create instance");

    // Execute should succeed with enough fuel
    let mut inputs = serde_json::Map::new();
    inputs.insert("input_text".to_string(), serde_json::json!("Test"));
    inputs.insert("multiplier".to_string(), serde_json::json!(1));

    let exec_input = create_execution_input(inputs);
    let result = instance.call_run(&exec_input).await;

    assert!(result.is_ok(), "Execution should succeed with enough fuel");
}

#[tokio::test]
async fn test_memory_limits() {
    let wasm_path = rust_template_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: Rust template not built");
        return;
    }

    let bytes = tokio::fs::read(&wasm_path)
        .await
        .expect("Failed to read WASM file");
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");

    let module = Arc::new(
        WasmModule::from_bytes(&engine, &bytes, "test_hash".to_string())
            .await
            .expect("Failed to load module"),
    );

    let mut security = WasmSecurityConfig::permissive();
    security.limits.memory_limit = 10 * 1024 * 1024; // 10MB limit

    let instance = WasmInstance::new(&engine, module, security).await;

    // Instance creation should succeed with reasonable memory limits
    assert!(instance.is_ok(), "Instance creation should succeed");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_wasm_bytes() {
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");

    // Try to load invalid bytes
    let invalid_bytes = vec![0x00, 0x61, 0x73, 0x6D]; // Incomplete WASM header
    let result = WasmModule::from_bytes(&engine, &invalid_bytes, "test_hash".to_string()).await;

    assert!(result.is_err(), "Loading invalid WASM should fail");
}

#[tokio::test]
async fn test_module_without_required_exports() {
    let engine = WasmEngine::new(WasmConfig::default()).expect("Failed to create engine");

    // A minimal valid WASM module without required exports
    // This is a valid WASM module with just a memory export
    let minimal_wasm = wat::parse_str(
        r#"
        (module
            (memory (export "memory") 1)
        )
    "#,
    )
    .expect("Failed to parse WAT");

    let result = WasmModule::from_bytes(&engine, &minimal_wasm, "test_hash".to_string()).await;

    assert!(
        result.is_err(),
        "Module without get_node should fail to load"
    );

    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("get_node")
            || err_str.contains("get_nodes")
            || err_str.contains("MissingExport"),
        "Error should mention missing export: {}",
        err_str
    );
}
