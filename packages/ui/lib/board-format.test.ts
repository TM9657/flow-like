import { describe, expect, test } from "bun:test";
import { boardFormatVersion } from "./board-format";

describe("board format advertisements", () => {
	test("accepts newer format versions without a feature-specific allowlist", () => {
		expect(boardFormatVersion(1)).toBe(1);
		expect(boardFormatVersion(2)).toBe(2);
		expect(boardFormatVersion(3)).toBe(3);
		expect(boardFormatVersion(0xffff_ffff)).toBe(0xffff_ffff);
	});

	test.each(
		[
			undefined,
			null,
			"2",
			{},
			[],
			0,
			-1,
			1.5,
			Number.NaN,
			Number.POSITIVE_INFINITY,
			2 ** 32,
		].map((value) => [value]),
	)("keeps the legacy format for an invalid advertisement: %p", (value) =>
		expect(boardFormatVersion(value)).toBe(1),
	);
});
