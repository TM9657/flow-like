export async function customizeHome(page, name = "Customize", options = {}) {
	const controls = page.locator("[data-home-controls]");
	await controls.waitFor({ timeout: options.timeout ?? 60_000 });
	const bounds = await controls.boundingBox();
	if (!bounds) throw new Error("Home controls have no visible bounds");
	await page.mouse.move(
		bounds.x + bounds.width / 2,
		bounds.y + bounds.height / 2,
	);
	await page.getByRole("button", { name, exact: true }).click(options);
}
