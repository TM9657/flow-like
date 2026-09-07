import assert from "node:assert/strict";
import { chromium } from "playwright-core";
import { customizeHome } from "./customize-home.mjs";

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
const errors = [];

async function verify(viewport) {
	const context = await browser.newContext({ viewport });
	const page = await context.newPage();
	page.on("pageerror", (error) => errors.push(error.stack ?? error.message));
	await page.route("**/*", (route) => {
		const url = new URL(route.request().url());
		return [allowedHost, "cdn.jsdelivr.net"].includes(url.hostname) ||
			["data:", "blob:"].includes(url.protocol)
			? route.continue()
			: route.abort();
	});
	const dialog = page.getByRole("dialog", { name: "Home layout JSON" });
	const confirmation = page.getByRole("alertdialog", {
		name: "Discard unapplied JSON changes?",
	});
	const close = page.getByRole("button", { name: "Close JSON editor" });
	const waitForFocus = (label) =>
		page.waitForFunction(
			(name) => document.activeElement?.getAttribute("aria-label") === name,
			label,
		);
	const editorValue = () =>
		page.evaluate(() =>
			globalThis.monaco.editor.getEditors().at(-1).getValue(),
		);
	const replaceValue = async (value) => {
		await page.evaluate(
			(next) => globalThis.monaco.editor.getEditors().at(-1).setValue(next),
			value,
		);
		await page.evaluate(
			() => new Promise((resolve) => requestAnimationFrame(resolve)),
		);
	};
	const unloadProtected = () =>
		page.evaluate(() => {
			const event = new Event("beforeunload", { cancelable: true });
			window.dispatchEvent(event);
			return event.defaultPrevented;
		});
	const open = async () => {
		await page
			.getByRole("button", { name: "Layout options", exact: true })
			.click();
		await page
			.getByRole("menuitem", { name: "Edit JSON", exact: true })
			.click();
		await dialog.waitFor();
		await page.getByRole("menu").waitFor({ state: "hidden" });
		await page.waitForFunction(() =>
			document
				.querySelector('[role="dialog"]')
				?.contains(document.activeElement),
		);
		await page.waitForFunction(
			() => globalThis.monaco?.editor.getEditors().length > 0,
			undefined,
			{ timeout: 60_000 },
		);
		await page.evaluate(
			() => new Promise((resolve) => requestAnimationFrame(resolve)),
		);
	};

	try {
		await page.goto(origin, { waitUntil: "domcontentloaded" });
		await customizeHome(page, "Customize", { timeout: 60_000 });
		await page
			.getByRole("button", { name: "Close widget panel", exact: true })
			.click();
		await open();
		const original = await editorValue();
		const candidate = {
			...JSON.parse(original),
			title: "Protected JSON draft",
		};
		const edited = JSON.stringify(candidate, null, 2);
		assert.equal(await unloadProtected(), false);

		await replaceValue(edited);
		assert.equal(await unloadProtected(), true);
		await close.click();
		await confirmation.waitFor();
		await page
			.getByRole("button", { name: "Keep editing", exact: true })
			.click();
		await confirmation.waitFor({ state: "hidden" });
		assert.equal(await editorValue(), edited);
		assert.equal(await unloadProtected(), true);
		await waitForFocus("Close JSON editor");

		await close.focus();
		await page.keyboard.press("Escape");
		await confirmation.waitFor();
		await page.keyboard.press("Escape");
		await confirmation.waitFor({ state: "hidden" });
		await dialog.waitFor();
		await waitForFocus("Close JSON editor");
		assert.equal(await editorValue(), edited);

		await page.mouse.click(5, 5);
		await confirmation.waitFor();
		await page
			.getByRole("button", { name: "Discard JSON edits", exact: true })
			.click();
		await dialog.waitFor({ state: "hidden" });
		await waitForFocus("Layout options");
		assert.equal(await unloadProtected(), false);
		assert.equal(
			await page.evaluate(() => window.homeQa.counters.saveAttempts),
			0,
		);
		assert.equal(
			await page.getByText("Editing layout", { exact: true }).count(),
			1,
		);

		await open();
		assert.equal(await editorValue(), original);
		await replaceValue(edited);
		await replaceValue(original);
		assert.equal(await unloadProtected(), false);
		await close.click();
		await dialog.waitFor({ state: "hidden" });
		await waitForFocus("Layout options");
		assert.equal(await confirmation.count(), 0);

		await open();
		await replaceValue(edited);
		await page
			.getByRole("button", { name: "Apply changes", exact: true })
			.click();
		await page.getByRole("button", { name: "Applied", exact: true }).waitFor();
		await close.click();
		await dialog.waitFor({ state: "hidden" });
		await waitForFocus("Layout options");
		assert.equal(await confirmation.count(), 0);
		assert.equal(
			await page.getByText("Unsaved changes", { exact: true }).count(),
			1,
		);
		await page.getByRole("button", { name: "Save", exact: true }).click();
		await page.locator('[data-home-editor][data-editing="false"]').waitFor();
		assert.equal(
			(await page.evaluate(() => window.homeQa.getSaved())).title,
			candidate.title,
		);
		assert.equal(await unloadProtected(), false);
	} finally {
		await context.close();
	}
}

try {
	await verify({ width: 1280, height: 900 });
	await verify({ width: 390, height: 844 });
	assert.deepEqual(errors, []);
	console.log("Home JSON dismissal checks passed on desktop and mobile.");
} finally {
	await browser.close();
}
