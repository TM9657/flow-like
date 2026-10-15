import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { Window } from "happy-dom";
import { canvasColor, readCanvasTheme } from "./theme";

const tokens: Record<string, string> = {
	"--background": "#101214",
	"--card": "#202428",
	"--foreground": "#f0f4f8",
	"--muted-foreground": "#909498",
	"--border": "#404448",
	"--primary": "#ff6633",
	"--tertiary": "#9966cc",
};

function withHost(run: (host: HTMLElement) => void) {
	const window = new Window();
	Object.assign(window, { SyntaxError, TypeError, Error });
	const oldDocument = Object.getOwnPropertyDescriptor(globalThis, "document");
	const oldComputed = Object.getOwnPropertyDescriptor(
		globalThis,
		"getComputedStyle",
	);
	Object.defineProperty(globalThis, "document", {
		configurable: true,
		value: window.document,
	});
	Object.defineProperty(globalThis, "getComputedStyle", {
		configurable: true,
		value: window.getComputedStyle.bind(window),
	});
	try {
		const host = window.document.createElement("div");
		window.document.body.append(host);
		for (const [key, value] of Object.entries(tokens))
			host.style.setProperty(key, value);
		run(host as unknown as HTMLElement);
	} finally {
		if (oldDocument) Object.defineProperty(globalThis, "document", oldDocument);
		else Reflect.deleteProperty(globalThis, "document");
		if (oldComputed)
			Object.defineProperty(globalThis, "getComputedStyle", oldComputed);
		else Reflect.deleteProperty(globalThis, "getComputedStyle");
		window.close();
	}
}

describe("native canvas theme color conversion", () => {
	test("converts opaque OKLCH without a browser canvas or DOM", () => {
		expect(canvasColor("oklch(0 0 0)")).toEqual([0, 0, 0, 255]);
		expect(canvasColor("oklch(100% 0 0)")).toEqual([255, 255, 255, 255]);
		expect(canvasColor("oklch(62.7955% 0.257683 29.2339)")).toEqual([
			255, 0, 0, 255,
		]);
	});

	test("preserves OKLCH alpha as either a number or percentage", () => {
		expect(canvasColor("oklch(100% 0 0 / .5)")).toEqual([255, 255, 255, 128]);
		expect(canvasColor("oklch(0 0 0 / 50%)")).toEqual([0, 0, 0, 128]);
		expect(canvasColor("oklch(100% 0 0 / 0)")).toEqual([255, 255, 255, 0]);
	});

	test("resolves every direct canvas token in both shipped palettes", () => {
		const css = readFileSync(
			new URL("../../global.css", import.meta.url),
			"utf8",
		);
		const names = new Set([
			...Object.keys(tokens),
			"--pin-string",
			"--pin-integer",
			"--pin-float",
			"--pin-boolean",
			"--pin-struct",
			"--pin-geometry",
			"--pin-byte",
			"--pin-generic",
			"--pin-date",
			"--pin-pathbuf",
			"--pin-fn-ref",
		]);
		let checked = 0;
		for (const match of css.matchAll(/(--[\w-]+):\s*([^;]+);/g)) {
			if (!names.has(match[1]) || match[2].includes("var(")) continue;
			const rgba = canvasColor(match[2]);
			expect(
				rgba.every(
					(channel) =>
						Number.isInteger(channel) && channel >= 0 && channel <= 255,
				),
			).toBe(true);
			expect(rgba[3]).toBe(255);
			checked++;
		}
		expect(checked).toBeGreaterThanOrEqual(names.size * 2);
	});

	test("preserves short and long hex alpha and transparent colors", () => {
		expect(canvasColor("#abc")).toEqual([170, 187, 204, 255]);
		expect(canvasColor("#abcd")).toEqual([170, 187, 204, 221]);
		expect(canvasColor("#10203080")).toEqual([16, 32, 48, 128]);
		expect(canvasColor("transparent")).toEqual([0, 0, 0, 0]);
	});

	test("accepts legacy and modern RGB syntax with alpha and channel clamping", () => {
		expect(canvasColor("rgb(16, 32, 48)")).toEqual([16, 32, 48, 255]);
		expect(canvasColor("rgba(16, 32, 48, 0.5)")).toEqual([16, 32, 48, 128]);
		expect(canvasColor("rgb(16 32 48 / .25)")).toEqual([16, 32, 48, 64]);
		expect(canvasColor("rgb(-10 300 20 / 2)")).toEqual([0, 255, 20, 255]);
	});

	test("rounds RGB and alpha percentages to the nearest byte", () => {
		expect(canvasColor("rgb(100% 50% 0% / 50%)")).toEqual([255, 128, 0, 128]);
	});

	test("rejects missing or malformed colors instead of silently painting black", () => {
		for (const color of [
			"",
			"var(--missing)",
			"#12",
			"#12345",
			"rgb(1 2)",
			"rgba(1 2 3 / nope)",
		]) {
			expect(() => canvasColor(color)).toThrow();
		}
	});
});

describe("native canvas theme token bridge", () => {
	test("reads scoped node and reroute rounding from the production CSS utilities", () => {
		withHost((host) => {
			const originalComputed = globalThis.getComputedStyle;
			let nodeRadius = "4px";
			let rerouteRadius = "6px";
			Object.defineProperty(globalThis, "getComputedStyle", {
				configurable: true,
				value: (element: Element) => {
					if (
						element.parentElement === host &&
						element.className.startsWith("rounded-")
					)
						return {
							borderTopLeftRadius:
								element.className === "rounded-md" ? nodeRadius : rerouteRadius,
						};
					return originalComputed(element);
				},
			});
			// Happy DOM does not resolve CSS radius calc/rem values. Supply that browser boundary.
			expect(readCanvasTheme(host)).toMatchObject({
				nodeRadius: 4,
				rerouteRadius: 6,
			});
			nodeRadius = "10px";
			rerouteRadius = "12px";
			expect(readCanvasTheme(host)).toMatchObject({
				nodeRadius: 10,
				rerouteRadius: 12,
			});
			nodeRadius = "0px";
			rerouteRadius = "0px";
			expect(readCanvasTheme(host)).toMatchObject({
				nodeRadius: 0,
				rerouteRadius: 0,
			});
			expect(host.childElementCount).toBe(0);
		});
	});
	test("reads rem sizing from the root and retains unsupported font families in DOM", () => {
		withHost((host) => {
			host.ownerDocument.documentElement.style.fontSize = "20px";
			host.style.fontSize = "12px";
			host.style.fontFamily = '"Inter", sans-serif';
			expect(readCanvasTheme(host).rootFontSize).toBe(20);
			expect(readCanvasTheme(host).nativeFontCompatible).toBe(true);
			host.style.fontFamily = 'Georgia, "Inter", serif';
			expect(readCanvasTheme(host).nativeFontCompatible).toBe(false);
		});
	});
	test("uses browser sRGB readback for computed lab, lch and color values", () => {
		for (const resolved of [
			"lab(3.94232 0.0860766 -2.90141)",
			"lch(50 20 90)",
			"color(display-p3 1 0 0 / .5)",
		]) {
			withHost((host) => {
				const document = host.ownerDocument;
				const originalCreate = document.createElement.bind(document);
				const originalComputed = globalThis.getComputedStyle;
				const context = {
					fillStyle: "",
					clearRect() {},
					fillRect() {},
					getImageData: () => ({
						data: new Uint8ClampedArray([12, 24, 36, 128]),
					}),
				};
				Object.defineProperty(document, "createElement", {
					configurable: true,
					value: (name: string) =>
						name === "canvas"
							? { width: 0, height: 0, getContext: () => context }
							: originalCreate(name),
				});
				Object.defineProperty(globalThis, "getComputedStyle", {
					configurable: true,
					value: (element: Element) =>
						element === host ? originalComputed(element) : { color: resolved },
				});
				// Happy DOM does not implement Color 4; the computed-style and Canvas2D boundary are supplied here.
				host.style.setProperty("--card", "rebeccapurple");
				expect(readCanvasTheme(host).palette.card).toEqual([12, 24, 36, 128]);
				expect(context.fillStyle).toBe(resolved);
				expect(host.childElementCount).toBe(0);
			});
		}
	});
	test("uses host-scoped tokens with execution and optional-pin fallbacks", () => {
		withHost((host) => {
			host.style.setProperty("--pin-string", "#e95da1");
			host.style.setProperty("--pin-fn-ref", "#dc7916");
			const theme = readCanvasTheme(host);
			expect(theme.palette.background).toEqual([16, 18, 20, 255]);
			expect(theme.pinColors.String).toEqual([233, 93, 161, 255]);
			expect(theme.pinColors.Execution).toEqual(theme.palette.foreground);
			expect(theme.pinColors.HashMap).toEqual(theme.palette.foreground);
			expect(theme.referenceColor).toEqual([220, 121, 22, 255]);
			expect(host.childElementCount).toBe(0);
		});
	});

	test("reads changed global styles on the next call and removes its probe", () => {
		withHost((host) => {
			expect(readCanvasTheme(host).palette.primary).toEqual([
				255, 102, 51, 255,
			]);
			host.style.setProperty("--primary", "#22aacc");
			const theme = readCanvasTheme(host);
			expect(theme.palette.primary).toEqual([34, 170, 204, 255]);
			expect(theme.referenceColor).toEqual(theme.palette.tertiary);
			expect(host.childElementCount).toBe(0);
		});
	});

	test("a missing required token fails visibly and cleans up the probe", () => {
		withHost((host) => {
			host.style.removeProperty("--background");
			expect(() => readCanvasTheme(host)).toThrow(
				"Missing canvas theme token --background",
			);
			expect(host.childElementCount).toBe(0);
		});
	});
});
