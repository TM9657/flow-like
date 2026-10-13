import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { gunzipSync, strToU8, unzipSync, zipSync } from "fflate";
import puppeteer from "puppeteer";
import sharp from "sharp";

declare global {
	interface Window {
		messages: Array<{
			type: string;
			dirty?: boolean;
			requestId?: string;
			revision?: number;
			error?: string;
			bytes?: ArrayBuffer;
			warnings?: string[];
		}>;
	}
}
async function verify(id: string) {
	const directory = await mkdtemp(
		resolve(tmpdir(), "flow-craft-image-browser-"),
	);
	assert.ok(["vectorcraft", "photocraft", "designcraft"].includes(id));
	const vector = id === "vectorcraft";
	const design = id === "designcraft";
	const format = design ? "idml" : vector ? "svg" : "png";
	const filename = design
		? "layout.idml"
		: vector
			? "drawing.svg"
			: "image.png";
	const sample = design
		? zipSync({
				mimetype: [
					strToU8("application/vnd.adobe.indesign-idml-package"),
					{ level: 0 },
				],
				"designmap.xml": strToU8(
					'<Document xmlns:idPkg="http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging" Self="d"><idPkg:Spread src="Spreads/one.xml"/></Document>',
				),
				"Spreads/one.xml": strToU8(
					'<idPkg:Spread xmlns:idPkg="http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging"><Spread Self="s"><Page Self="p" GeometricBounds="0 0 300 400" ItemTransform="1 0 0 1 0 0"/><Rectangle Self="r" ItemTransform="1 0 0 1 0 0" FillColor="Color/Black"><Properties><PathGeometry><GeometryPathType PathOpen="false"><PathPointArray><PathPointType Anchor="30 30" LeftDirection="30 30" RightDirection="30 30"/><PathPointType Anchor="200 30" LeftDirection="200 30" RightDirection="200 30"/><PathPointType Anchor="200 150" LeftDirection="200 150" RightDirection="200 150"/><PathPointType Anchor="30 150" LeftDirection="30 150" RightDirection="30 150"/></PathPointArray></GeometryPathType></PathGeometry></Properties></Rectangle></Spread></idPkg:Spread>',
				),
			})
		: vector
			? new TextEncoder().encode(
					'<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300"><rect x="30" y="30" width="260" height="180" fill="#ff2200"/><circle cx="220" cy="150" r="50" fill="#2244ff"/></svg>',
				)
			: await sharp({
					create: {
						width: 400,
						height: 300,
						channels: 4,
						background: { r: 230, g: 100, b: 40, alpha: 1 },
					},
				})
					.png()
					.toBuffer();
	const assets = resolve(import.meta.dir, "../../apps/web/public/craft");
	const server = Bun.serve({
		hostname: "127.0.0.1",
		port: 0,
		async fetch(req) {
			const path = new URL(req.url).pathname;
			if (path === "/")
				return new Response(
					`<!doctype html><link rel="icon" href="data:,"><style>html,body{margin:0;width:100%;height:100%}iframe{width:100%;height:100%;border:0}</style><iframe id="editor" src="/${id}/index.html?session=smoke"></iframe><script>window.messages=[];window.addEventListener('message',e=>{if(e.source===document.querySelector('iframe').contentWindow){window.messages.push(e.data);console.log('HOST',e.data.type,e.data.error||'',e.data.dirty)}});</script>`,
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
	try {
		const page = await browser.newPage();
		await page.setViewport({ width: 1280, height: 900 });

		const errors: string[] = [];
		page.on("pageerror", (e) => errors.push(String(e)));
		page.on("console", (m) => {
			if (m.type() === "error") console.error(id, m.text());
		});
		await page.setRequestInterception(true);
		page.on("request", (request) => {
			if (
				request.url().startsWith(server.url.origin) ||
				/^(blob|data):/.test(request.url())
			)
				void request.continue();
			else {
				errors.push(`External request: ${request.url()}`);
				void request.abort();
			}
		});
		await page.goto(server.url.origin);
		const event = async (type: string, requestId?: string) => {
			await page.waitForFunction(
				(type, requestId) =>
					window.messages.some(
						(m) => m.type === type && (!requestId || m.requestId === requestId),
					),
				{ timeout: 120000 },
				type,
				requestId,
			);
			return page.evaluate(
				(type, requestId) => {
					const m = window.messages.find(
						(m) => m.type === type && (!requestId || m.requestId === requestId),
					);
					if (!m) throw new Error(`Missing runtime event: ${type}`);
					return {
						...m,
						bytes: m.bytes ? Array.from(new Uint8Array(m.bytes)) : undefined,
					};
				},
				type,
				requestId,
			);
		};
		const send = async (message: {
			type: string;
			name?: string;
			bytes?: number[];
			readOnly?: boolean;
			requestId?: string;
			format?: string;
			revision?: number;
		}) =>
			page.evaluate((message) => {
				const bytes = message.bytes
					? Uint8Array.from(message.bytes).buffer
					: undefined;
				const target = document.querySelector("iframe")?.contentWindow;
				if (!target) throw new Error("The editor iframe is missing.");
				target.postMessage(
					{ ...message, bytes, channel: "flow-like-craft", session: "smoke" },
					location.origin,
				);
			}, message);
		await event("ready");
		await send({
			type: "open",
			name: filename,
			bytes: Array.from(sample),
			readOnly: true,
		});
		const opened = await event("opened");
		if (design)
			assert.ok(
				opened.warnings?.some((warning) =>
					warning.includes("Tables, footnotes"),
				),
				"IDML warns about unsupported content before editing",
			);
		await new Promise((r) => setTimeout(r, 400));
		await page.screenshot({ path: resolve(directory, `${id}-preview.png`) });
		const originalPreview = await page.screenshot();
		const preview = await sharp(originalPreview).stats();
		assert.ok(
			preview.channels.some((c) => c.stdev > 20),
			"rendered preview is nonuniform",
		);
		await send({ type: "export", requestId: "readonly", format });
		assert.match((await event("error", "readonly")).error || "", /editing/i);
		await send({ type: "set-read-only", readOnly: false });
		await new Promise((r) => setTimeout(r, 1500));
		await page.screenshot({ path: resolve(directory, `${id}-editing.png`) });
		const frame = page
			.frames()
			.find((f) => f.url().includes(`${id}/index.html`));
		assert.ok(frame);
		await frame.click("canvas", { offset: { x: 600, y: 420 } });
		if (vector) {
			await page.mouse.move(300, 300);
			await page.mouse.down();
			await page.mouse.move(340, 325, { steps: 12 });
			await page.mouse.up();
		} else if (design) {
			await page.keyboard.press("KeyM");
			await page.mouse.move(300, 300);
			await page.mouse.down();
			await page.mouse.move(420, 410, { steps: 12 });
			await page.mouse.up();
		} else {
			await page.keyboard.press("KeyB");
			await page.mouse.move(500, 460);
			await page.mouse.down();
			await page.mouse.move(620, 520, { steps: 12 });
			await page.mouse.up();
		}
		await page.waitForFunction(
			() => window.messages.some((m) => m.type === "dirty" && m.dirty),
			{ timeout: 20000 },
		);
		await send({ type: "export", requestId: "save", format });
		const output = await event("exported", "save");
		assert.ok(output.bytes);
		assert.ok(typeof output.revision === "number");
		const bytes = Uint8Array.from(output.bytes);
		if (design) {
			const archive = unzipSync(bytes);
			assert.equal(
				new TextDecoder().decode(archive.mimetype),
				"application/vnd.adobe.indesign-idml-package",
			);
			assert.ok(archive["designmap.xml"]);
		} else if (vector) assert.match(new TextDecoder().decode(bytes), /<svg/);
		else {
			assert.deepEqual(
				[...bytes.slice(0, 8)],
				[137, 80, 78, 71, 13, 10, 26, 10],
			);
			const metadata = await sharp(bytes).metadata();
			assert.equal(metadata.width, 400);
			assert.equal(metadata.height, 300);
		}
		assert.notDeepEqual([...bytes], [...sample], "edited bytes changed");

		await page.mouse.move(vector ? 320 : 520, vector ? 320 : 440);
		await page.mouse.down();
		await page.mouse.move(vector ? 360 : 640, vector ? 345 : 500, {
			steps: 12,
		});
		await page.mouse.up();
		await send({ type: "saved", revision: output.revision });
		await new Promise((r) => setTimeout(r, 300));
		assert.equal(
			await page.evaluate(() =>
				window.messages.some((m) => m.type === "dirty" && !m.dirty),
			),
			false,
			"stale save acknowledgment leaves newer edits dirty",
		);
		await send({ type: "export", requestId: "save-latest", format });
		const latest = await event("exported", "save-latest");
		assert.ok(typeof latest.revision === "number");
		assert.ok(
			latest.revision > output.revision,
			"second edit has a new revision",
		);
		await send({ type: "saved", revision: latest.revision });
		await page.waitForFunction(
			() => window.messages.some((m) => m.type === "dirty" && !m.dirty),
			{ timeout: 20000 },
		);
		const nativeFormat = design
			? "designcraft"
			: vector
				? "vectorcraft"
				: "pcraft";
		await send({ type: "export", requestId: "native", format: nativeFormat });
		const native = await event("exported", "native");
		assert.ok(native.bytes);
		const nativeBytes = Uint8Array.from(native.bytes);
		if (vector) {
			const jsonBytes =
				nativeBytes[0] === 31 ? gunzipSync(nativeBytes) : nativeBytes;
			assert.match(
				new TextDecoder().decode(jsonBytes),
				/"format"\s*:\s*"(vectorcraft|drawcraft)"/,
			);
		} else {
			const archive = unzipSync(nativeBytes);
			assert.ok(
				Object.keys(archive).length > 0,
				"native document is a ZIP archive",
			);
			if (!design) assert.ok(archive["manifest.json"]);
		}
		assert.ok(latest.bytes);
		await page.evaluate(() => {
			window.messages = [];
			const iframe = document.querySelector("iframe");
			if (!iframe) throw new Error("The editor iframe is missing.");
			iframe.contentWindow?.location.reload();
		});
		await event("ready");
		await send({
			type: "open",
			name: filename,
			bytes: latest.bytes,
			readOnly: true,
		});
		await event("opened");
		await new Promise((resolve) => setTimeout(resolve, 400));
		const reopened = await page.screenshot({
			path: resolve(directory, `${id}-reopened.png`),
		});
		assert.notDeepEqual(
			reopened,
			originalPreview,
			"reopened output contains the visible edits",
		);
		assert.equal(
			await page.evaluate(() =>
				window.messages.some(
					(message) => message.type === "dirty" && message.dirty,
				),
			),
			false,
			"reopening the saved file starts clean",
		);
		await page.evaluate(() => {
			window.messages = [];
			document.querySelector("iframe")?.contentWindow?.location.reload();
		});
		await event("ready");
		await send({
			type: "open",
			name: `converted.${nativeFormat}`,
			bytes: native.bytes,
			readOnly: true,
		});
		await event("opened");
		await new Promise((resolve) => setTimeout(resolve, 400));
		const nativePreview = await page.screenshot({
			path: resolve(directory, `${id}-native.png`),
		});
		assert.notDeepEqual(
			nativePreview,
			originalPreview,
			"native format preserves the visible edits",
		);
		assert.ok(
			(await sharp(nativePreview).stats()).channels.some((c) => c.stdev > 20),
		);
		assert.deepEqual(errors, []);
		console.log(
			`${id}: read-only preview, editing, revision-safe save, same-format reopen (${bytes.length} bytes), and native-format reopen (${nativeBytes.length} bytes) passed.`,
		);
		if (process.env.CRAFT_KEEP_SCREENSHOTS)
			console.log(`Browser screenshots: ${directory}`);
		else await rm(directory, { recursive: true, force: true });
	} catch (error) {
		console.error(`Browser screenshots: ${directory}`);
		throw error;
	} finally {
		await browser.close();
		server.stop(true);
	}
}

const selected = process.argv.slice(2);
for (const id of selected.length
	? selected
	: ["vectorcraft", "photocraft", "designcraft"]) {
	await verify(id);
}
