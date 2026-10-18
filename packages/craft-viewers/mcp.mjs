const MAX_REQUEST_BYTES = 1024 * 1024;
const VERSIONS = ["2025-06-18", "2025-03-26", "2024-11-05"];

export function validateMcpRequest(data) {
	const { requestId, message, deadlineAtMs } = data;
	if (
		typeof requestId !== "string" ||
		!requestId ||
		requestId.length > 128 ||
		!Number.isSafeInteger(deadlineAtMs) ||
		!message ||
		Array.isArray(message) ||
		message.jsonrpc !== "2.0" ||
		typeof message.method !== "string" ||
		(message.id !== undefined &&
			typeof message.id !== "string" &&
			!Number.isSafeInteger(message.id)) ||
		(message.params !== undefined &&
			(!message.params ||
				typeof message.params !== "object" ||
				Array.isArray(message.params)))
	) {
		throw new Error("Invalid editor MCP request.");
	}
	if (
		new TextEncoder().encode(JSON.stringify(message)).byteLength >
		MAX_REQUEST_BYTES
	)
		throw new Error("The editor MCP request exceeds 1 MiB.");
	return { type: "mcp-request", requestId, message, deadlineAtMs };
}

export function mcpError(command, message, code = -32603) {
	return {
		requestId: command.requestId,
		response:
			command.message?.id === undefined
				? null
				: {
						jsonrpc: "2.0",
						id: command.message.id,
						error: { code, message },
					},
	};
}

// Transport negotiation stays outside each upstream editor's tool implementation.
export function handleMcpControl(
	command,
	{ name, version = "1" },
	now = Date.now(),
) {
	if (command.deadlineAtMs <= now)
		return mcpError(command, "The editor MCP request expired.", -32000);
	const { message } = command;
	if (message.id === undefined)
		return { requestId: command.requestId, response: null };
	let result;
	switch (message.method) {
		case "initialize":
			result = {
				protocolVersion: VERSIONS.includes(message.params?.protocolVersion)
					? message.params.protocolVersion
					: VERSIONS[0],
				capabilities: { tools: { listChanged: false } },
				serverInfo: { name, version },
				instructions:
					"Tools control the currently open Flow-Like editor document. Use the document's Save controls to persist edits.",
			};
			break;
		case "ping":
			result = {};
			break;
		case "tools/list":
		case "tools/call":
			return undefined;
		default:
			return mcpError(
				command,
				`Unsupported editor MCP method: ${message.method}`,
				-32601,
			);
	}
	return {
		requestId: command.requestId,
		response: { jsonrpc: "2.0", id: message.id, result },
	};
}
