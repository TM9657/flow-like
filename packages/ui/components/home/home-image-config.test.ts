import { describe, expect, it } from "bun:test";
import { createHomeWidget } from "./catalog";
import {
	homeImageReference,
	isHomeStorageImagePath,
} from "./home-image-config";
import { parseHomeLayoutJson, serializeHomeLayout } from "./home-layout-json";

describe("Home image sources", () => {
	it("keeps existing URL images and root-relative images compatible", () => {
		for (const url of ["https://example.com/image.png", "/images/logo.svg"])
			expect(homeImageReference({ imageUrl: url })).toEqual({
				source: "url",
				url,
			});
		for (const imageUrl of [
			"javascript:alert(1)",
			"data:image/png;base64,eA==",
			"//example.com/a.png",
			"mailto:a@b.com",
		])
			expect(homeImageReference({ imageUrl })).toBeUndefined();
	});

	it("resolves only the selected source and requires an app-relative file path", () => {
		const config = {
			imageSource: "storage",
			imageAppId: "app-a",
			imagePath: "media/team photo.png",
			imageUrl: "https://example.com/old.png",
		};
		expect(homeImageReference(config)).toEqual({
			source: "storage",
			appId: "app-a",
			path: "media/team photo.png",
		});
		expect(homeImageReference({ ...config, imageAppId: "" })).toBeUndefined();
		expect(homeImageReference({ ...config, imagePath: "" })).toBeUndefined();
		expect(homeImageReference({ ...config, imageSource: "url" })).toEqual({
			source: "url",
			url: config.imageUrl,
		});
		for (const path of [
			"/media/a.png",
			"../a.png",
			"media/../a.png",
			"media//a.png",
			"media/",
			"storage://media/a.png",
			"https://example.com/a.png",
			"C:\\a.png",
			"media\na.png",
		])
			expect(isHomeStorageImagePath(path)).toBe(false);
	});

	it("round-trips durable storage references through saved layout JSON", () => {
		const widget = createHomeWidget("image-card");
		widget.config = {
			...widget.config,
			imageSource: "storage",
			imageAppId: "app-a",
			imagePath: "media/photo.webp",
			imageAlt: "Our workspace",
		};
		const layout = { version: 1 as const, widgets: [widget] };
		expect(parseHomeLayoutJson(serializeHomeLayout(layout))).toEqual({
			ok: true,
			layout,
		});
	});
});
