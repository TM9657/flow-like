/// HTTP Request Node — Demonstrates declaring HTTP permissions (C++)
///
/// This example shows how to declare the "http" permission and use the
/// raw host import to make outbound HTTP requests from a C++ WASM node.
/// Copy this pattern into your node.cpp when you need network access.

#include "flow_like_sdk.h"
using namespace flowlike;

// ============================================================================
// Node definition — note `def.add_permission("http")`
// ============================================================================

static NodeDefinition build_http_get_definition() {
    NodeDefinition def;
    def.name          = "http_get_request_cpp";
    def.friendly_name = "HTTP GET Request (C++)";
    def.description   = "Sends a GET request to a URL and reports the result";
    def.category      = "Network/HTTP";
    def.add_permission("http");

    def.add_pin(PinDefinition::input("exec", "Execute", "Trigger execution", DataType::Exec));
    def.add_pin(PinDefinition::input("url", "URL", "Target URL", DataType::String)
                    .with_default("\"https://httpbin.org/get\""));
    def.add_pin(PinDefinition::input("headers_json", "Headers (JSON)",
                                     "Request headers as JSON", DataType::String)
                    .with_default("\"{}\""));
    def.add_pin(PinDefinition::output("exec_out", "Done", "Fires after the request",
                                      DataType::Exec));
    def.add_pin(PinDefinition::output("success", "Success",
                                      "Whether the HTTP call was accepted", DataType::Bool));
    return def;
}

// ============================================================================
// Run handler — uses the raw host import directly
// ============================================================================

// Raw host import (declared in flow_like_sdk.h)
// __attribute__((import_module("flowlike_http"), import_name("request")))
// int32_t _fl_http_request(int32_t method,
//     const char* url_ptr, uint32_t url_len,
//     const char* hdr_ptr, uint32_t hdr_len,
//     const char* body_ptr, uint32_t body_len);

static ExecutionResult handle_http_get(Context& ctx) {
    std::string url     = ctx.get_string("url", "https://httpbin.org/get");
    std::string headers = ctx.get_string("headers_json", "{}");

    log::info("Sending GET request to " + url);

    // Method 0 = GET.  The host checks the "http" capability.
    int32_t result = _fl_http_request(
        0,
        url.data(),     static_cast<uint32_t>(url.size()),
        headers.data(), static_cast<uint32_t>(headers.size()),
        nullptr,        0);

    bool ok = (result != -1);

    if (ok) {
        log::info("HTTP capability granted — request dispatched");
    } else {
        log::error("HTTP capability denied — is the 'http' permission declared?");
    }

    ctx.set_output("success", ok ? "true" : "false");
    return ctx.success();
}
