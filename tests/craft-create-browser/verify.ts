import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import puppeteer from "puppeteer";
import {
	CRAFT_EDITOR_IDS,
	craftCreationFilename,
	craftCreationViewer,
} from "../../packages/ui/components/storage/craft-viewers";

interface HostEvent {
	type: string;
	requestId?: string;
	error?: string;
	dirty?: boolean;
	readOnly?: boolean;
	revision?: number;
	bytes?: ArrayBuffer;
	response?: {
		id?: string;
		error?: unknown;
		result?: Record<string, unknown>;
	};
}
declare global {
	interface Window {
		craftCreateEvents: HostEvent[];
		craftCreateFrame: HTMLIFrameElement;
	}
}

const directory = await mkdtemp(join(tmpdir(), "flow-like-craft-create-"));
const assets = resolve(import.meta.dir, "../../apps/web/public");
const nestedPreview = process.env.CRAFT_NESTED_PREVIEW === "1";
const runtimes =
	process.argv.length > 2 ? process.argv.slice(2) : CRAFT_EDITOR_IDS;
const server = Bun.serve({
	hostname: "127.0.0.1",
	port: 0,
	async fetch(request) {
		const path = new URL(request.url).pathname;
		if (path === "/")
			return new Response(
				'<!doctype html><html><link rel="icon" href="data:,"><style>html,body,iframe{width:100%;height:100%;margin:0;border:0}</style><script>window.craftCreateEvents=[];</script><body></body></html>',
				{ headers: { "Content-Type": "text/html" } },
			);
		const file = Bun.file(resolve(assets, `.${path}`));
		return (await file.exists())
			? new Response(file)
			: new Response("Not found", { status: 404 });
	},
});
const browser = await puppeteer.launch({
	headless: true,
	args: ["--enable-unsafe-swiftshader"],
	...(process.env.CHROME_PATH
		? { executablePath: process.env.CHROME_PATH }
		: {}),
});
let failed = false;
try {
	for (const runtime of runtimes) {
		const filename = craftCreationFilename(runtime);
		const viewer = craftCreationViewer(runtime);
		assert.ok(filename && viewer, `Unsupported editor ${runtime}`);
		const page = await browser.newPage();
		await page.setViewport({ width: 1280, height: 900 });
		const errors: string[] = [];
		page.on("pageerror", (error) => errors.push(String(error)));
		await page.setRequestInterception(true);
		page.on("request", (request) => {
			if (
				request.url().startsWith(server.url.origin) ||
				/^(blob|data):/.test(request.url())
			)
				void request.continue();
			else {
				errors.push(`External request ${request.url()}`);
				void request.abort();
			}
		});
		const channel =
			runtime === "wordcraft" ? "flow-like-wordcraft" : "flow-like-craft";
		const send = (data: Record<string, unknown>) =>
			page.evaluate(
				({ data, channel }) => {
					window.craftCreateFrame.contentWindow?.postMessage(
						{ ...data, channel, session: "create-test" },
						location.origin,
					);
				},
				{ data, channel },
			);
		const eventCount = () =>
			page.evaluate(() => window.craftCreateEvents.length);
		const wait = async (
			type: string,
			requestId?: string,
			start = 0,
			dirty?: boolean,
		) => {
			await page.waitForFunction(
				(type, requestId, start, dirty) =>
					window.craftCreateEvents
						.slice(start)
						.some(
							(item) =>
								item.type === "error" ||
								(item.type === type &&
									(!requestId || item.requestId === requestId) &&
									(dirty === undefined || item.dirty === dirty)),
						),
				{ timeout: type === "ready" ? 180_000 : 30_000 },
				type,
				requestId,
				start,
				dirty,
			);
			const result = await page.evaluate(
				(type, requestId, start, dirty) => {
					const found = window.craftCreateEvents
						.slice(start)
						.find(
							(item) =>
								item.type === "error" ||
								(item.type === type &&
									(!requestId || item.requestId === requestId) &&
									(dirty === undefined || item.dirty === dirty)),
						);
					if (!found) throw new Error("Missing host response");
					return {
						...found,
						bytes: found.bytes
							? Array.from(new Uint8Array(found.bytes))
							: undefined,
					};
				},
				type,
				requestId,
				start,
				dirty,
			);
			assert.notEqual(result.type, "error", `${runtime}: ${result.error}`);
			return result;
		};
		let nextId = 0;
		const rpc = async (method: string, params = {}) => {
			const id = `create-rpc-${++nextId}`;
			await send({
				type: "mcp-request",
				requestId: id,
				deadlineAtMs: Date.now() + 30_000,
				message: { jsonrpc: "2.0", id, method, params },
			});
			const response = (await wait("mcp-response", id)).response;
			assert.equal(response?.id, id);
			assert.equal(response.error, undefined, JSON.stringify(response.error));
			assert.ok(response.result);
			assert.ok(!response.result.isError, JSON.stringify(response.result));
			return response.result;
		};
		const inspect = async () => {
			await rpc("initialize", {
				protocolVersion: "2025-06-18",
				capabilities: {},
				clientInfo: { name: "blank-document-test", version: "1" },
			});
			const tools = (await rpc("tools/list")).tools as { name: string }[];
			assert.ok(tools.length > 0, `${runtime} must expose original tools`);
			const name = [
				"doc_inspect",
				"inspect_workbook",
				"inspect_presentation",
				"inspect_document",
				"document_inspect",
				"inspect_design",
				"inspect_drawing",
				"drawing_inspect",
				"inspect_session",
				"list_commands",
				"command_list",
			].find((name) => tools.some((tool) => tool.name === name));
			assert.ok(name, `${runtime} must expose document inspection`);
			const result = await rpc("tools/call", { name, arguments: {} });
			assert.ok(Array.isArray(result.content));
			return tools.length;
		};
		const mount = async () => {
			await page.goto(server.url.origin);
			await page.evaluate(
				async ({ path, nestedPreview }) => {
					let hostDocument = document;
					if (nestedPreview) {
						const preview = document.createElement("iframe");
						await new Promise<void>((resolve) => {
							preview.addEventListener("load", () => resolve(), { once: true });
							document.body.append(preview);
						});
						if (!preview.contentDocument)
							throw new Error("Missing preview document");
						hostDocument = preview.contentDocument;
						const style = hostDocument.createElement("style");
						style.textContent =
							"html,body,iframe{width:100%;height:100%;margin:0;border:0}";
						hostDocument.head.append(style);
					}
					const frame = hostDocument.createElement("iframe");
					window.craftCreateFrame = frame;
					const hostWindow = hostDocument.defaultView;
					if (!hostWindow) throw new Error("Missing preview window");
					hostWindow.addEventListener("message", (event) => {
						if (
							event.origin === window.location.origin &&
							event.source === frame.contentWindow &&
							event.data.session === "create-test"
						)
							window.craftCreateEvents.push(event.data);
					});
					frame.src = path;
					hostDocument.body.append(frame);
				},
				{
					path:
						runtime === "wordcraft"
							? "/wordcraft/index.html?session=create-test"
							: `/craft/${runtime}/index.html?session=create-test`,
					nestedPreview,
				},
			);
			await wait("ready");
		};
		const snapshot = async (download: boolean) => {
			const requestId = `create-export-${++nextId}`;
			await send({
				type: "export",
				requestId,
				format: viewer.outputExtension,
				...(download ? { purpose: "download" } : {}),
			});
			const result = await wait("exported", requestId);
			assert.ok(result.bytes?.length, `${runtime} exported an empty file`);
			assert.ok(Number.isSafeInteger(result.revision));
			return result;
		};
		try {
			await mount();
			await send({ type: "create", name: filename, readOnly: false });
			assert.equal((await wait("opened")).readOnly, false);
			await wait("dirty", undefined, 0, true);
			const tools = await inspect();
			await send({ type: "set-read-only", readOnly: true });
			const downloaded = await snapshot(true);
			await send({ type: "saved", revision: downloaded.revision });
			// A completed RPC confirms the preceding acknowledgment was processed.
			await rpc("tools/list");
			assert.equal(
				await page.evaluate(
					() =>
						window.craftCreateEvents.findLast((event) => event.type === "dirty")
							?.dirty,
				),
				true,
				"A download must not authorize a saved acknowledgment",
			);
			await send({ type: "set-read-only", readOnly: false });
			const saved = await snapshot(false);
			assert.ok(saved.bytes);
			const beforeSaved = await eventCount();
			await send({ type: "saved", revision: saved.revision });
			await wait("dirty", undefined, beforeSaved, false);
			await writeFile(
				join(directory, `${runtime}.${viewer.outputExtension}`),
				new Uint8Array(saved.bytes),
			);
			if (process.env.CRAFT_KEEP_SCREENSHOTS)
				await page.screenshot({
					path: join(directory, `${runtime}-created.png`),
				});
			await mount();
			await page.evaluate(
				({ bytes: data, filename, channel }) => {
					const bytes = new Uint8Array(data).buffer;
					window.craftCreateFrame.contentWindow?.postMessage(
						{
							type: "open",
							channel,
							session: "create-test",
							name: filename,
							readOnly: true,
							bytes,
						},
						location.origin,
						[bytes],
					);
				},
				{ bytes: saved.bytes, filename, channel },
			);
			assert.equal((await wait("opened")).readOnly, true);
			assert.equal(await inspect(), tools);
			assert.deepEqual(errors, []);
			console.log(
				`${runtime}${nestedPreview ? " (nested preview)" : ""}: blank create, ${tools} MCP tools, read-only download, save acknowledgment and .${viewer.outputExtension} reopen verified`,
			);
		} catch (error) {
			await page.screenshot({
				path: join(directory, `${runtime}-failure.png`),
			});
			await writeFile(
				join(directory, `${runtime}-events.json`),
				JSON.stringify(
					await page.evaluate(() => window.craftCreateEvents),
					null,
					2,
				),
			);
			throw error;
		} finally {
			let timer: ReturnType<typeof setTimeout> | undefined;
			await Promise.race([
				page.close(),
				new Promise<void>((resolve) => {
					timer = setTimeout(resolve, 1000);
				}),
			]);
			clearTimeout(timer);
		}
	}
} catch (error) {
	failed = true;
	throw error;
} finally {
	const cleanup = setTimeout(() => {
		browser.process()?.kill("SIGKILL");
		browser.disconnect();
	}, 10_000);
	try {
		await browser.close();
	} finally {
		clearTimeout(cleanup);
	}
	server.stop(true);
	if (failed || process.env.CRAFT_KEEP_SCREENSHOTS)
		console.log(`Creation browser artifacts: ${directory}`);
	else await rm(directory, { recursive: true, force: true });
}

process.exit(0);
