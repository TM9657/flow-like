//! HTTP Request Node - Demonstrates declaring HTTP permissions
//!
//! This example shows how to declare the `http` permission so the
//! runtime grants your WASM node access to make outbound HTTP requests.
//! The `http_request` host function verifies the capability and
//! initiates the request through the sandbox.

use flow_like_wasm_sdk::*;

// ============================================================================
// Node Definition — note the `permissions: ["http"]` field
// ============================================================================

node! {
    name: "http_get_request",
    friendly_name: "HTTP GET Request",
    description: "Sends a GET request to a URL and reports the result",
    category: "Network/HTTP",
    permissions: ["http"],
    inputs: {
        exec: Exec,
        url: String = "https://httpbin.org/get",
        headers_json: String = "{}",
    },
    outputs: {
        exec_out: Exec,
        success: Bool,
    },
}

run_node!(run);

fn run(mut ctx: Context) -> ExecutionResult {
    let url = ctx.get_string("url");
    let headers = ctx.get_string("headers_json");

    ctx.info(&format!("Sending GET request to {}", url));

    // Method 0 = GET.  The host checks the `http` capability before
    // executing the request.  If the permission was not declared the
    // runtime would have blocked this node from being placed.
    let ok = http::http_request(0, &url, &headers, &[]);

    if ok {
        ctx.info("HTTP capability granted — request dispatched");
    } else {
        ctx.error("HTTP capability denied — is the 'http' permission declared?");
    }

    ctx.set_output("success", ok);
    ctx.success()
}
