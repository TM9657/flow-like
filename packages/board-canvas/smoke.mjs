import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer";
import sharp from "sharp";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const assets = resolve(root, "apps/web/public/board-canvas");
const output = resolve(root, "tmp/board-canvas");
await mkdir(output, { recursive: true });
const server = createServer(async (request, response) => {
	try {
		const path = new URL(request.url, "http://localhost").pathname;
		if (path === "/") {
			response.setHeader("Content-Type", "text/html");
			response.end(
				'<!doctype html><html><head><style>html,body{margin:0;width:100%;height:100%;overflow:hidden}#surface{width:100%;height:100%;outline:none}canvas{width:100%;height:100%;display:block;outline:none;pointer-events:none}</style></head><body><div id="surface" tabindex="0"><canvas id="board" tabindex="-1"></canvas></div></body></html>',
			);
			return;
		}
		const file = path.slice(1);
		if (
			!/^(board_canvas\.js|board_canvas_bg\.wasm|Inter-(Regular|Medium)\.ttf)$/.test(
				file,
			)
		) {
			response.writeHead(404).end();
			return;
		}
		response.setHeader(
			"Content-Type",
			file.endsWith(".wasm")
				? "application/wasm"
				: file.endsWith(".js")
					? "text/javascript"
					: "application/octet-stream",
		);
		response.end(await readFile(resolve(assets, file)));
	} catch (error) {
		response.writeHead(500).end(String(error));
	}
});
await new Promise((resolve, reject) => {
	server.once("error", reject);
	server.listen(0, "127.0.0.1", resolve);
});
let browser;
try {
	browser = await puppeteer.launch({
		headless: process.argv.includes("--headless"),
		executablePath:
			process.env.CHROME_PATH ||
			"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
		defaultViewport: null,
		args: [
			"--window-size=1100,800",
			"--disable-background-timer-throttling",
			"--disable-renderer-backgrounding",
		],
	});
	const page = await browser.newPage();
	const errors = [];
	page.on("pageerror", (error) => errors.push(String(error)));
	await page.goto(`http://127.0.0.1:${server.address().port}`);
	await page.evaluate(async () => {
		window.browserEvents = [];
		for (const event of [
			"pointermove",
			"pointerdown",
			"pointerup",
			"wheel",
			"keydown",
			"keyup",
			"focus",
			"blur",
			"resize",
			"visibilitychange",
		]) {
			window.addEventListener(
				event,
				() => window.browserEvents.push({ event, at: performance.now() }),
				true,
			);
		}
		const hostInput = document.createElement("input");
		hostInput.id = "host-input";
		hostInput.style.position = "absolute";
		hostInput.style.left = "-1000px";
		document.body.append(hostInput);
		hostInput.focus();
		const module = await import("/board_canvas.js");
		await module.default();
		const palette = {
			background: [18, 20, 27, 255],
			card: [29, 33, 43, 255],
			foreground: [230, 232, 238, 255],
			muted: [140, 145, 160, 255],
			border: [63, 69, 84, 255],
			primary: [68, 140, 240, 255],
			tertiary: [45, 180, 145, 255],
		};
		const node = (id, x, y, kind) => ({
			id,
			x,
			y,
			width: 180,
			height: 92,
			label: `${kind} workflow ${id}`,
			kind,
			pins: [
				{
					id: "in",
					direction: "input",
					x: 0,
					y: 36,
					label: "Run",
					labelMaxWidth: 70,
					color: palette.foreground,
					kind: "execution",
				},
				{
					id: "out",
					direction: "output",
					x: 180,
					y: 36,
					label: "Next",
					labelMaxWidth: 70,
					color: palette.foreground,
					kind: "execution",
				},
				{
					id: "data-in",
					direction: "input",
					x: 0,
					y: 64,
					label: "Input with a long label",
					labelMaxWidth: 70,
					color: [60, 185, 145, 255],
					kind: "data",
				},
				{
					id: "data-out",
					direction: "output",
					x: 180,
					y: 64,
					label: "Output value",
					labelMaxWidth: 70,
					color: [60, 185, 145, 255],
					kind: "data",
				},
			],
		});
		const scene = {
			nodes: [
				node("a", 50, 60, "event"),
				node("b", 380, 160, "standard"),
				node("c", 710, 60, "pure"),
			],
			edges: [
				{
					id: "ab",
					source: { nodeId: "a", pinId: "out" },
					target: { nodeId: "b", pinId: "in" },
					kind: "execution",
					color: palette.foreground,
				},
				{
					id: "bc",
					source: { nodeId: "b", pinId: "data-out" },
					target: { nodeId: "c", pinId: "data-in" },
					kind: "data",
					color: [60, 185, 145, 255],
				},
			],
		};
		window.scene = scene;
		window.palette = palette;
		window.handle = await module.start(
			document.getElementById("board"),
			JSON.stringify(scene),
			JSON.stringify(palette),
		);
		window.startupFocusPreserved = document.activeElement === hostInput;
		const fonts = await Promise.all(
			["Regular", "Medium"].map(
				async (weight) =>
					new Uint8Array(
						await (await fetch(`/Inter-${weight}.ttf`)).arrayBuffer(),
					),
			),
		);
		window.handle.set_fonts(...fonts);
		window.handle.set_viewport(20, 20, 1);
	});
	const stats = () => page.evaluate(() => JSON.parse(window.handle.stats()));
	const afterFrame = (frames) =>
		page.waitForFunction(
			(previous) => JSON.parse(window.handle.stats()).frames > previous,
			{},
			frames,
		);
	await page.waitForFunction(() => {
		const current = JSON.parse(window.handle.stats());
		return (
			current.frames > 1 &&
			current.viewport[0] === 20 &&
			current.viewport[1] === 20
		);
	});
	const before = await stats();
	assert.deepEqual(
		await page.evaluate(() => ({
			focus: window.startupFocusPreserved,
			inputs: document.querySelectorAll("input").length,
			tabIndex: document.getElementById("board").tabIndex,
			surfaceTabIndex: document.getElementById("surface").tabIndex,
		})),
		{ focus: true, inputs: 1, tabIndex: -1, surfaceTabIndex: 0 },
	);
	assert.equal(before.nodeCount, 3);
	assert.equal(before.renderedNodes, 3);
	assert.equal(before.renderedEdges, 2);
	assert.equal(before.pinsDrawn, 12);
	assert.equal(before.labelsDrawn, 15);
	assert.deepEqual(
		await page.evaluate(() => {
			const surface = document.getElementById("surface");
			surface.focus();
			let delivered = false;
			const onKey = () => {
				delivered = true;
			};
			window.addEventListener("keydown", onKey);
			const event = new KeyboardEvent("keydown", {
				key: "Tab",
				code: "Tab",
				bubbles: true,
				cancelable: true,
			});
			surface.dispatchEvent(event);
			window.removeEventListener("keydown", onKey);
			document.getElementById("host-input").focus();
			return { delivered, prevented: event.defaultPrevented };
		}),
		{ delivered: true, prevented: false },
	);
	await page.screenshot({ path: resolve(output, "smoke.png") });
	const moveFrame = await page.evaluate(() => {
		const frames = JSON.parse(window.handle.stats()).frames;
		window.handle.set_positions(JSON.stringify([{ id: "b", x: 420, y: 200 }]));
		return frames;
	});
	await afterFrame(moveFrame);
	const moved = await stats();
	assert.equal(moved.geometryRebuilds, before.geometryRebuilds + 2);
	const updateFrame = await page.evaluate(() => {
		const frames = JSON.parse(window.handle.stats()).frames;
		window.scene.nodes[1].x = 420;
		window.scene.nodes[1].y = 200;
		window.scene.nodes[0].dom = true;
		window.handle.set_scene(JSON.stringify(window.scene));
		window.handle.set_overlay(
			JSON.stringify({
				connection: {
					from: [600, 236],
					to: [710, 96],
					fromDirection: "output",
					kind: "reference",
					valid: true,
					color: window.palette.primary,
				},
				hoveredPin: { nodeId: "a", pinId: "out", valid: true },
				selectionRect: [720, 320, -300, -100],
			}),
		);
		return frames;
	});
	await afterFrame(updateFrame);
	const updated = await stats();
	assert.equal(updated.geometryRebuilds, moved.geometryRebuilds);
	assert.equal(updated.renderedNodes, 2);
	assert.equal(updated.renderedEdges, 2);
	const edgeLayerFrame = await page.evaluate(() => {
		const frames = JSON.parse(window.handle.stats()).frames;
		document.body.style.background = "rgb(74, 108, 142)";
		window.handle.set_palette(
			JSON.stringify({ ...window.palette, background: [240, 240, 248, 0] }),
		);
		window.handle.set_layer("edges");
		window.scene.edges[0].dom = true;
		window.handle.set_scene(JSON.stringify(window.scene));
		return frames;
	});
	await afterFrame(edgeLayerFrame);
	const edgesOnly = await stats();
	assert.equal(edgesOnly.paintLayer, "edges");
	assert.equal(edgesOnly.renderedNodes, 0);
	assert.equal(edgesOnly.renderedEdges, 1);
	const transparentImage = await page.screenshot({
		path: resolve(output, "transparent-edges.png"),
	});
	const corner = await sharp(transparentImage)
		.extract({ left: 2, top: 2, width: 1, height: 1 })
		.removeAlpha()
		.raw()
		.toBuffer();
	assert.deepEqual(
		[...corner],
		[74, 108, 142],
		"Transparent clear must preserve the DOM background without leaking RGB",
	);
	const nodeLayerFrame = await page.evaluate(() => {
		const frames = JSON.parse(window.handle.stats()).frames;
		window.handle.set_layer("nodes");
		return frames;
	});
	await afterFrame(nodeLayerFrame);
	const nodesOnly = await stats();
	assert.equal(nodesOnly.paintLayer, "nodes");
	assert.equal(nodesOnly.renderedNodes, 2);
	assert.equal(nodesOnly.renderedEdges, 0);
	await page.evaluate(() => {
		window.handle.set_layer("all");
		window.handle.set_palette(
			JSON.stringify({
				...window.palette,
				background: [240, 240, 248, 255],
				card: [255, 255, 255, 255],
				foreground: [30, 35, 45, 255],
			}),
		);
		window.handle.set_lod(true);
		window.handle.set_viewport(20, 20, 0.08);
	});
	await page.waitForFunction(
		() => JSON.parse(window.handle.stats()).lodLevel === "tiny",
	);
	const lod = await stats();
	assert.equal(lod.labelsDrawn, 0);
	assert.equal(lod.pinsDrawn, 0);
	assert.equal(
		await page.evaluate(() => {
			try {
				window.handle.set_positions(
					JSON.stringify([{ id: "gone", x: 1, y: 1 }]),
				);
				return false;
			} catch {
				return true;
			}
		}),
		true,
	);
	let idleFrames;
	for (let attempt = 0; attempt < 5; attempt++) {
		await new Promise((resolve) => setTimeout(resolve, 600));
		const idleBefore = await page.evaluate(() => ({
			frames: JSON.parse(window.handle.stats()).frames,
			events: window.browserEvents.length,
		}));
		await new Promise((resolve) => setTimeout(resolve, 600));
		const idleAfter = await page.evaluate(() => ({
			frames: JSON.parse(window.handle.stats()).frames,
			events: window.browserEvents.length,
		}));
		// A headed run can receive real pointer/focus events from another active app.
		if (idleAfter.events !== idleBefore.events) continue;
		idleFrames = idleAfter.frames - idleBefore.frames;
		assert.equal(
			idleFrames,
			0,
			"The painter repainted without scene, viewport or browser input changes",
		);
		break;
	}
	assert.notEqual(
		idleFrames,
		undefined,
		"No event-free idle interval was available",
	);
	await page.evaluate(() => {
		window.handle.destroy();
		window.handle.destroy();
		window.handle.free();
	});
	assert.deepEqual(errors, []);
	console.log(
		JSON.stringify({
			before,
			moved,
			updated,
			edgesOnly,
			nodesOnly,
			lod,
			idleFrames,
			screenshot: resolve(output, "smoke.png"),
			devicePixelRatio: await page.evaluate(() => devicePixelRatio),
		}),
	);
} finally {
	if (browser) await browser.close();
	await new Promise((resolve) => server.close(resolve));
}
