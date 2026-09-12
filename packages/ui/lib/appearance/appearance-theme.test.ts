import { describe, expect, test } from "bun:test";
import {
	APPEARANCE_END,
	APPEARANCE_START,
	DEFAULT_APPEARANCE_STATE,
	buildAppearanceBlock,
	cloneAppearanceState,
	composeAppearanceSheet,
	contrastRatio,
	hexToOklch,
	oklchToHex,
	parseAppearanceSheet,
} from "./appearance-theme";

describe("colour conversion", () => {
	test("oklch round-trips back to the same hex", () => {
		for (const hex of ["#FF5A3C", "#101014", "#FFFFFF", "#000000", "#2A2A34"]) {
			expect(oklchToHex(hexToOklch(hex))).toBe(hex.toLowerCase());
		}
	});

	test("contrast matches known pairs", () => {
		expect(contrastRatio("#FFFFFF", "#000000")).toBeCloseTo(21, 1);
		expect(contrastRatio("#FFFFFF", "#FFFFFF")).toBeCloseTo(1, 5);
	});
});

describe("sheet round-trip", () => {
	test("a built sheet parses back to the same state", () => {
		const state = cloneAppearanceState(DEFAULT_APPEARANCE_STATE);
		state.radius = 18;
		state.spacing = 5;
		state.motion = 1.4;
		state.palette.dark.primary = "#7C3AED";
		state.effects.aurora = true;
		state.effects.bevel = true;
		state.effects.shimmer = false;

		const parsed = parseAppearanceSheet(buildAppearanceBlock(state));

		expect(parsed.managed).toBe(true);
		expect(parsed.state.radius).toBe(18);
		expect(parsed.state.spacing).toBe(5);
		expect(parsed.state.motion).toBeCloseTo(1.4, 2);
		expect(parsed.state.palette.dark.primary).toBe("#7c3aed");
		expect(parsed.state.palette.light.primary).toBe(
			DEFAULT_APPEARANCE_STATE.palette.light.primary.toLowerCase(),
		);
		expect(parsed.state.effects.aurora).toBe(true);
		expect(parsed.state.effects.bevel).toBe(true);
		expect(parsed.state.effects.shimmer).toBe(false);
	});

	test("rules after the end marker survive a rebuild", () => {
		const tail = ".fl-thing {\n  color: red;\n}";
		const sheet = composeAppearanceSheet(
			buildAppearanceBlock(DEFAULT_APPEARANCE_STATE),
			tail,
		);
		const parsed = parseAppearanceSheet(sheet);
		expect(parsed.tail).toBe(tail);
		expect(
			composeAppearanceSheet(buildAppearanceBlock(parsed.state), parsed.tail),
		).toContain(tail);
	});

	test("a sheet without markers is kept whole as the author's own rules", () => {
		const existing = "body {\n  background: black;\n}";
		const parsed = parseAppearanceSheet(existing);
		expect(parsed.managed).toBe(false);
		expect(parsed.tail).toBe(existing);
		expect(parsed.state).toEqual(DEFAULT_APPEARANCE_STATE);
	});

	test("hand-edited hex values inside the block are adopted", () => {
		const sheet = `${APPEARANCE_START}\n:root {\n  --primary: #00ff00;\n}\n${APPEARANCE_END}`;
		expect(parseAppearanceSheet(sheet).state.palette.light.primary).toBe(
			"#00ff00",
		);
	});

	test("the preview projection emits one palette and no media query", () => {
		const preview = buildAppearanceBlock(DEFAULT_APPEARANCE_STATE, {
			mode: "dark",
			boost: true,
		});
		expect(preview).not.toContain("prefers-color-scheme: dark");
		expect(preview).toContain(":root, :root.dark {");
		expect(preview).toContain(
			hexToOklch(DEFAULT_APPEARANCE_STATE.palette.dark.background),
		);
	});

	test("elevation and the inner highlight share one shadow rule", () => {
		const state = cloneAppearanceState(DEFAULT_APPEARANCE_STATE);
		state.effects.bevel = true;
		state.effects.elevation = true;
		const css = buildAppearanceBlock(state);
		expect(css.match(/\[data-slot="card"\] \{\n {2}box-shadow:/g)).toHaveLength(
			1,
		);
		expect(css).toContain("inset 0 1px 0");
		expect(css).toContain("0 10px 28px -16px");
	});

	test("motion effects carry the reduced-motion guard and the prefix", () => {
		const state = cloneAppearanceState(DEFAULT_APPEARANCE_STATE);
		state.effects.entrance = true;
		const css = buildAppearanceBlock(state);
		expect(css).toContain("@keyframes fl-appearance-rise");
		expect(css).toContain("prefers-reduced-motion: reduce");
	});
});
