import { describe, expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { NATIVE_FONT_SHA256, hasNativeGlyphs } from "./font-coverage";

describe("native font coverage", () => {
	test("coverage remains paired with both shipped font files", () => {
		for (const [weight, file] of [
			["regular", "Inter-Regular.ttf"],
			["medium", "Inter-Medium.ttf"],
		] as const) {
			const bytes = readFileSync(
				new URL(`../../../board-canvas/assets/${file}`, import.meta.url),
			);
			expect(createHash("sha256").update(bytes).digest("hex")).toBe(
				NATIVE_FONT_SHA256[weight],
			);
		}
	});
	test("supported Latin, Greek and Cyrillic stay native, while shaped and missing scripts use DOM", () => {
		expect(hasNativeGlyphs("Résumé · Δοκιμή · Пример")).toBe(true);
		for (const label of [
			"工作流程",
			"ワークフロー",
			"워크플로",
			"مرحبا",
			"😀",
			"👩‍💻",
		]) {
			expect(hasNativeGlyphs(label)).toBe(false);
		}
	});
});
