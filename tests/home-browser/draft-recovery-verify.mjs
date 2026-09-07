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

const candidate = (title) => ({
	version: 1,
	title,
	widgets: [
		{
			id: "retained-message",
			type: "information",
			title,
			size: { columns: 6, rows: 3 },
			appearance: { variant: "card", accent: "blue" },
			config: { mode: "markdown", body: title },
		},
	],
});

async function verify(admin, releaseBeforeEdit) {
	const context = await browser.newContext({
		viewport: { width: 1280, height: 900 },
	});
	const page = await context.newPage();
	page.on("pageerror", (error) => errors.push(error.stack ?? error.message));
	await page.route("**/*", (route) => {
		const url = new URL(route.request().url());
		return [allowedHost, "cdn.jsdelivr.net"].includes(url.hostname) ||
			["data:", "blob:"].includes(url.protocol)
			? route.continue()
			: route.abort();
	});
	const saveLabel = admin ? "Publish" : "Save";
	const editLabel = admin ? "Edit default" : "Customize";
	const initial = candidate("First applied JSON");
	const newer = candidate("Newer recovered JSON");
	const published = candidate("Published revision two");
	const dialog = page.getByRole("dialog", { name: "Home layout JSON" });
	const widgetTitle = (title) =>
		page
			.locator('[data-home-widget="retained-message"]')
			.getByText(title, { exact: true })
			.first();
	const applyJson = async (layout) => {
		await page
			.getByRole("button", { name: "Layout options", exact: true })
			.click();
		await page
			.getByRole("menuitem", { name: "Edit JSON", exact: true })
			.click();
		await dialog.waitFor();
		await page.waitForFunction(
			() => globalThis.monaco?.editor.getEditors().length > 0,
			undefined,
			{ timeout: 60_000 },
		);
		await page.evaluate(
			() =>
				new Promise((resolve) =>
					requestAnimationFrame(() => requestAnimationFrame(resolve)),
				),
		);
		await page.evaluate(
			(source) => globalThis.monaco.editor.getEditors().at(-1).setValue(source),
			JSON.stringify(layout),
		);
		await page.evaluate(
			() =>
				new Promise((resolve) =>
					requestAnimationFrame(() => requestAnimationFrame(resolve)),
				),
		);
		await page
			.getByRole("button", { name: "Apply changes", exact: true })
			.click();
		await page.getByRole("button", { name: "Applied", exact: true }).waitFor();
		await page.getByRole("button", { name: "Close JSON editor" }).click();
		await dialog.waitFor({ state: "hidden" });
		await widgetTitle(layout.title).waitFor();
	};
	const remount = async () => {
		await page.evaluate(() => window.homeQa.remount());
		await page
			.getByRole("button", { name: "Resume editing", exact: true })
			.waitFor();
	};
	const resume = async () => {
		await customizeHome(page, "Resume editing");
		await page.waitForFunction(() => window.homeQa.editing?.editing === true);
		assert.equal(
			await page.evaluate(() => window.homeQa.editing.baseRevision),
			"r1",
			"A recovered draft must retain its original revision, not adopt a new publication.",
		);
	};
	const releaseSave = async () => {
		await page.evaluate(() => {
			window.homeQa.holdSave = false;
			window.homeQa.releaseSave();
		});
		await page.waitForFunction(() => window.homeQa.counters.saves === 1);
		await page.evaluate(
			() => new Promise((resolve) => requestAnimationFrame(resolve)),
		);
	};
	try {
		await page.goto(`${origin}/?revision=r1${admin ? "&admin=1" : ""}`, {
			waitUntil: "domcontentloaded",
		});
		await customizeHome(page, editLabel, { timeout: 60_000 });
		await applyJson(initial);
		await page.evaluate(() => {
			window.homeQa.holdSave = true;
		});
		await page.getByRole("button", { name: saveLabel, exact: true }).click();
		await page.waitForFunction(
			() => typeof window.homeQa.releaseSave === "function",
		);

		await page.evaluate((value) => {
			window.homeQa.setRevision("r2");
			window.homeQa.replacePublishedLayout(value);
		}, published);
		await remount();
		await resume();
		await widgetTitle(initial.title).waitFor();

		if (releaseBeforeEdit) {
			await releaseSave();
			await remount();
			await resume();
			await widgetTitle(initial.title).waitFor();
		}
		await applyJson(newer);
		if (!releaseBeforeEdit) await releaseSave();
		assert.equal(
			(await page.evaluate(() => window.homeQa.getSaved())).title,
			initial.title,
		);
		await remount();
		await resume();
		await widgetTitle(newer.title).waitFor();
		assert.equal(
			await page.evaluate(() => window.homeQa.counters.saveAttempts),
			1,
		);
		assert.equal(
			await page.getByText("Unsaved changes", { exact: true }).count(),
			1,
		);

		await page.getByRole("button", { name: "Cancel", exact: true }).click();
		await page
			.getByRole("button", { name: "Discard changes", exact: true })
			.click();
		await page.getByRole("button", { name: editLabel, exact: true }).waitFor();
		await page.evaluate(() => window.homeQa.setRevision("r3"));
		await customizeHome(page, editLabel);
		await page.waitForFunction(() => window.homeQa.editing?.editing === true);
		assert.equal(
			await page.evaluate(() => window.homeQa.editing.baseRevision),
			"r3",
		);
		await page.getByRole("button", { name: "Cancel", exact: true }).click();
		await page.getByRole("button", { name: editLabel, exact: true }).waitFor();
		console.log(
			`${admin ? "Admin" : "Personal"} draft recovery passed (${releaseBeforeEdit ? "resume owns cache before edit" : "new JSON before late save"}).`,
		);
	} finally {
		await context.close();
	}
}

try {
	for (const admin of [false, true]) {
		await verify(admin, false);
		await verify(admin, true);
	}
	assert.deepEqual(
		errors.filter(
			(error) =>
				!error.includes("Uncaught (in promise) Canceled: Canceled") ||
				!error.includes("monaco-editor"),
		),
		[],
	);
	console.log("Home draft recovery and pinned revision checks passed.");
} finally {
	await browser.close();
}
