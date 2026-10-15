import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium, webkit } from "playwright";

const engine = process.env.BOARD_CANVAS_BROWSER ?? "chromium";
const origin = process.env.BOARD_CANVAS_URL ?? "http://127.0.0.1:4338";
const output = resolve(
	process.env.BOARD_CANVAS_OUTPUT ?? "tmp/board-canvas-browser/culling",
	engine,
);
await mkdir(output, { recursive: true });
const browser =
	engine === "webkit"
		? await webkit.launch({ headless: true })
		: await chromium.launch({
				headless: true,
				executablePath:
					process.env.CHROME_PATH ??
					"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
			});
const page = await browser.newPage({
	viewport: { width: 1280, height: 960 },
	deviceScaleFactor: 1,
});
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
const api = (method, ...args) =>
	page.evaluate(
		({ method, args }) => window.__boardCanvasHarness[method](...args),
		{ method, args },
	);
const results = {};
try {
	await page.goto(`${origin}/?boardRenderer=egui&nodes=10000&richNodes=512`, {
		waitUntil: "networkidle",
	});
	await page.waitForFunction(() => window.__boardCanvasHarness?.ready(), null, {
		timeout: 30000,
	});
	await api("setViewport", 20, 10, 1);
	await page.mouse.move(0, 0);
	await page.waitForTimeout(400);
	results.initial = await page.evaluate(() => ({
		mounted: document.querySelectorAll("[data-board-rich-node]").length,
		eligible: window.__boardCanvasHarness.snapshot().presentation.domNodes,
	}));
	assert.ok(results.initial.eligible > 256);
	assert.ok(
		results.initial.mounted < results.initial.eligible / 4,
		"Distant rich editors are culled",
	);
	assert.equal(
		await page.locator('.react-flow__node[data-id="extra-9910"]').count(),
		0,
	);
	await api("setViewport", 20, -11700, 1);
	const far = page.locator('.react-flow__node[data-id="extra-9910"]');
	await far.waitFor({ state: "visible" });
	await page.waitForTimeout(300);
	const validatePin = async () => {
		const expected = await api("pin", "extra-9910", "extra-9910.out");
		const actual = await far
			.locator('[data-handleid="extra-9910.out"]')
			.boundingBox();
		assert.ok(actual);
		assert.ok(
			Math.hypot(
				expected.x - actual.x - actual.width / 2,
				expected.y - actual.y - actual.height / 2,
			) < 2,
			"Remounted editor ports match native scene geometry",
		);
	};
	await validatePin();
	results.far = await page.locator("[data-board-rich-node]").count();
	await api("setViewport", 20, 10, 1);
	await far.waitFor({ state: "detached" });
	await api("setViewport", 20, -11700, 1);
	await far.waitFor({ state: "visible" });
	await page.waitForTimeout(300);
	await validatePin();
	results.remounted = true;
	assert.deepEqual(errors, []);
	await page.screenshot({ path: resolve(output, "far.png") });
	console.log(JSON.stringify(results));
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
