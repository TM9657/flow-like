import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { paletteColor } from "./palette";

describe("board palette bridge", () => {
	test("resolves every current board token without browser pixel reads", () => {
		const css = readFileSync(
			new URL("../../packages/ui/global.css", import.meta.url),
			"utf8",
		);
		const tokens = [
			...css.matchAll(
				/--(?:background|card|foreground|border|primary|tertiary|pin-string|pin-float|pin-integer):\s*(oklch\([^;]+\));/g,
			),
		];
		expect(tokens.length).toBe(18);
		for (const [, value] of tokens) {
			const channels = paletteColor(value);
			expect(channels[3]).toBe(255);
			expect(channels.slice(0, 3).some((channel) => channel > 0)).toBe(true);
		}
		expect(paletteColor("oklch(0.164 0.0111 268.0057)")).toEqual([
			12, 14, 19, 255,
		]);
	});

	test("rejects missing tokens instead of silently drawing black", () => {
		expect(() => paletteColor("")).toThrow("Unsupported board palette colour");
		expect(paletteColor("#12345680")).toEqual([18, 52, 86, 128]);
	});
});
