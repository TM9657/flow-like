// HTTP Request Node - Demonstrates declaring HTTP permissions
//
// This example shows how to declare the "http" permission so the runtime
// grants your WASM node access to make outbound HTTP requests.
// Copy this pattern into your main.go when you need network access.

package main

import (
	"encoding/json"

	sdk "github.com/TM9657/flow-like/libs/wasm-sdk/wasm-sdk-go"
)

// buildHTTPGetDefinition creates the node definition with the "http" permission.
func buildHTTPGetDefinition() sdk.NodeDefinition {
	def := sdk.NewNodeDefinition()
	def.Name = "http_get_request_go"
	def.FriendlyName = "HTTP GET Request (Go)"
	def.Description = "Sends a GET request to a URL and reports the result"
	def.Category = "Network/HTTP"
	def.AddPermission("http")

	def.AddPin(sdk.InputPin("exec", "Execute", "Trigger execution", "Exec"))
	def.AddPin(sdk.InputPin("url", "URL", "Target URL", "String").
		WithDefault(`"https://httpbin.org/get"`))
	def.AddPin(sdk.InputPin("headers_json", "Headers (JSON)", "Request headers as JSON", "String").
		WithDefault(`"{}"`))
	def.AddPin(sdk.OutputPin("exec_out", "Done", "Fires after the request", "Exec"))
	def.AddPin(sdk.OutputPin("success", "Success", "Whether the HTTP call was accepted", "Bool"))

	return def
}

// runHTTPGet executes the HTTP GET request.
func runHTTPGet(ctx *sdk.Context) sdk.ExecutionResult {
	url := ctx.GetString("url", "https://httpbin.org/get")
	headers := ctx.GetString("headers_json", "{}")

	ctx.Info("Sending GET request to " + url)

	// Method 0 = GET.  The host checks the "http" capability before
	// executing the request.
	ok := sdk.HTTPRequest(0, url, headers, "")

	if ok {
		ctx.Info("HTTP capability granted — request dispatched")
	} else {
		ctx.Error("HTTP capability denied — is the 'http' permission declared?")
	}

	ctx.SetOutput("success", func() string {
		if ok {
			return "true"
		}
		return "false"
	}())
	return ctx.Success()
}

// Example: serialise the definition (for reference — wire this up in your main get_node/run exports).
func exampleSerialize() int64 {
	def := buildHTTPGetDefinition()
	b, _ := json.Marshal(def)
	return sdk.PackString(string(b))
}
