import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import { handleMcpControl, validateMcpRequest } from "./mcp.mjs";
import { CHANNEL, validateCommand } from "./protocol.mjs";
import {
	validateCommand as wordCommand,
	CHANNEL as WORD_CHANNEL,
} from "../wordcraft/protocol.mjs";
import { blockEnd, bridgeBranch, rustFunction } from "./rust-source.mjs";

const request = (method, params = {}, extra = {}) => ({
	type: "mcp-request",
	requestId: "call-1",
	deadlineAtMs: 1000,
	message: { jsonrpc: "2.0", id: 7, method, params },
	...extra,
});

test("MCP envelopes preserve upstream arguments and reject oversized or malformed requests", () => {
	const original = request("tools/call", {
		name: "command_run",
		arguments: { params: { points: [1, 2] } },
	});
	assert.deepEqual(validateMcpRequest(original), original);
	for (const message of [
		[],
		null,
		{ method: "tools/list" },
		{ jsonrpc: "2.0", id: null, method: "tools/list" },
		{ jsonrpc: "2.0", id: 1, method: "tools/list", params: [] },
	]) {
		assert.throws(
			() => validateMcpRequest({ ...original, message }),
			/Invalid/,
		);
	}
	assert.throws(
		() =>
			validateMcpRequest(
				request("tools/call", { text: "x".repeat(1024 * 1024) }),
			),
		/exceeds/,
	);
	assert.throws(
		() =>
			validateMcpRequest(request("tools/call", { text: "é".repeat(600_000) })),
		/exceeds/,
	);
});

test("both editor protocols scope MCP and cancellation to their iframe session", () => {
	for (const [validate, channel] of [
		[validateCommand, CHANNEL],
		[wordCommand, WORD_CHANNEL],
	]) {
		const command = { ...request("tools/list"), channel, session: "live" };
		assert.deepEqual(validate(command, "live"), request("tools/list"));
		assert.equal(validate(command, "other"), null);
		assert.deepEqual(
			validate(
				{ channel, session: "live", type: "mcp-cancel", requestId: "call-1" },
				"live",
			),
			{ type: "mcp-cancel", requestId: "call-1" },
		);
	}
});

test("transport negotiates MCP, passes tools through, and rejects expired work", () => {
	const info = { name: "solidcraft", version: "0.15" };
	const init = handleMcpControl(
		request("initialize", { protocolVersion: "2025-03-26" }),
		info,
		0,
	);
	assert.equal(init.response.result.protocolVersion, "2025-03-26");
	assert.deepEqual(init.response.result.capabilities, {
		tools: { listChanged: false },
	});
	assert.equal(handleMcpControl(request("tools/list"), info, 0), undefined);
	assert.equal(
		handleMcpControl(request("tools/call"), info, 1000).response.error.code,
		-32000,
	);
	assert.equal(
		handleMcpControl(request("resources/list"), info, 0).response.error.code,
		-32601,
	);
	assert.equal(
		handleMcpControl(
			request(
				"notifications/initialized",
				{},
				{ message: { jsonrpc: "2.0", method: "notifications/initialized" } },
			),
			info,
			0,
		).response,
		null,
	);
});

test("upstream extraction preserves nested bridge logic, strings and Rust parameter types", () => {
	const source = `#[tool(description = "brace } and quoted \\\" text")]
async fn inspect(&self, Parameters(p): Parameters<DocIndex>) -> Result<Value, Error> {
    if let Some(r) = self.headless_op(move |h| { h.inspect(p) }).await { return r; }
    let v = json!({"text": "}", "raw": r#"{"#});
    /* nested { /* } */ } */
    self.bridge.call(v).await
}`;
	const item = rustFunction(source, "inspect", { attributes: true });
	assert.equal(item, source);
	const bridge = bridgeBranch(item);
	assert.ok(!bridge.includes("headless_op"));
	assert.ok(bridge.includes("self.bridge.call(v).await"));
	assert.ok(bridge.includes("Parameters<DocIndex>"));
	assert.equal(blockEnd('{"}"}', 0), 5);
	assert.throws(() => rustFunction(source, "renamed"), /missing/);
	assert.throws(() => blockEnd("{", 0), /Unclosed/);
});

test("live hosts revoke access even with a full queue and discard cancelled or expired tools", () => {
	for (const [directory, channel, validate, takeName] of [
		["craft-viewers", CHANNEL, validateCommand, "flowLikeCraftTake"],
		["wordcraft", WORD_CHANNEL, wordCommand, "flowLikeWordcraftTake"],
	]) {
		let now = 100;
		const listeners = new Map();
		const sent = [];
		const parent = { postMessage: (message) => sent.push(message) };
		const window = {
			parent,
			addEventListener: (name, callback) => listeners.set(name, callback),
		};
		const source = readFileSync(
			new URL(`../${directory}/host.js`, import.meta.url),
			"utf8",
		).replace(/^import .*;\n/gm, "");
		vm.runInNewContext(source.slice(0, source.lastIndexOf("\ntry {")), {
			window,
			location: { search: "?session=live", origin: "https://flow.test" },
			URLSearchParams,
			Date: { now: () => now },
			CHANNEL: channel,
			MAX_DOCUMENT_BYTES: 1024,
			validateCommand: validate,
			handleMcpControl: (command, info) => handleMcpControl(command, info, now),
			mcpError: (command, message, code) => ({
				requestId: command.requestId,
				response: {
					jsonrpc: "2.0",
					id: command.message.id,
					error: { message, code },
				},
			}),
		});
		const deliver = (command) =>
			listeners.get("message")({
				source: parent,
				origin: "https://flow.test",
				data: { channel, session: "live", ...command },
			});
		for (let id = 0; id < 8; id++)
			deliver(
				request(
					"tools/list",
					{},
					{
						requestId: `q${id}`,
						message: { jsonrpc: "2.0", id, method: "tools/list" },
					},
				),
			);
		deliver({ type: "set-read-only", readOnly: true });
		assert.equal(window[takeName]().readOnly, true);
		deliver({ type: "mcp-cancel", requestId: "q0" });
		deliver(
			request(
				"notifications/cancelled",
				{},
				{
					message: {
						jsonrpc: "2.0",
						method: "notifications/cancelled",
						params: { requestId: 1 },
					},
				},
			),
		);
		assert.equal(window[takeName]().requestId, "q2");
		now = 1001;
		assert.equal(window[takeName](), null);
		assert.equal(
			sent.filter((message) => message.response?.error?.code === -32000).length,
			5,
		);
		assert.equal(
			sent.some((message) => message.type === "error"),
			false,
		);
	}
});
