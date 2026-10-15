export async function setFixtureRenderer(page, renderer) {
	const settings = page.getByTestId("fixture-renderer-settings");
	const summary = settings.locator("summary");
	if ((await settings.getAttribute("open")) === null) await summary.click();
	const toggle = settings.getByRole("switch", {
		name: "Use WASM board (preview)",
	});
	const checked = renderer === "egui";
	if ((await toggle.getAttribute("aria-checked")) !== String(checked))
		await toggle.click();
	await summary.click();
	await page.waitForFunction(
		(expected) =>
			document
				.querySelector("[data-board-renderer]")
				?.getAttribute("data-board-renderer") === expected,
		renderer,
	);
}
