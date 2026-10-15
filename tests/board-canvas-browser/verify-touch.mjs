import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium } from "playwright";

const origin = process.env.BOARD_CANVAS_URL ?? "http://127.0.0.1:4338";
const output = resolve(
	process.env.BOARD_CANVAS_OUTPUT ?? "tmp/board-canvas-browser/touch",
);
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
	headless: true,
	executablePath:
		process.env.CHROME_PATH ??
		"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
});
const context = await browser.newContext({
	viewport: { width: 1280, height: 960 },
	hasTouch: true,
	deviceScaleFactor: 1,
});
const page = await context.newPage();
const errors = [];
const results = [];
page.on("pageerror", (error) => errors.push(error.message));
const api = (method, ...args) =>
	page.evaluate(
		({ method, args }) => window.__boardCanvasHarness[method](...args),
		{ method, args },
	);
const cdp = await context.newCDPSession(page);
const touch = async (type, points) => {
	await cdp.send("Input.dispatchTouchEvent", {
		type,
		touchPoints: points.map((point) => ({
			...point,
			radiusX: 4,
			radiusY: 4,
			force: 1,
		})),
	});
	await page.waitForTimeout(25);
};
const point = (id, x, y) => ({ id, x, y });
try {
	await page.goto(`${origin}/?boardRenderer=egui`, {
		waitUntil: "networkidle",
	});
	await page.waitForFunction(() => window.__boardCanvasHarness?.ready());
	assert.equal(await page.locator("[data-standalone-board]").count(), 1);
	assert.equal(await page.locator(".react-flow__renderer").count(), 0);
	await api("setViewport", 20, 10, 1);
	const before = await api("snapshot");
	const bounds = await page.locator("[data-standalone-board]").boundingBox();
	assert.ok(bounds);
	const center = { x: bounds.x + 650, y: bounds.y + 330 };
	const a = point(1, center.x - 75, center.y);
	const b = point(2, center.x + 75, center.y);
	await touch("touchStart", [a]);
	await touch("touchStart", [a, b]);
	for (let i = 1; i <= 6; i++) {
		await touch("touchMove", [
			point(1, a.x - i * 8, a.y + i * 3),
			point(2, b.x + i * 8, b.y + i * 3),
		]);
	}
	await touch("touchEnd", [point(1, a.x - 48, a.y + 18)]);
	await touch("touchEnd", []);
	const after = await api("snapshot");
	assert.ok(
		Math.abs(after.viewport.zoom - 1.64) < 0.03,
		`Pinch zoom ${after.viewport.zoom}`,
	);
	const anchor = {
		x: (650 - before.viewport.x) / before.viewport.zoom,
		y: (330 - before.viewport.y) / before.viewport.zoom,
	};
	assert.ok(
		Math.abs(anchor.x * after.viewport.zoom + after.viewport.x - 650) < 2,
	);
	assert.ok(
		Math.abs(anchor.y * after.viewport.zoom + after.viewport.y - 348) < 2,
	);
	assert.deepEqual(after.nodes, before.nodes);
	assert.deepEqual(after.history, before.history);
	results.push({
		name: "Two-touch zoom keeps the moving centroid anchored without editing the graph",
		passed: true,
	});

	await api("reset");
	await api("setViewport", 20, 10, 1);
	const initial = await api("snapshot");
	const node = await api("node", "exec-b");
	const first = point(3, node.x, node.y);
	await touch("touchStart", [first]);
	await touch("touchMove", [point(3, first.x + 30, first.y + 20)]);
	const second = point(4, first.x + 180, first.y + 20);
	await touch("touchStart", [point(3, first.x + 30, first.y + 20), second]);
	await touch("touchMove", [
		point(3, first.x + 10, first.y + 20),
		point(4, second.x + 20, second.y),
	]);
	await touch("touchCancel", []);
	await page.waitForTimeout(100);
	const cancelled = await api("snapshot");
	assert.deepEqual(
		cancelled.nodes,
		initial.nodes,
		"Second touch restores the pending single-finger drag",
	);
	assert.deepEqual(cancelled.history, initial.history);
	assert.equal(cancelled.commits, initial.commits);
	assert.ok(
		cancelled.viewport.zoom > 1,
		"Pinch was active before cancellation",
	);
	const nextNode = await api("node", "exec-b");
	await page.mouse.move(nextNode.x, nextNode.y);
	await page.mouse.down();
	await page.mouse.move(nextNode.x + 40, nextNode.y + 20, { steps: 5 });
	await page.mouse.up();
	await page.waitForFunction(
		(count) => window.__boardCanvasHarness.snapshot().commits > count,
		initial.commits,
	);
	results.push({
		name: "Pinch cancels a pending node drag and touch cancellation releases the next pointer gesture",
		passed: true,
	});
	assert.deepEqual(errors, []);
	await page.screenshot({ path: resolve(output, "touch.png") });
	console.log(JSON.stringify({ passed: results.length, results }));
} catch (error) {
	await page
		.screenshot({ path: resolve(output, "failure.png") })
		.catch(() => {});
	throw error;
} finally {
	await writeFile(
		resolve(output, "results.json"),
		JSON.stringify({ results, errors }, null, 2),
	);
	await browser.close();
}
