import assert from "node:assert/strict";
import { chromium } from "playwright-core";

const origin = process.argv[2] ?? "http://127.0.0.1:4318";
const allowedHost = new URL(origin).hostname;
const browser = await chromium.launch({
	executablePath:
		process.env.CHROME_EXECUTABLE_PATH ||
		(process.platform === "darwin"
			? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
			: undefined),
	headless: true,
	args: ["--disable-dev-shm-usage", "--no-sandbox"],
});
const context = await browser.newContext({
	viewport: { width: 1280, height: 900 },
	permissions: ["clipboard-read", "clipboard-write"],
});
const page = await context.newPage();
const errors = [];

await page.route("**/*", (route) => {
	const url = new URL(route.request().url());
	return [allowedHost, "cdn.jsdelivr.net"].includes(url.hostname) ||
		["data:", "blob:"].includes(url.protocol)
		? route.continue()
		: route.abort();
});
page.on("pageerror", (error) =>
	errors.push(`page: ${error.stack ?? error.message}`),
);
page.on("console", (message) => {
	if (message.type() === "error") errors.push(`console: ${message.text()}`);
});

const imported = {
	version: 1,
	title: "Transferred home",
	widgets: [
		{
			id: "transferred-message",
			type: "information",
			title: "Transferred layout",
			description: "Copied from another profile",
			size: { columns: 6, rows: 3 },
			appearance: { variant: "card", accent: "blue" },
			config: { mode: "markdown", body: "## Imported JSON\nReady to use." },
		},
	],
};

const replaceEditorValue = async (value) => {
	await page.waitForFunction(
		() => globalThis.monaco?.editor.getEditors().length > 0,
	);
	await page.evaluate(
		() =>
			new Promise((resolve) =>
				requestAnimationFrame(() => requestAnimationFrame(resolve)),
			),
	);
	await page.evaluate((next) => {
		const editors = globalThis.monaco.editor.getEditors();
		editors.at(-1).setValue(next);
	}, value);
	await page.waitForFunction(
		(next) => globalThis.monaco.editor.getEditors().at(-1).getValue() === next,
		value,
	);
	await page.evaluate(
		() =>
			new Promise((resolve) =>
				requestAnimationFrame(() => requestAnimationFrame(resolve)),
			),
	);
};

try {
	await page.goto(`${origin}/`, {
		waitUntil: "domcontentloaded",
	});
	await page
		.getByRole("button", { name: "Customize", exact: true })
		.waitFor({ timeout: 60_000 });
	await page.getByRole("button", { name: "Customize", exact: true }).click();
	await page.getByRole("button", { name: "Edit JSON", exact: true }).click();
	await page.getByRole("dialog").waitFor();
	await page.getByRole("button", { name: "Copy", exact: true }).click();
	await page.getByRole("button", { name: "Copied", exact: true }).waitFor();
	await page.locator(".monaco-editor").waitFor({ timeout: 60_000 });
	await page.screenshot({
		path: "/private/tmp/home-json-editor.png",
		fullPage: true,
	});
	assert.match(
		await page.evaluate(() => navigator.clipboard.readText()),
		/"version": 1/,
	);
	await page
		.getByRole("button", { name: "Apply changes", exact: true })
		.click();
	await page.getByRole("button", { name: "Applied", exact: true }).waitFor();
	assert.equal(
		await page.getByText("Editing layout", { exact: true }).count(),
		1,
	);
	assert.equal(
		await page.locator('button[aria-label="Undo layout change"]').isDisabled(),
		true,
	);

	await replaceEditorValue("{");
	await page
		.getByRole("button", { name: "Apply changes", exact: true })
		.click();
	const validationAlert = page
		.getByRole("dialog", { name: "Home layout JSON" })
		.locator('[role="alert"]')
		.filter({ hasText: "Invalid JSON" });
	await validationAlert.waitFor();
	assert.match(await validationAlert.innerText(), /Invalid JSON/);
	assert.equal(await page.locator("[data-home-widget]").count(), 4);
	await page.locator(".monaco-editor").click();
	await page.keyboard.press(
		process.platform === "darwin" ? "Meta+S" : "Control+S",
	);
	assert.equal(
		await page.evaluate(() => window.homeQa.counters.saveAttempts),
		0,
	);
	await page.getByRole("button", { name: "Close JSON editor" }).click();
	await page.getByRole("button", { name: "Edit JSON", exact: true }).click();
	await page.locator(".monaco-editor").waitFor();

	await replaceEditorValue(JSON.stringify(imported));
	await page
		.getByRole("button", { name: "Apply changes", exact: true })
		.click();
	await page.getByRole("button", { name: "Applied", exact: true }).waitFor();
	await page.getByRole("button", { name: "Close JSON editor" }).click();
	await page.locator('[data-home-widget="transferred-message"]').waitFor();
	assert.equal(await page.locator("[data-home-widget]").count(), 1);
	assert.equal(
		await page.getByText("Unsaved changes", { exact: true }).count(),
		1,
	);

	await page.getByRole("button", { name: "Undo layout change" }).click();
	assert.equal(await page.locator("[data-home-widget]").count(), 4);
	await page.getByRole("button", { name: "Redo layout change" }).click();
	await page.locator('[data-home-widget="transferred-message"]').waitFor();

	await page.getByRole("button", { name: "Save", exact: true }).click();
	await page.locator('[data-home-editor][data-editing="false"]').waitFor();
	assert.deepEqual(
		JSON.parse(
			JSON.stringify(await page.evaluate(() => window.homeQa.getSaved())),
		),
		imported,
	);

	await page.getByRole("button", { name: "Customize", exact: true }).click();
	await page.getByRole("button", { name: "Layout options" }).click();
	await page.getByRole("menuitem", { name: "Reset to default" }).click();
	await page.getByRole("button", { name: "Reset draft", exact: true }).click();
	await page.getByRole("button", { name: "Edit JSON", exact: true }).click();
	await page.locator(".monaco-editor").waitFor();
	await page
		.getByRole("button", { name: "Apply changes", exact: true })
		.click();
	await page.getByRole("button", { name: "Applied", exact: true }).waitFor();
	await page.getByRole("button", { name: "Close JSON editor" }).click();
	await page.getByRole("button", { name: "Save", exact: true }).click();
	await page.locator('[data-home-editor][data-editing="false"]').waitFor();
	assert.equal(await page.evaluate(() => window.homeQa.counters.resets), 1);

	const mobilePage = await context.newPage();
	await mobilePage.setViewportSize({ width: 390, height: 844 });
	await mobilePage.route("**/*", (route) => {
		const url = new URL(route.request().url());
		return [allowedHost, "cdn.jsdelivr.net"].includes(url.hostname) ||
			["data:", "blob:"].includes(url.protocol)
			? route.continue()
			: route.abort();
	});
	mobilePage.on("pageerror", (error) =>
		errors.push(`page: ${error.stack ?? error.message}`),
	);
	mobilePage.on("console", (message) => {
		if (message.type() === "error") errors.push(`console: ${message.text()}`);
	});
	await mobilePage.goto(`${origin}/`, { waitUntil: "domcontentloaded" });
	await mobilePage
		.getByRole("button", { name: "Customize", exact: true })
		.waitFor({ timeout: 60_000 });
	await mobilePage
		.getByRole("button", { name: "Customize", exact: true })
		.click();
	await mobilePage
		.getByRole("button", { name: "Close widget panel", exact: true })
		.click();
	await mobilePage
		.getByRole("button", { name: "Edit JSON", exact: true })
		.click();
	await mobilePage.locator(".monaco-editor").waitFor({ timeout: 60_000 });
	const mobileDialog = mobilePage.getByRole("dialog", {
		name: "Home layout JSON",
	});
	const mobileBounds = await mobileDialog.boundingBox();
	assert.ok(mobileBounds);
	assert.ok(mobileBounds.x >= 0);
	assert.ok(mobileBounds.x + mobileBounds.width <= 390);
	assert.equal(
		await mobilePage.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth + 1,
		),
		true,
	);
	await mobilePage.screenshot({
		path: "/private/tmp/home-json-editor-mobile.png",
		fullPage: true,
	});
	await mobilePage.close();

	assert.deepEqual(
		errors.filter(
			(error) =>
				!error.includes("Uncaught (in promise) Canceled: Canceled") ||
				!error.includes("monaco-editor"),
		),
		[],
	);
	console.log("Home JSON copy, validation, apply, undo, redo, and save passed");
} finally {
	await browser.close();
}
