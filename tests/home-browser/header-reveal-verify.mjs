import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { chromium } from "playwright-core";

const baseURL = process.env.HOME_BROWSER_BASE_URL || "http://127.0.0.1:4329";
const browser = await chromium.launch({
	executablePath:
		process.env.CHROME_EXECUTABLE_PATH ||
		(process.platform === "darwin"
			? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
			: undefined),
	headless: true,
	args: ["--no-sandbox"],
});
const report = { passed: [], errors: [], blocked: [] };
const createPage = async (options) => {
	const page = await browser.newPage(options);
	page.on("pageerror", (error) => report.errors.push(error.message));
	page.on("console", (message) => {
		if (message.type() === "error") report.errors.push(message.text());
	});
	await page.route("**/*", (route) => {
		const url = new URL(route.request().url());
		if (url.origin === baseURL || ["data:", "blob:"].includes(url.protocol))
			return route.continue();
		report.blocked.push(url.origin);
		return route.abort();
	});
	return page;
};
const page = await createPage({ viewport: { width: 1480, height: 1050 } });
const controls = (target = page) => target.locator("[data-home-controls]");
const opacity = (target, expected) =>
	target.waitForFunction(
		(value) =>
			getComputedStyle(document.querySelector("[data-home-controls]"))
				.opacity === String(value),
		expected,
	);
const ready = async (target, pathname = "/default-fixture") => {
	await target.goto(`${baseURL}${pathname}`, { waitUntil: "domcontentloaded" });
	await controls(target).waitFor({ timeout: 60_000 });
	await target.locator("[data-home-widget]").first().waitFor();
};
const away = async (target = page) => {
	const box = await target.locator("[data-home-editor]").boundingBox();
	await target.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
};
const near = async () => {
	const box = await controls().boundingBox();
	await page.mouse.move(box.x - 40, box.y + box.height / 2);
};
const geometry = () =>
	page.evaluate(() => {
		const box = (selector) => {
			const { x, y, width, height } = document
				.querySelector(selector)
				.getBoundingClientRect();
			return { x, y, width, height };
		};
		return {
			editor: box("[data-home-editor]"),
			scroll: box("[data-home-scroll]"),
			canvas: box("[data-home-canvas]"),
		};
	});

try {
	await ready(page);
	assert.equal(
		await page.evaluate(
			() => matchMedia("(hover: hover) and (pointer: fine)").matches,
		),
		true,
	);
	await away();
	await opacity(page, 0);
	assert.equal(await controls().getAttribute("data-floating"), "true");
	assert.equal(
		await controls().evaluate(
			(element) => getComputedStyle(element).pointerEvents,
		),
		"none",
		"Hidden controls leave the canvas available to pointer input",
	);
	assert.equal(await controls().getByText("Home", { exact: true }).count(), 0);
	assert.equal(
		await controls().getByText("Default home", { exact: true }).count(),
		0,
	);
	const hiddenGeometry = await geometry();
	assert.ok(
		Math.abs(hiddenGeometry.editor.y - hiddenGeometry.scroll.y) <= 1,
		"Personal home content starts at the editor top without a header row",
	);
	await page.screenshot({ path: "/private/tmp/home-header-hidden.png" });
	await near();
	await opacity(page, 1);
	assert.equal(await controls().getAttribute("data-revealed"), "true");
	assert.deepEqual(
		await geometry(),
		hiddenGeometry,
		"Revealing controls does not move or resize the canvas",
	);
	await page.screenshot({ path: "/private/tmp/home-header-revealed.png" });
	await away();
	await opacity(page, 0);
	report.passed.push(
		"Home has no reserved header row; approaching the controls reveals them without moving content, and moving away hides them",
	);

	await near();
	await opacity(page, 1);
	await page
		.getByRole("button", { name: "Layout options", exact: true })
		.click();
	await page.getByRole("menuitem", { name: "Edit with FlowPilot" }).waitFor();
	await away();
	await opacity(page, 1);
	assert.equal(await controls().getAttribute("data-revealed"), "true");
	await page.keyboard.press("Escape");
	await page.getByLabel("Profile state", { exact: true }).focus();
	await away();
	await opacity(page, 0);
	report.passed.push(
		"Layout controls remain visible while the options menu is open",
	);

	await page.keyboard.press("Tab");
	assert.equal(
		await page
			.getByRole("button", { name: "Customize", exact: true })
			.evaluate((element) => element === document.activeElement),
		true,
		"Tab reaches Customize even when pointer controls are hidden",
	);
	await opacity(page, 1);
	await page.keyboard.press("Enter");
	await page.locator('[data-home-editor][data-editing="true"]').waitFor();
	await away();
	await opacity(page, 1);
	assert.equal(await controls().getAttribute("data-floating"), "false");
	await page.getByRole("button", { name: "Save", exact: true }).waitFor();
	const editGeometry = await geometry();
	assert.ok(
		editGeometry.scroll.y > editGeometry.editor.y + 40,
		"Editing keeps a persistent toolbar with its own space",
	);
	report.passed.push(
		"Keyboard focus reveals Customize, and entering edit mode keeps the complete toolbar visible",
	);

	await ready(page, "/?admin=1");
	await away();
	await opacity(page, 1);
	assert.equal(await controls().getAttribute("data-floating"), "false");
	await controls().getByText("Default home", { exact: true }).waitFor();
	await page
		.getByRole("button", { name: "Edit default", exact: true })
		.waitFor();
	report.passed.push(
		"Default-home administration retains its persistent toolbar",
	);

	const touch = await createPage({
		viewport: { width: 390, height: 844 },
		hasTouch: true,
		isMobile: true,
	});
	await ready(touch);
	assert.equal(
		await touch.evaluate(
			() => matchMedia("(hover: hover) and (pointer: fine)").matches,
		),
		false,
	);
	await opacity(touch, 1);
	const touchBox = await controls(touch).boundingBox();
	assert.ok(touchBox.x >= 0 && touchBox.x + touchBox.width <= 391);
	await touch.screenshot({ path: "/private/tmp/home-header-touch.png" });
	await touch.getByRole("button", { name: "Customize", exact: true }).tap();
	await touch.locator('[data-home-editor][data-editing="true"]').waitFor();
	report.passed.push(
		"Touch devices keep compact controls visible and can tap Customize",
	);
	await touch.close();
	assert.deepEqual(report.errors, []);
	assert.deepEqual(report.blocked, []);
} catch (error) {
	report.failure = error.stack;
	await page.screenshot({ path: "/private/tmp/home-header-failure.png" });
	throw error;
} finally {
	await writeFile(
		"/private/tmp/home-header-report.json",
		JSON.stringify(report, null, 2),
	);
	console.log(JSON.stringify(report, null, 2));
	await browser.close();
}
