import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { chromium } from "playwright-core";

const output =
	process.env.FORECAST_SCREENSHOT_DIR || "/tmp/flow-like-forecast-screenshots";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
	executablePath:
		process.env.CHROME_EXECUTABLE_PATH ||
		(process.platform === "darwin"
			? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
			: undefined),
	headless: true,
	args: ["--no-sandbox"],
});
const page = await browser.newPage({
	viewport: { width: 1440, height: 1100 },
	deviceScaleFactor: 1,
});
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
page.on("console", (message) => {
	if (message.type() === "error") errors.push(message.text());
});
await page.route("**/*", (route) => {
	const url = new URL(route.request().url());
	return url.hostname === "127.0.0.1" ||
		["data:", "blob:"].includes(url.protocol)
		? route.continue()
		: route.abort();
});
const capture = async (name) => {
	await page.screenshot({
		path: `${output}/${name}.png`,
		fullPage: true,
		animations: "disabled",
	});
	if (/^0[1-4]-/.test(name))
		await page
			.getByRole("region", { name: "Forecast setup", exact: true })
			.screenshot({
				path: `${output}/wizard-${name}.png`,
				animations: "disabled",
			});
};
const fits = async () => {
	const sizes = await page.evaluate(() => ({
		viewport: innerWidth,
		page: document.documentElement.scrollWidth,
	}));
	assert.ok(sizes.page <= sizes.viewport + 1, JSON.stringify(sizes));
};
const select = async (name, option) => {
	await page.getByRole("combobox", { name, exact: true }).click();
	await page.getByRole("option", { name: option, exact: true }).click();
};
const step = async (title, index) => {
	await page
		.getByRole("button", { name: `Step ${index}: ${title}`, exact: true })
		.click();
	await page
		.getByRole("heading", { name: `Step ${index} of 4: ${title}` })
		.waitFor();
};

try {
	await page.goto("http://127.0.0.1:4344/", { waitUntil: "networkidle" });
	await page.getByText("4 columns available", { exact: true }).waitFor();
	await fits();
	await capture("01-source");
	await page
		.getByRole("button", { name: "Next: Columns", exact: true })
		.click();
	await select("Series ID", "machine_id");
	const load = page.getByRole("button", { name: "load_pct", exact: true });
	if ((await load.getAttribute("aria-pressed")) === "true") await load.click();
	await fits();
	await capture("02-columns-per-machine");
	await page.getByRole("button", { name: "Next: Model", exact: true }).click();
	await page
		.getByRole("switch", { name: "Evaluate on held-out data", exact: true })
		.click();
	await page
		.getByRole("spinbutton", { name: "Holdout steps", exact: true })
		.fill("12");
	await fits();
	await capture("03-model-and-evaluation");
	await page.getByRole("button", { name: "Next: Review", exact: true }).click();
	await fits();
	await capture("04-review");
	await page.getByRole("button", { name: "Run forecast", exact: true }).click();
	await page.getByText(/Forecast ready with 72 predictions/).waitFor();
	await select("Chart series", "Machine B");
	await page.getByRole("button", { name: "Outlook", exact: true }).click();
	await fits();
	await capture("05-grouped-forecast");
	await page
		.getByRole("region", { name: "Forecast visualization", exact: true })
		.screenshot({
			path: `${output}/05-grouped-chart-detail.png`,
			animations: "disabled",
		});
	await page.getByRole("button", { name: "Holdout", exact: true }).click();
	const slider = page.getByRole("slider", {
		name: "Holdout forecast as of",
		exact: true,
	});
	await slider.focus();
	await slider.press("ArrowLeft");
	await page.waitForFunction(
		() =>
			window.forecastingQa.calls.filter((call) => call.method === "forecast")
				.length === 2,
	);
	await page.getByText(/Forecast ready with 72 predictions/).waitFor();
	await fits();
	await capture("06-historical-evaluation");
	await page
		.getByRole("region", { name: "Forecast visualization", exact: true })
		.screenshot({
			path: `${output}/06-historical-chart-detail.png`,
			animations: "disabled",
		});
	const forecastCalls = await page.evaluate(() =>
		window.forecastingQa.calls.filter((call) => call.method === "forecast"),
	);
	assert.equal(forecastCalls[0].payload.mapping.series_id, "machine_id");
	assert.deepEqual(forecastCalls[0].payload.mapping.targets, ["temperature_c"]);
	assert.equal(forecastCalls[1].payload.backtest_offset, 1);
	await page.setViewportSize({ width: 390, height: 844 });
	await fits();
	await capture("07-mobile-evaluation");
	await page.getByRole("button", { name: "Edit setup", exact: true }).click();
	await step("Columns", 2);
	await fits();
	await capture("08-mobile-columns");
	await page.setViewportSize({ width: 1440, height: 1100 });
	await step("Model", 3);
	await select("Forecasting model", "IBM Granite TinyTimeMixer");
	await page.getByText("Point forecast only", { exact: true }).waitFor();
	await capture("09-point-only-model");
	await page
		.getByRole("button", { name: "New prediction", exact: true })
		.click();
	await page.getByText("4 columns available", { exact: true }).waitFor();
	await page.getByRole("button", { name: "SQL query", exact: true }).click();
	await page.waitForFunction(
		() => window.monaco?.editor.getModels().length > 0,
	);
	// Monaco creates its model before React attaches the change subscription.
	await page.waitForTimeout(300);
	await page.evaluate(() =>
		window.monaco.editor
			.getModels()[0]
			.setValue(
				'SELECT timestamp, AVG(temperature_c) AS temperature_c\nFROM "sensor_readings"\nGROUP BY timestamp\nORDER BY timestamp',
			),
	);
	await page.getByText("Source needs a preview", { exact: true }).waitFor();
	await page
		.getByRole("button", { name: "Preview columns", exact: true })
		.click();
	await page.getByText("2 columns available", { exact: true }).waitFor();
	await fits();
	await capture("10-overall-source-sql");
	await page
		.getByRole("region", { name: "Forecast setup", exact: true })
		.screenshot({
			path: `${output}/wizard-10-overall-source-sql.png`,
			animations: "disabled",
		});
	await step("Columns", 2);
	assert.equal(
		await page
			.getByRole("combobox", { name: "Series ID", exact: true })
			.innerText(),
		"Single series",
	);
	await capture("11-overall-columns");
	assert.deepEqual(errors, []);
	console.log(
		`Forecast browser checks passed: source, grouped mapping, model, review, grouped forecast, historical cutoff rerun, point-only model, overall SQL preview, and desktop/mobile layouts. Screenshots: ${output}`,
	);
} catch (error) {
	console.error("Browser errors:", errors);
	await capture("failure");
	throw error;
} finally {
	await browser.close();
}
