import assert from "node:assert/strict";
import { chromium } from "playwright-core";
import { customizeHome } from "./customize-home.mjs";

const origin = process.argv[2] ?? "http://127.0.0.1:4329";
const requestedScenario = process.argv[3];
const browser = await chromium.launch({
	executablePath:
		process.env.CHROME_EXECUTABLE_PATH ||
		(process.platform === "darwin"
			? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
			: undefined),
	headless: true,
	args: ["--disable-dev-shm-usage", "--no-sandbox"],
});

const candidate = (title) => ({
	version: 1,
	title,
	widgets: [
		{
			id: "saved-message",
			type: "information",
			title,
			size: { columns: 6, rows: 3 },
			appearance: { variant: "card", accent: "neutral" },
			config: { mode: "markdown", body: title },
		},
	],
});

async function verify(mode) {
	const context = await browser.newContext({
		viewport: { width: 1280, height: 900 },
	});
	const original = candidate("Original home");
	const changed = candidate(`Changed home: ${mode}`);
	const profile = {
		id: "save-integration-profile",
		name: "Save integration profile",
		hub: "https://api.flow-like.com",
		user_id: "save-integration-user",
		bit_ids: [],
		apps: [],
		created_at: "2026-09-07T10:00:00Z",
		updated_at: "2026-09-07T10:00:00Z",
		...(mode === "legacy" ? {} : { home_layout: original }),
	};
	let writes = 0;
	let reads = 0;
	let staleReadPending = false;
	let acceptsSaves = mode === "modern" || mode === "stale";
	const errors = [];
	const routeRequest = async (route) => {
		const request = route.request();
		const url = new URL(request.url());
		if (url.pathname.startsWith("/api/v1/")) {
			const path = url.pathname.slice("/api/v1/".length);
			const respond = (body) =>
				route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify(body),
				});
			if (path === "profile" && request.method() === "GET") {
				reads++;
				if (staleReadPending) {
					staleReadPending = false;
					return respond([
						{
							...profile,
							home_layout: original,
							updated_at: "2026-09-07T10:00:00Z",
						},
					]);
				}
				return respond([profile]);
			}
			if (path === `profile/${profile.id}` && request.method() === "POST") {
				writes++;
				if (acceptsSaves) {
					profile.home_layout = request.postDataJSON().home_layout;
					profile.updated_at = "2026-09-07T10:01:00Z";
					staleReadPending = mode === "stale";
				}
				return respond({ profile });
			}
			if (path === "info/home-defaults")
				return respond({
					main: { id: "main", revision: "r1", layout: original },
					profile: null,
				});
			if (path === "user/info")
				return respond({ name: "Fixture user", tier: "FREE" });
			throw new Error(`Unexpected fixture API: ${request.method()} ${path}`);
		}
		if (url.origin === origin || ["data:", "blob:"].includes(url.protocol))
			return route.continue();
		return route.abort();
	};
	await context.route("**/*", routeRequest);
	const openPage = async (targetContext = context) => {
		const page = await targetContext.newPage();
		page.on("pageerror", (error) => errors.push(error.message));
		page.on("dialog", (dialog) => dialog.accept());
		await page.goto(`${origin}/save-integration.html`, {
			waitUntil: "domcontentloaded",
		});
		await page.waitForFunction(
			() =>
				window.saveIntegrationQa?.snapshot()?.defaultLayout.title ===
				"Original home",
			undefined,
			{ timeout: 60_000 },
		);
		return page;
	};
	const snapshot = (page) =>
		page.evaluate(() =>
			JSON.parse(JSON.stringify(window.saveIntegrationQa.snapshot())),
		);
	const save = async (page) => {
		const response = page.waitForResponse(
			(response) =>
				response.request().method() === "POST" &&
				new URL(response.url()).pathname === `/api/v1/profile/${profile.id}`,
		);
		await page.getByRole("button", { name: "Save", exact: true }).click();
		await response;
		await page.waitForFunction(
			() =>
				!window.saveIntegrationQa.snapshot().editing ||
				Array.from(document.querySelectorAll("button")).some(
					(button) => button.textContent === "Save" && !button.disabled,
				),
		);
		await page.evaluate(
			() =>
				new Promise((resolve) =>
					requestAnimationFrame(() => requestAnimationFrame(resolve)),
				),
		);
	};
	try {
		const page = await openPage();
		await customizeHome(page);
		assert.equal(
			(
				await page.evaluate(
					(value) => window.saveIntegrationQa.stage(value),
					changed,
				)
			).status,
			"staged",
		);
		await save(page);
		assert.equal(writes, 1);
		if (mode === "legacy" || mode === "ignored") {
			assert.equal(
				(await snapshot(page)).editing,
				true,
				"An ignored HTTP 200 must keep the editor open and its draft recoverable",
			);
			assert.deepEqual((await snapshot(page)).layout, changed);
			assert.equal((await snapshot(page)).dirty, true);
			assert.equal(
				await page.getByText("Your home is saved", { exact: true }).count(),
				0,
			);
			await page.reload({ waitUntil: "domcontentloaded" });
			await customizeHome(page, "Resume editing");
			assert.deepEqual(
				(await snapshot(page)).layout,
				changed,
				"A refused legacy save must survive reload",
			);
			acceptsSaves = true;
			await save(page);
			assert.equal(
				(await snapshot(page)).editing,
				false,
				"The recovered draft must save after the server supports layouts",
			);
			assert.deepEqual(profile.home_layout, changed);
		} else {
			let expectedSaved = changed;
			assert.equal((await snapshot(page)).editing, false);
			assert.deepEqual(
				(await snapshot(page)).layout,
				changed,
				"A stale follow-up read must not replace the accepted layout",
			);
			if (mode === "stale") {
				staleReadPending = true;
				await page.evaluate(() => window.saveIntegrationQa.refresh());
				await page.evaluate(
					() =>
						new Promise((resolve) =>
							requestAnimationFrame(() => requestAnimationFrame(resolve)),
						),
				);
				assert.deepEqual(
					(await snapshot(page)).layout,
					changed,
					"A later background refresh must not roll back the acknowledged revision",
				);
				expectedSaved = candidate("Changed on another device");
				profile.home_layout = expectedSaved;
				profile.updated_at = "2026-09-07T10:02:00Z";
				await page.evaluate(() => window.saveIntegrationQa.refresh());
				await page.waitForFunction(
					(title) => window.saveIntegrationQa.snapshot().layout.title === title,
					expectedSaved.title,
				);
				assert.deepEqual(
					(await snapshot(page)).layout,
					expectedSaved,
					"Newer changes from another device must still arrive",
				);
			}
			staleReadPending = false;
			await page.reload({ waitUntil: "domcontentloaded" });
			await page.waitForFunction(() =>
				Boolean(window.saveIntegrationQa?.snapshot()),
			);
			assert.deepEqual(
				(await snapshot(page)).layout,
				expectedSaved,
				"Saved home must survive an actual page reload",
			);
			if (mode === "modern") {
				await customizeHome(page);
				await page
					.getByRole("button", { name: "Layout options", exact: true })
					.click();
				await page
					.getByRole("menuitem", { name: "Reset to default", exact: true })
					.click();
				await page
					.getByRole("button", { name: "Reset draft", exact: true })
					.click();
				await save(page);
				assert.equal(
					profile.home_layout,
					null,
					"Following the default requires the server to store explicit null",
				);
				expectedSaved = original;
				assert.deepEqual((await snapshot(page)).layout, expectedSaved);
				await page.reload({ waitUntil: "domcontentloaded" });
				await page.waitForFunction(
					(title) =>
						window.saveIntegrationQa?.snapshot()?.layout.title === title,
					expectedSaved.title,
				);
				assert.deepEqual(
					(await snapshot(page)).layout,
					expectedSaved,
					"The server-confirmed reset must survive reload",
				);
			}
			const secondContext = await browser.newContext({
				viewport: { width: 1280, height: 900 },
			});
			try {
				await secondContext.route("**/*", routeRequest);
				const secondPage = await openPage(secondContext);
				assert.deepEqual(
					(await snapshot(secondPage)).layout,
					expectedSaved,
					"An independent browser session must read the saved server layout",
				);
			} finally {
				await secondContext.close();
			}
		}
		assert.deepEqual(errors, []);
		console.log(
			`Real HomePage, HomeEditor, WebUserState passed: ${mode} (${writes} writes, ${reads} reads)`,
		);
	} finally {
		await context.close();
	}
}

try {
	for (const mode of requestedScenario
		? [requestedScenario]
		: ["legacy", "ignored", "modern", "stale"])
		await verify(mode);
} finally {
	await browser.close();
}
