import { CHANNEL, MAX_DOCUMENT_BYTES, validateCommand } from "./protocol.js";

const session = new URLSearchParams(location.search).get("session");
const origin = location.origin;
const queue = [];
let runtime;
let manifest;

async function verify(bytes, expected) {
	const hash = await crypto.subtle.digest("SHA-256", bytes);
	const actual = Array.from(new Uint8Array(hash), (byte) =>
		byte.toString(16).padStart(2, "0"),
	).join("");
	if (actual !== expected)
		throw new Error(
			"The document editor download failed its integrity check. Reload and try again.",
		);
}

async function loadWasm() {
	const response = await fetch("./runtime.json");
	if (!response.ok)
		throw new Error("The document editor runtime is unavailable.");
	manifest = await response.json();
	const { wasm } = manifest;
	document.title = manifest.label;
	const canvas = document.querySelector("canvas");
	if (canvas) canvas.id = manifest.canvasId || "craft_canvas";
	if (
		wasm?.compression !== "gzip" ||
		!Array.isArray(wasm.parts) ||
		!wasm.parts.length ||
		wasm.parts.length > 32 ||
		!Number.isSafeInteger(wasm.byteLength) ||
		wasm.byteLength < 1 ||
		wasm.byteLength > 512 * 1024 * 1024
	) {
		throw new Error("The document editor runtime manifest is invalid.");
	}
	if (typeof DecompressionStream === "undefined")
		throw new Error(
			"This browser does not support the document editor. Update your browser and try again.",
		);
	const parts = [];
	for (const part of wasm.parts) {
		if (
			!/^craft\.wasm\.gz\.\d{3}$/.test(part.file) ||
			!Number.isSafeInteger(part.byteLength) ||
			part.byteLength < 1 ||
			part.byteLength > 8 * 1024 * 1024
		)
			throw new Error("The document editor runtime manifest is invalid.");
		const result = await fetch(`./${part.file}`);
		if (!result.ok)
			throw new Error(
				"The document editor download failed. Reload and try again.",
			);
		const bytes = await result.arrayBuffer();
		if (bytes.byteLength !== part.byteLength)
			throw new Error("The document editor download is incomplete.");
		await verify(bytes, part.sha256);
		parts.push(bytes);
	}
	const bytes = await new Response(
		new Blob(parts).stream().pipeThrough(new DecompressionStream("gzip")),
	).arrayBuffer();
	if (bytes.byteLength !== wasm.byteLength)
		throw new Error("The document editor runtime is incomplete.");
	await verify(bytes, wasm.sha256);
	return bytes;
}

function send(type, detail, bytes) {
	if (!session || window.parent === window) return;
	const message = { ...detail, channel: CHANNEL, session, type };
	if (bytes) {
		message.bytes = Uint8Array.from(bytes).buffer;
		window.parent.postMessage(message, origin, [message.bytes]);
	} else {
		window.parent.postMessage(message, origin);
	}
}

window.flowLikeCraftEmit = (type, detail, bytes) => {
	const parsed = JSON.parse(detail);
	if (type === "ready") document.getElementById("loading")?.remove();
	if (type === "exported" && bytes.length > MAX_DOCUMENT_BYTES) {
		send("error", {
			requestId: parsed.requestId,
			error: "The exported document exceeds 200 MiB.",
		});
		return;
	}
	send(type, parsed, bytes.length ? bytes : undefined);
};
window.flowLikeCraftTake = () => queue.shift() ?? null;

window.addEventListener("message", (event) => {
	if (event.source !== window.parent || event.origin !== origin) return;
	try {
		const command = validateCommand(event.data, session, manifest);
		if (!command) return;
		// The host sends one document per iframe. Never queue replacement documents or
		// an unbounded number of exports while a large file is being laid out.
		if (queue.length >= 8)
			throw new Error("The document editor is busy. Try again.");
		queue.push(command);
		runtime?.request_repaint();
	} catch (error) {
		send("error", {
			requestId: event.data?.requestId,
			error: String(error.message ?? error),
		});
	}
});
window.addEventListener("error", (event) =>
	send("error", { error: event.message || "The document editor failed." }),
);
window.addEventListener("unhandledrejection", (event) =>
	send("error", { error: String(event.reason?.message ?? event.reason) }),
);

try {
	if (!session || session.length > 128 || window.parent === window) {
		throw new Error("Open this document from Flow-Like storage.");
	}
	runtime = await import("./craft.js");
	await runtime.default({ module_or_path: await loadWasm() });
} catch (error) {
	const message = String(error.message ?? error);
	const loading = document.getElementById("loading");
	if (loading) loading.textContent = message;
	send("error", { error: message });
}
