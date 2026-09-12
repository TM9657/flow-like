/**
 * The model behind the Appearance editor: a palette, a few shape tokens and a set of
 * effects, plus the CSS those produce and the parser that reads them back.
 *
 * The sheet is scoped to the app root at runtime (`:root` is rewritten to
 * `[data-app-id="…"]`), so two rules follow from that and are not negotiable here:
 * the sheet can never see the document's theme class, which is why dark values ride on
 * `prefers-color-scheme`; and `@keyframes` names stay global, which is why every one of
 * them carries the `fl-appearance-` prefix.
 */

export const APPEARANCE_START =
	"/* @flow-like appearance:start — managed by the Appearance editor */";
export const APPEARANCE_END = "/* @flow-like appearance:end */";

export type AppearanceMode = "light" | "dark";

export type AppearanceColorKey =
	| "background"
	| "foreground"
	| "card"
	| "border"
	| "mutedForeground"
	| "primary"
	| "primaryForeground";

export type AppearancePalette = Record<AppearanceColorKey, string>;

export type AppearanceEffectId =
	| "glass"
	| "elevation"
	| "bevel"
	| "hairline"
	| "wash"
	| "grain"
	| "glow"
	| "lift"
	| "press"
	| "entrance"
	| "sheen"
	| "aurora"
	| "focus"
	| "shimmer"
	| "rows";

export interface AppearanceState {
	palette: Record<AppearanceMode, AppearancePalette>;
	/** Corner radius in px; written as rem. */
	radius: number;
	/** Tailwind's spacing step in px; every gap and padding in the app scales with it. */
	spacing: number;
	fontSans: string;
	/** Duration multiplier every emitted animation and transition is measured in. */
	motion: number;
	effects: Record<AppearanceEffectId, boolean>;
}

export interface AppearanceColorField {
	key: AppearanceColorKey;
	token: string;
	/** The colour this one is read against, when contrast is worth reporting. */
	against?: AppearanceColorKey;
}

export const APPEARANCE_COLORS: readonly AppearanceColorField[] = [
	{ key: "background", token: "--background" },
	{ key: "foreground", token: "--foreground", against: "background" },
	{ key: "card", token: "--card" },
	{ key: "border", token: "--border" },
	{ key: "mutedForeground", token: "--muted-foreground", against: "card" },
	{ key: "primary", token: "--primary" },
	{
		key: "primaryForeground",
		token: "--primary-foreground",
		against: "primary",
	},
];

export interface AppearanceEffect {
	id: AppearanceEffectId;
	group: "surface" | "motion";
	/** Emitted CSS. `ms` wraps a duration so the motion multiplier stays in one place. */
	css: (ms: (duration: string) => string) => string;
}

const GRAIN_URI =
	"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.85' numOctaves='3'/%3E%3C/filter%3E%3Crect width='160' height='160' filter='url(%23n)'/%3E%3C/svg%3E";

/** Elevation and the inner highlight both write box-shadow, so the builder joins them. */
const CARD_SHADOWS: Partial<Record<AppearanceEffectId, string>> = {
	bevel: "inset 0 1px 0 oklch(1 0 0 / 0.08)",
	elevation: "0 10px 28px -16px oklch(0 0 0 / 0.45)",
};

export const APPEARANCE_EFFECTS: readonly AppearanceEffect[] = [
	{
		id: "glass",
		group: "surface",
		css: () => `[data-slot="card"],
[data-slot="popover-content"],
[data-slot="dialog-content"] {
  backdrop-filter: blur(16px) saturate(1.4);
  background-color: color-mix(in oklch, var(--card) 70%, transparent);
}`,
	},
	{ id: "elevation", group: "surface", css: () => "" },
	{ id: "bevel", group: "surface", css: () => "" },
	{
		id: "hairline",
		group: "surface",
		css: () => `[data-slot="card"] {
  border-color: color-mix(in oklch, var(--primary) 32%, var(--border));
}`,
	},
	{
		id: "wash",
		group: "surface",
		css: () => `:root {
  background-image: radial-gradient(120% 80% at 12% 0%, color-mix(in oklch, var(--primary) 16%, transparent), transparent 62%);
}`,
	},
	{
		id: "grain",
		group: "surface",
		css: () => `:root {
  position: relative;
}

:root::after {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  opacity: 0.05;
  mix-blend-mode: overlay;
  background-image: url("${GRAIN_URI}");
}`,
	},
	{
		id: "glow",
		group: "surface",
		css: () => `[data-slot="button"].bg-primary {
  box-shadow: 0 8px 28px -8px color-mix(in oklch, var(--primary) 58%, transparent);
}`,
	},
	{
		id: "lift",
		group: "motion",
		css: (ms) => `[data-slot="card"],
[data-slot="button"] {
  transition: transform ${ms("160ms")} ease, box-shadow ${ms("160ms")} ease;
}

[data-slot="card"]:hover,
[data-slot="button"]:hover {
  transform: translateY(-2px);
}`,
	},
	{
		id: "press",
		group: "motion",
		css: () => `[data-slot="button"]:active {
  transform: scale(0.972);
}`,
	},
	{
		id: "entrance",
		group: "motion",
		css: (ms) => `@keyframes fl-appearance-rise {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: none; }
}

[data-slot="card"] {
  animation: fl-appearance-rise ${ms("500ms")} cubic-bezier(0.2, 0.7, 0.3, 1) backwards;
}

[data-slot="card"]:nth-child(2) { animation-delay: ${ms("60ms")}; }
[data-slot="card"]:nth-child(3) { animation-delay: ${ms("120ms")}; }
[data-slot="card"]:nth-child(4) { animation-delay: ${ms("180ms")}; }`,
	},
	{
		id: "sheen",
		group: "motion",
		css: (ms) => `@keyframes fl-appearance-sheen {
  from { transform: translateX(-130%) skewX(-14deg); }
  to { transform: translateX(260%) skewX(-14deg); }
}

[data-slot="button"].bg-primary {
  position: relative;
  overflow: hidden;
}

[data-slot="button"].bg-primary::after {
  content: "";
  position: absolute;
  inset-block: 0;
  width: 38%;
  background: linear-gradient(100deg, transparent, oklch(1 0 0 / 0.42), transparent);
  animation: fl-appearance-sheen ${ms("3.6s")} ease-in-out infinite;
}`,
	},
	{
		id: "aurora",
		group: "motion",
		css: (ms) => `@keyframes fl-appearance-aurora {
  from { transform: translate3d(-3%, -2%, 0) scale(1); }
  to { transform: translate3d(4%, 3%, 0) scale(1.08); }
}

:root {
  position: relative;
  isolation: isolate;
}

:root::before {
  content: "";
  position: absolute;
  inset: -20%;
  z-index: -1;
  pointer-events: none;
  filter: blur(26px);
  background:
    radial-gradient(42% 38% at 22% 28%, color-mix(in oklch, var(--primary) 30%, transparent), transparent 70%),
    radial-gradient(38% 34% at 78% 14%, color-mix(in oklch, var(--primary) 16%, transparent), transparent 68%);
  animation: fl-appearance-aurora ${ms("22s")} ease-in-out infinite alternate;
}`,
	},
	{
		id: "focus",
		group: "motion",
		css: (ms) => `@keyframes fl-appearance-focus {
  from { box-shadow: 0 0 0 0 color-mix(in oklch, var(--primary) 45%, transparent); }
  to { box-shadow: 0 0 0 6px transparent; }
}

[data-slot="input"]:focus-visible,
[data-slot="button"]:focus-visible {
  animation: fl-appearance-focus ${ms("900ms")} ease-out;
}`,
	},
	{
		id: "shimmer",
		group: "motion",
		css: (ms) => `@keyframes fl-appearance-shimmer {
  from { background-position: -180% 0; }
  to { background-position: 180% 0; }
}

[data-slot="skeleton"] {
  background-image: linear-gradient(90deg, transparent 25%, color-mix(in oklch, var(--foreground) 12%, transparent) 50%, transparent 75%);
  background-size: 220% 100%;
  animation: fl-appearance-shimmer ${ms("1.5s")} linear infinite;
}`,
	},
	{
		id: "rows",
		group: "motion",
		css: (ms) => `[data-slot="table-row"] {
  transition: background-color ${ms("160ms")} ease;
}

[data-slot="table-row"]:hover {
  background-color: color-mix(in oklch, var(--primary) 8%, transparent);
}`,
	},
];

export const DEFAULT_APPEARANCE_STATE: AppearanceState = {
	palette: {
		light: {
			background: "#FBFAF8",
			foreground: "#191720",
			card: "#FFFFFF",
			border: "#E4DFD8",
			mutedForeground: "#6E6973",
			primary: "#D93C1F",
			primaryForeground: "#FFFFFF",
		},
		dark: {
			background: "#101014",
			foreground: "#ECEAF0",
			card: "#17171D",
			border: "#2A2A34",
			mutedForeground: "#9895A3",
			primary: "#FF5A3C",
			primaryForeground: "#1B0A06",
		},
	},
	radius: 10,
	spacing: 4,
	fontSans: '"Inter", ui-sans-serif, system-ui, sans-serif',
	motion: 1,
	effects: {
		glass: false,
		elevation: true,
		bevel: false,
		hairline: false,
		wash: false,
		grain: false,
		glow: false,
		lift: true,
		press: true,
		entrance: false,
		sheen: false,
		aurora: false,
		focus: false,
		shimmer: true,
		rows: true,
	},
};

export function cloneAppearanceState(state: AppearanceState): AppearanceState {
	return {
		...state,
		palette: {
			light: { ...state.palette.light },
			dark: { ...state.palette.dark },
		},
		effects: { ...state.effects },
	};
}

/* ------------------------------------------------------------------ colour */

function channels(hex: string): [number, number, number] {
	const value = Number.parseInt(hex.slice(1), 16);
	return [
		((value >> 16) & 255) / 255,
		((value >> 8) & 255) / 255,
		(value & 255) / 255,
	];
}
const toLinear = (c: number) =>
	c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
const toGamma = (c: number) =>
	c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055;
const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

export function hexToOklch(hex: string): string {
	const [r, g, b] = channels(hex).map(toLinear) as [number, number, number];
	const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
	const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
	const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
	const lightness = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
	const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
	const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
	const chroma = Math.hypot(a, bb);
	let hue = (Math.atan2(bb, a) * 180) / Math.PI;
	if (hue < 0) hue += 360;
	const hueOut = chroma < 0.0005 ? "0" : hue.toFixed(2);
	return `oklch(${lightness.toFixed(4)} ${chroma.toFixed(4)} ${hueOut})`;
}

export function oklchToHex(value: string): string | null {
	const match = value
		.trim()
		.match(/^oklch\(\s*([\d.]+%?)\s+([\d.]+)\s+([\d.]+)(?:deg)?\s*\)$/i);
	if (!match) return null;
	const lightness = match[1].endsWith("%")
		? Number.parseFloat(match[1]) / 100
		: Number.parseFloat(match[1]);
	const chroma = Number.parseFloat(match[2]);
	const hue = (Number.parseFloat(match[3]) * Math.PI) / 180;
	const a = chroma * Math.cos(hue);
	const b = chroma * Math.sin(hue);
	const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
	const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
	const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
	const rgb = [
		4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
		-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
		-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
	].map((channel) => Math.round(clamp01(toGamma(channel)) * 255));
	return `#${rgb.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}

export function contrastRatio(a: string, b: string): number {
	const luminance = (hex: string) => {
		const [r, g, bl] = channels(hex).map(toLinear) as [number, number, number];
		return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
	};
	const first = luminance(a);
	const second = luminance(b);
	return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}

/* ------------------------------------------------------------------- build */

export interface BuildAppearanceOptions {
	/**
	 * Emit one palette instead of light plus a `prefers-color-scheme` block. The preview
	 * needs this: its mode is a toggle, not the visitor's system setting.
	 */
	mode?: AppearanceMode;
	/** Raise `:root` specificity so the preview beats the editor's own theme class. */
	boost?: boolean;
}

function paletteBlock(
	selector: string,
	palette: AppearancePalette,
	extra: string[] = [],
): string {
	const lines = APPEARANCE_COLORS.map(
		(field) => `  ${field.token}: ${hexToOklch(palette[field.key])};`,
	);
	return `${selector} {\n${[...lines, ...extra].join("\n")}\n}`;
}

function derivedTokens(state: AppearanceState): string[] {
	return [
		"  --card-foreground: var(--foreground);",
		"  --popover: var(--card);",
		"  --popover-foreground: var(--foreground);",
		"  --input: var(--border);",
		"  --ring: var(--primary);",
		`  --radius: ${(state.radius / 16).toFixed(3)}rem;`,
		`  --spacing: ${(state.spacing / 16).toFixed(4)}rem;`,
		`  --font-sans: ${state.fontSans};`,
		`  --fl-motion: ${state.motion.toFixed(2)};`,
	];
}

export function buildAppearanceBlock(
	state: AppearanceState,
	options: BuildAppearanceOptions = {},
): string {
	const ms = (duration: string) => `calc(${duration} * var(--fl-motion))`;
	const on = (id: AppearanceEffectId) => state.effects[id];
	const parts: string[] = [APPEARANCE_START];

	if (options.mode) {
		const selector = options.boost ? ":root, :root.dark" : ":root";
		parts.push(
			paletteBlock(selector, state.palette[options.mode], derivedTokens(state)),
		);
	} else {
		parts.push(
			"/* :root is rewritten to this app's root element when the sheet is applied. */",
			paletteBlock(":root", state.palette.light, derivedTokens(state)),
			`@media (prefers-color-scheme: dark) {\n${paletteBlock(
				"  :root",
				state.palette.dark,
			)
				.split("\n")
				.join("\n  ")}\n}`,
		);
	}

	const shadows = (Object.keys(CARD_SHADOWS) as AppearanceEffectId[])
		.filter((id) => on(id))
		.map((id) => CARD_SHADOWS[id] as string);
	if (shadows.length > 0) {
		parts.push(
			`[data-slot="card"] {\n  box-shadow: ${shadows.join(",\n    ")};\n}`,
		);
	}

	for (const effect of APPEARANCE_EFFECTS) {
		if (!on(effect.id)) continue;
		const css = effect.css(ms);
		if (!css) continue;
		parts.push(`/* @fx ${effect.id} */\n${css}`);
	}
	for (const id of Object.keys(CARD_SHADOWS) as AppearanceEffectId[]) {
		if (on(id)) parts.push(`/* @fx ${id} */`);
	}

	if (
		APPEARANCE_EFFECTS.some(
			(effect) => effect.group === "motion" && on(effect.id),
		)
	) {
		parts.push(
			"@media (prefers-reduced-motion: reduce) {\n  :root *,\n  :root *::before,\n  :root *::after {\n    animation: none !important;\n    transition: none !important;\n  }\n}",
		);
	}

	parts.push(APPEARANCE_END);
	return parts.join("\n\n");
}

export function composeAppearanceSheet(block: string, tail: string): string {
	const trimmed = tail.trim();
	return trimmed ? `${block}\n\n${trimmed}\n` : `${block}\n`;
}

/* ------------------------------------------------------------------- parse */

export interface ParsedAppearanceSheet {
	state: AppearanceState;
	/** Everything after the end marker — the author's own rules, kept verbatim. */
	tail: string;
	/** False when the sheet had no managed block, so the whole sheet is the tail. */
	managed: boolean;
}

function readDeclarations(block: string, palette: AppearancePalette): void {
	for (const field of APPEARANCE_COLORS) {
		const match = block.match(
			new RegExp(`${field.token}\\s*:\\s*([^;]+);`, "i"),
		);
		if (!match) continue;
		const raw = match[1].trim();
		const hex = raw.startsWith("#")
			? raw.slice(0, 7)
			: raw.toLowerCase().startsWith("oklch")
				? oklchToHex(raw)
				: null;
		if (hex && /^#[0-9a-f]{6}$/i.test(hex)) palette[field.key] = hex;
	}
}

/** The first `:root { … }` body inside the managed block, and the dark one if present. */
function rootBodies(managed: string): { light: string; dark: string } {
	const dark = managed.match(
		/@media\s*\(\s*prefers-color-scheme\s*:\s*dark\s*\)\s*\{([\s\S]*?)\n\}/i,
	);
	const withoutDark = dark ? managed.replace(dark[0], "") : managed;
	const light = withoutDark.match(/:root[^{]*\{([^}]*)\}/);
	return { light: light?.[1] ?? "", dark: dark?.[1] ?? "" };
}

export function parseAppearanceSheet(sheet: string): ParsedAppearanceSheet {
	const state = cloneAppearanceState(DEFAULT_APPEARANCE_STATE);
	const start = sheet.indexOf(APPEARANCE_START);
	const end = sheet.indexOf(APPEARANCE_END);
	if (start < 0 || end < 0 || end < start) {
		return { state, tail: sheet.trim(), managed: false };
	}

	const managed = sheet.slice(start, end);
	const tail = sheet.slice(end + APPEARANCE_END.length).trim();
	const bodies = rootBodies(managed);
	readDeclarations(bodies.light, state.palette.light);
	readDeclarations(bodies.dark || bodies.light, state.palette.dark);

	const radius = managed.match(/--radius:\s*([\d.]+)rem/);
	if (radius) state.radius = Math.round(Number.parseFloat(radius[1]) * 16);
	const spacing = managed.match(/--spacing:\s*([\d.]+)rem/);
	if (spacing)
		state.spacing = Math.round(Number.parseFloat(spacing[1]) * 16 * 100) / 100;
	const motion = managed.match(/--fl-motion:\s*([\d.]+)/);
	if (motion) state.motion = Number.parseFloat(motion[1]);
	const font = managed.match(/--font-sans:\s*([^;]+);/);
	if (font) state.fontSans = font[1].trim();

	for (const effect of APPEARANCE_EFFECTS) {
		state.effects[effect.id] = managed.includes(`/* @fx ${effect.id} */`);
	}

	return { state, tail, managed: true };
}
