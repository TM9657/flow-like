import assert from "node:assert/strict";
import { chromium } from "playwright-core";
import { customizeHome } from "./customize-home.mjs";

const browser = await chromium.launch({
	executablePath:
		process.env.CHROME_EXECUTABLE_PATH ||
		(process.platform === "darwin"
			? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
			: undefined),
	headless: true,
	args: ["--disable-dev-shm-usage", "--no-sandbox"],
});
const page = await browser.newPage({ viewport: { width: 1480, height: 1050 } });
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
await page.route("**/*", async (route) => {
	const url = new URL(route.request().url());
	if (url.pathname.startsWith("/home-fixture-images/")) {
		return route.fulfill({
			contentType: "image/svg+xml",
			body: '<svg xmlns="http://www.w3.org/2000/svg" width="640" height="240"><rect width="640" height="240" fill="#0369a1"/><text x="32" y="130" font-size="32" fill="white">Home image fixture</text></svg>',
		});
	}
	if (url.hostname === "127.0.0.1" || ["data:", "blob:"].includes(url.protocol))
		return route.continue();
	return route.abort();
});

const waitForImage = async () => {
	await page.waitForFunction(() => {
		const image = document.querySelector('img[alt="Team workspace"]');
		return image?.complete && image.naturalWidth === 640;
	});
};

try {
	await page.goto("http://127.0.0.1:4318/?persist", {
		waitUntil: "domcontentloaded",
	});
	await customizeHome(page);
	await page
		.getByRole("button", { name: "Add Image and caption", exact: true })
		.click();
	await page.getByLabel("Title", { exact: true }).fill("Workspace image");
	await page
		.getByLabel("Image URL", { exact: true })
		.fill("/home-fixture-images/url/team.svg");
	await page
		.getByLabel("Image description", { exact: true })
		.fill("Team workspace");
	await waitForImage();
	await page
		.getByLabel("Image source", { exact: true })
		.selectOption("storage");
	await page
		.getByRole("checkbox", { name: "Knowledge Chat Library", exact: true })
		.check();
	await page
		.getByRole("button", { name: "Open folder media", exact: true })
		.click();
	assert.equal(
		await page.getByRole("button", { name: "Select image notes.txt" }).count(),
		0,
	);
	await page
		.getByRole("button", { name: "Select image team.svg", exact: true })
		.click();
	await waitForImage();
	assert.equal(
		await page.getByLabel("Selected image", { exact: true }).innerText(),
		"media/team.svg",
	);
	assert.match(
		await page.getByAltText("Team workspace").getAttribute("src"),
		/^http:\/\/127\.0\.0\.1:4318\/home-fixture-images\/fixture-app-0\/media\/team\.svg\?/,
	);
	await page.screenshot({
		path: "/tmp/home-image-settings.png",
		fullPage: true,
	});

	// Choosing another app must clear the first app's path before any new load.
	await page
		.getByRole("checkbox", { name: "Invoice OCR Library", exact: true })
		.check();
	assert.equal(
		await page.getByLabel("Selected image", { exact: true }).count(),
		0,
	);
	await page
		.getByRole("button", { name: "Open folder media", exact: true })
		.waitFor();
	await page
		.getByRole("checkbox", { name: "Knowledge Chat Library", exact: true })
		.check();
	await page
		.getByRole("button", { name: "Open folder media", exact: true })
		.click();
	await page
		.getByRole("button", { name: "Select image team.svg", exact: true })
		.click();
	await waitForImage();
	await page.getByRole("button", { name: "Save", exact: true }).click();
	await page.locator('[data-home-editor][data-editing="false"]').waitFor();
	await waitForImage();
	const config = await page.evaluate(
		() =>
			window.homeQa
				.getSaved()
				.widgets.find((widget) => widget.title === "Workspace image").config,
	);
	assert.equal(config.imageSource, "storage");
	assert.equal(config.imageAppId, "fixture-app-0");
	assert.equal(config.imagePath, "media/team.svg");
	assert.equal(config.imageUrl, "/home-fixture-images/url/team.svg");
	assert.ok(!JSON.stringify(config).includes("blob:"));
	const savedImageSrc = await page
		.getByAltText("Team workspace")
		.getAttribute("src");
	await page.reload({ waitUntil: "domcontentloaded" });
	await waitForImage();
	assert.equal(
		await page.getByAltText("Team workspace").getAttribute("src"),
		savedImageSrc,
		"Reload should resolve the saved app and path to its image URL",
	);
	assert.equal(await page.evaluate(() => window.homeQa.images.downloads), 1);
	assert.equal(await page.evaluate(() => window.homeQa.images.listings), 0);

	// Loading another app exercises denied access without reusing a signed URL.
	await page.evaluate(() => {
		window.homeQa.images.denied = true;
		const layout = structuredClone(window.homeQa.getSaved());
		layout.widgets.find(
			(widget) => widget.title === "Workspace image",
		).config.imageAppId = "fixture-app-1";
		window.homeQa.replacePublishedLayout(layout);
	});
	await page
		.getByText(
			"This image could not load. Check your connection and access to the app.",
			{ exact: true },
		)
		.waitFor();
	assert.equal(await page.getByAltText("Team workspace").count(), 0);
	await page.evaluate(() => {
		window.homeQa.images.denied = false;
	});
	await page.getByRole("button", { name: "Try again", exact: true }).click();
	await waitForImage();
	assert.equal(await page.evaluate(() => window.homeQa.images.listings), 0);
	await page.setViewportSize({ width: 390, height: 844 });
	await page.screenshot({ path: "/tmp/home-image-mobile.png", fullPage: true });
	assert.equal(
		await page.evaluate(
			() => document.documentElement.scrollWidth > innerWidth,
		),
		false,
	);
	assert.deepEqual(errors, []);
	console.log(
		"Home image browser checks passed: URL, storage picker, app switch, save/reload resolution, denied access, retry, mobile layout.",
	);
} finally {
	await browser.close();
}
