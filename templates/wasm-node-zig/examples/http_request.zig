/// HTTP Request Node — Demonstrates declaring HTTP permissions (Zig)
///
/// This example shows how to declare the "http" permission and use the
/// host function to make outbound HTTP requests from a Zig WASM node.
/// Copy this pattern into your main.zig when you need network access.

const std = @import("std");
const sdk = @import("flow_like_sdk");

// ============================================================================
// Node definition — note `def.addPermission("http")`
// ============================================================================

pub fn buildHttpGetDefinition() sdk.NodeDefinition {
    var def = sdk.NodeDefinition.init(sdk.allocator);
    def.name = "http_get_request_zig";
    def.friendly_name = "HTTP GET Request (Zig)";
    def.description = "Sends a GET request to a URL and reports the result";
    def.category = "Network/HTTP";
    def.addPermission("http");

    def.addPin(sdk.inputPin("exec", "Execute", "Trigger execution", .exec));
    def.addPin(sdk.inputPin("url", "URL", "Target URL", .string)
        .withDefault("\"https://httpbin.org/get\""));
    def.addPin(sdk.inputPin("headers_json", "Headers (JSON)", "Request headers as JSON", .string)
        .withDefault("\"{}\""));
    def.addPin(sdk.outputPin("exec_out", "Done", "Fires after the request", .exec));
    def.addPin(sdk.outputPin("success", "Success", "Whether the HTTP call was accepted", .bool_type));

    return def;
}

// ============================================================================
// Run handler
// ============================================================================

pub fn handleHttpGet(ctx: *sdk.Context) sdk.ExecutionResult {
    const url = ctx.getString("url") orelse "https://httpbin.org/get";
    const headers = ctx.getString("headers_json") orelse "{}";

    sdk.log.info("Sending GET request");

    // Method 0 = GET.  The host checks the "http" capability.
    const ok = sdk.httpRequest(0, url, headers, "");

    if (ok) {
        sdk.log.info("HTTP capability granted — request dispatched");
    } else {
        sdk.log.err("HTTP capability denied — is the 'http' permission declared?");
    }

    ctx.setOutput("success", if (ok) "true" else "false");
    return ctx.success();
}
