"use client";

import { useTranslation } from "@flow-like/locales";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { cn } from "../../lib";
import {
	FLOW_MARK_BODY,
	FLOW_MARK_INSET,
	FLOW_MARK_VIEW_BOX,
} from "./flow-mark-paths";

interface LoadingScreenProps {
	message?: string;
	progress?: number;
	className?: string;
}

interface TipCard {
	readonly text: string;
	readonly hint: boolean;
}

interface Ember {
	x: number;
	y: number;
	vx: number;
	vy: number;
	life: number;
	ttl: number;
	r: number;
	warm: boolean;
	phase: number;
}

const TIPS = [
	"Press Ctrl+K to open the command palette from anywhere.",
	"Connect nodes by dragging from one pin to another.",
	"Browse community packages in the Registry to extend your workflows.",
	"Your flows auto-save — no need to hit save manually.",
	"Use the search bar in the node catalog to find nodes quickly.",
	"Double-click the canvas to create a new node at that position.",
	"Select multiple nodes with Shift+Click to move them together.",
	"Test individual nodes by right-clicking and selecting 'Run'.",
	"Toggle dark mode in Settings → Appearance.",
	"Undo with Ctrl+Z — works for node connections too.",
	"Use the Data Viewer node to inspect values mid-flow.",
	"Pin frequently used nodes to the toolbar for faster access.",
] as const;

const HINTS = [
	"Try Flow Like Studio — the desktop app for offline editing and local execution.",
	"FlowPilot can generate entire workflows from a text description.",
	"Build custom nodes with WASM — use any language that compiles to WebAssembly.",
	"Deploy flows to the cloud with one click from the Studio.",
	"Flow Like Studio syncs your projects across all your devices.",
	"Studio supports local-only mode — your data never leaves your machine.",
] as const;

const CARDS: readonly TipCard[] = [
	...TIPS.map((text) => ({ text, hint: false })),
	...HINTS.map((text) => ({ text, hint: true })),
];

const STEPS = [
	["loadingStepRestoringSession", "Restoring session"],
	["loadingStepLoadingProfile", "Loading profile"],
	["loadingStepFetchingApps", "Fetching your apps"],
	["loadingStepConnectingHub", "Connecting to hub"],
	["loadingStepVerifying", "Verifying"],
	["loadingStepSyncing", "Syncing"],
	["loadingStepAlmostReady", "Almost ready"],
] as const;

/** Percentage at which each step counts as done. */
const THRESHOLDS = [10, 30, 60, 90, 95, 98, 100] as const;

/** Fill line in user space: y at 0%, y at 99%, y at 100%. */
const BOTTOM = 1036;
const TOP99 = 204;
const TOP = 130;

const MAX_EMBERS = 28;
const TIP_INTERVAL_MS = 5500;
const TIP_SWAP_MS = 260;

/** Horizontal spans of the mark, keyed by user-space y, used to seat embers. */
const EMBER_BANDS: readonly (readonly [number, number, number, number])[] = [
	[224, 448, 280, 1000],
	[448, 516, 280, 496],
	[516, 747, 280, 888],
	[747, 1024, 282, 496],
];

const FALLBACK_AMBER: readonly [number, number, number] = [253, 171, 56];
const FALLBACK_EMBER: readonly [number, number, number] = [251, 72, 39];

function levelFor(p: number): number {
	return p <= 0.99
		? BOTTOM - (BOTTOM - TOP99) * (p / 0.99)
		: TOP99 - (TOP99 - TOP) * ((p - 0.99) / 0.01);
}

function smoothstep(a: number, b: number, x: number): number {
	const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
	return t * t * (3 - 2 * t);
}

function hexToRgb(value: string): [number, number, number] | null {
	const match = /^#?([0-9a-f]{6})$/i.exec(value.trim());
	if (!match) return null;
	const int = Number.parseInt(match[1], 16);
	return [(int >> 16) & 255, (int >> 8) & 255, int & 255];
}

function pickTip(exclude: number): number {
	let index = exclude;
	while (index === exclude && CARDS.length > 1) {
		index = Math.floor(Math.random() * CARDS.length);
	}
	return index;
}

const LOADING_SCREEN_CSS = `
@property --ls-settle { syntax: "<number>"; inherits: true; initial-value: 1; }

.ls-root {
	--ls-amber: #fdab38;
	--ls-ember: #fb4827;
	--ls-red: #ea183d;
	--ls-ghost: #b8aaa2;
	--ls-heat-core: #ffcd6b;
	--ls-heat-core-a: 0.55;
	--ls-indet-dim: saturate(0.7) opacity(0.5);
	--ls-bloom-max: 0.42;
	--ls-ember-alpha: 0.32;
}
.dark .ls-root {
	--ls-ghost: #3e4147;
	--ls-heat-core: #ffe2b0;
	--ls-heat-core-a: 0.82;
	--ls-indet-dim: brightness(0.42) saturate(0.9);
	--ls-bloom-max: 0.8;
	--ls-ember-alpha: 0.7;
}
.ls-root :focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }

.ls-grid {
	background: radial-gradient(var(--border) 1px, transparent 1.2px) 0 0 / 28px 28px;
	-webkit-mask-image: radial-gradient(ellipse 64% 78% at 46% 50%, #000 20%, transparent 100%);
	mask-image: radial-gradient(ellipse 64% 78% at 46% 50%, #000 20%, transparent 100%);
}

.ls-screen {
	display: grid;
	grid-template-rows: auto 1fr auto;
	gap: 24px;
	height: 100%;
	max-width: 1280px;
	margin: 0 auto;
	padding: 40px 56px;
	overflow: auto;
}
.ls-stage {
	align-self: center;
	justify-self: center;
	display: grid;
	grid-template-columns: auto auto;
	align-items: center;
	column-gap: clamp(40px, 6vw, 80px);
	max-width: 100%;
}
.ls-mark-wrap {
	--ls-p: 0;
	--ls-heat: 0;
	--ls-settle: 1;
	position: relative;
	width: clamp(176px, 26vh, 238px);
	aspect-ratio: 752 / 820;
	transition: --ls-settle 900ms ease;
}
.ls-bloom {
	inset: -34%;
	background: radial-gradient(closest-side,
		color-mix(in srgb, var(--ls-ember) 58%, transparent) 0%,
		color-mix(in srgb, var(--ls-red) 30%, transparent) 40%,
		color-mix(in srgb, var(--ls-amber) 14%, transparent) 62%,
		transparent 100%);
	filter: blur(26px);
	opacity: calc(var(--ls-bloom-max) * (0.1 + 0.9 * var(--ls-p)) * var(--ls-settle, 1));
	transform: scale(calc(0.68 + 0.42 * var(--ls-p)));
	will-change: opacity, transform;
}
.ls-flare {
	inset: -20%;
	opacity: 0;
	background: radial-gradient(closest-side, color-mix(in srgb, var(--ls-amber) 70%, transparent), transparent 70%);
	filter: blur(18px);
}
.ls-embers { left: -45%; top: -70%; width: 190%; height: 240%; }

.ls-ghost path { fill: none; stroke: var(--ls-ghost); stroke-width: 1.75; vector-effect: non-scaling-stroke; }
.ls-trace path {
	fill: none;
	stroke: var(--ls-amber);
	stroke-width: 1.6;
	vector-effect: non-scaling-stroke;
	stroke-linecap: round;
	stroke-dasharray: 0.14 0.86;
	stroke-dashoffset: 1;
	opacity: 0;
}
.ls-boot .ls-trace path { animation: ls-trace 1500ms cubic-bezier(0.4, 0, 0.2, 1) 120ms forwards; }
@keyframes ls-trace {
	0% { stroke-dashoffset: 1; opacity: 0; }
	12% { opacity: 1; }
	86% { opacity: 1; }
	100% { stroke-dashoffset: -0.14; opacity: 0; }
}
.ls-lit, .ls-heat { transition: opacity 360ms ease; }
.ls-heat { opacity: var(--ls-heat, 0); }
.ls-pulse { display: none; }
.ls-heat-core { stop-color: var(--ls-heat-core); stop-opacity: var(--ls-heat-core-a); }
.ls-indet .ls-heat { display: none; }
.ls-indet .ls-lit { filter: var(--ls-indet-dim); }
.ls-indet .ls-pulse { display: block; }
.ls-ready .ls-heat { opacity: 0; }
.ls-ready .ls-mark-wrap { --ls-settle: 0.6; animation: ls-settle 900ms cubic-bezier(0.2, 0.8, 0.2, 1) both; }
.ls-ready .ls-flare { animation: ls-flare 800ms ease-out both; }
@keyframes ls-settle { 0% { transform: scale(1.035); } 100% { transform: scale(1); } }
@keyframes ls-flare { 0% { opacity: 0; } 25% { opacity: 0.5; } 100% { opacity: 0; } }

.ls-copy { display: grid; row-gap: 28px; }
.ls-status {
	margin: 0;
	display: flex;
	align-items: baseline;
	gap: 16px;
	font-size: 22px;
	font-weight: 500;
	letter-spacing: -0.01em;
	line-height: 1.2;
	text-wrap: balance;
}
.ls-status-stack { display: inline-grid; }
.ls-status-stack > * { grid-area: 1 / 1; }
.ls-sizer { visibility: hidden; }
.ls-pct {
	display: inline-grid;
	grid-template-columns: 3ch auto;
	font-size: 13px;
	font-weight: 400;
	letter-spacing: 0;
}
.ls-pct-n { text-align: right; }
.ls-indet .ls-pct-unit { visibility: hidden; }

.ls-rail { position: relative; }
.ls-rail ol { list-style: none; margin: 0; padding: 0; display: grid; row-gap: 10px; }
.ls-rail-track { position: absolute; left: 3.5px; top: 8px; bottom: 8px; width: 1px; background: var(--border); }
.ls-rail-wire {
	position: absolute;
	left: 3.5px;
	top: 8px;
	width: 1px;
	height: calc(100% - 16px);
	background: linear-gradient(to bottom, var(--ls-amber), var(--ls-ember) 60%, var(--ls-red));
	transform-origin: top;
	transform: scaleY(var(--ls-wire, 0));
	transition: transform 320ms cubic-bezier(0.4, 0, 0.2, 1);
}
.ls-indet .ls-rail-wire {
	transform: none;
	transition: none;
	opacity: 0.9;
	background: linear-gradient(to bottom, transparent, var(--primary), transparent) 0 calc(var(--ls-k, 0.5) * 100%) / 100% 44px no-repeat;
}
.ls-step {
	position: relative;
	display: grid;
	grid-template-columns: 8px 24px 1fr;
	column-gap: 14px;
	align-items: center;
	font-size: 13px;
	line-height: 1.3;
	color: var(--muted-foreground);
}
.ls-pin {
	width: 8px;
	height: 8px;
	border-radius: 50%;
	position: relative;
	z-index: 1;
	border: 1.5px solid var(--border);
	background: var(--background);
	transition: border-color 200ms, background-color 200ms;
}
.ls-pin::after {
	content: "";
	position: absolute;
	inset: -5px;
	border-radius: 50%;
	border: 1px solid var(--primary);
	opacity: 0;
	transform: scale(0.5);
}
.ls-ord { font-size: 11px; letter-spacing: 0.1em; color: var(--muted-foreground); }
.ls-step-name { transition: color 200ms; }
.ls-step[data-state="done"] .ls-pin { border-color: var(--primary); background: var(--primary); }
.ls-step[data-state="done"] .ls-step-name { color: var(--foreground); opacity: 0.62; }
.ls-step[data-state="active"] .ls-pin { border-color: var(--primary); background: var(--background); }
.ls-step[data-state="active"] .ls-step-name { color: var(--foreground); font-weight: 500; }
.ls-step[data-state="active"] .ls-pin::after { animation: ls-halo 2.6s ease-out infinite; }
@keyframes ls-halo {
	0% { opacity: 0.55; transform: scale(0.45); }
	100% { opacity: 0; transform: scale(1.35); }
}

.ls-bottom { display: flex; align-items: flex-start; justify-content: space-between; gap: 32px; min-height: 3.2em; }
.ls-tip {
	margin: 0;
	display: grid;
	grid-template-columns: auto 1fr;
	column-gap: 14px;
	align-items: baseline;
	max-width: 62ch;
	min-height: 1.5em;
	font-size: 13px;
}
.ls-tip-label {
	display: inline-grid;
	font-size: 11px;
	letter-spacing: 0.12em;
	text-transform: uppercase;
	white-space: nowrap;
}
.ls-tip-label > * { grid-area: 1 / 1; }
.ls-tip-label, .ls-tip-text { transition: opacity 260ms; }
.ls-tip[data-swap="true"] .ls-tip-label, .ls-tip[data-swap="true"] .ls-tip-text { opacity: 0.25; }

.ls-boot .ls-wordmark { animation: ls-rise 600ms cubic-bezier(0.2, 0.8, 0.2, 1) both; }
.ls-boot .ls-status { animation: ls-rise 600ms cubic-bezier(0.2, 0.8, 0.2, 1) 80ms both; }
.ls-boot .ls-rail { animation: ls-rise 600ms cubic-bezier(0.2, 0.8, 0.2, 1) 140ms both; }
.ls-boot .ls-tip { animation: ls-rise 600ms cubic-bezier(0.2, 0.8, 0.2, 1) 200ms both; }
@keyframes ls-rise {
	from { opacity: 0.45; transform: translateY(5px); }
	to { opacity: 1; transform: none; }
}

@media (max-width: 1200px) { .ls-screen { padding-bottom: 68px; } }
@media (max-width: 860px) {
	.ls-screen { padding: 28px 22px 64px; }
	.ls-stage { grid-template-columns: 1fr; row-gap: 32px; justify-items: start; justify-self: start; }
	.ls-mark-wrap { width: min(52vw, 220px); }
	.ls-status { font-size: 19px; }
	.ls-bottom { flex-direction: column; align-items: flex-start; min-height: 4.6em; }
}
@media (max-width: 600px) { .ls-screen { padding-bottom: 96px; } }
@media (max-width: 480px) { .ls-tip { grid-template-columns: 1fr; row-gap: 4px; } }

@media (prefers-reduced-motion: reduce) {
	.ls-boot .ls-wordmark,
	.ls-boot .ls-status,
	.ls-boot .ls-rail,
	.ls-boot .ls-tip,
	.ls-boot .ls-trace path,
	.ls-ready .ls-mark-wrap,
	.ls-ready .ls-flare,
	.ls-step[data-state="active"] .ls-pin::after { animation: none; }
	.ls-mark-wrap,
	.ls-rail-wire,
	.ls-pin,
	.ls-tip-label,
	.ls-tip-text,
	.ls-lit,
	.ls-heat { transition: none; }
	.ls-indet .ls-rail-wire { display: none; }
	.ls-indet .ls-pulse { display: none; }
}
`;

export function LoadingScreen({
	message,
	progress = 0,
	className,
}: Readonly<LoadingScreenProps>) {
	const { t } = useTranslation("common");
	const raw = useId();
	const prefix = `ls-${raw.replace(/[^a-zA-Z0-9]/g, "")}`;
	const id = useCallback((key: string) => `${prefix}-${key}`, [prefix]);
	const url = useCallback((key: string) => `url(#${prefix}-${key})`, [prefix]);

	const clamped = Math.min(Math.max(progress, 0), 100);
	const indeterminate = !(clamped > 0);
	const ready = clamped >= 100;

	const rootRef = useRef<HTMLDivElement>(null);
	const markWrapRef = useRef<HTMLDivElement>(null);
	const levelRef = useRef<SVGGElement>(null);
	const levelGhostRef = useRef<SVGGElement>(null);
	const heatGradientRef = useRef<SVGLinearGradientElement>(null);
	const pulseGradientRef = useRef<SVGLinearGradientElement>(null);
	const percentRef = useRef<HTMLSpanElement>(null);
	const railRef = useRef<HTMLDivElement>(null);
	const stepRefs = useRef<(HTMLLIElement | null)[]>([]);
	const canvasRef = useRef<HTMLCanvasElement>(null);

	const stateRef = useRef({
		target: 0,
		disp: 0,
		indeterminate: true,
		ready: false,
	});
	const reducedRef = useRef(false);
	const kickRef = useRef<() => void>(() => {});

	// The first server and client render must agree. Randomizing in the state initializer causes a
	// hydration recovery that can remount consumers and replay their startup effects.
	const [tipIndex, setTipIndex] = useState(0);
	const [tipSwap, setTipSwap] = useState(false);
	const [reduced, setReduced] = useState(false);

	// Declared before the animation effect so the very first frame already sees the real props.
	useEffect(() => {
		const state = stateRef.current;
		state.target = clamped / 100;
		state.indeterminate = indeterminate;
		state.ready = ready;
		if (reducedRef.current) state.disp = state.target;
		kickRef.current();
	}, [clamped, indeterminate, ready]);

	useEffect(() => {
		const root = rootRef.current;
		const markWrap = markWrapRef.current;
		const level = levelRef.current;
		const levelGhost = levelGhostRef.current;
		const heatGradient = heatGradientRef.current;
		const pulseGradient = pulseGradientRef.current;
		const percent = percentRef.current;
		const rail = railRef.current;
		const canvas = canvasRef.current;
		if (
			!root ||
			!markWrap ||
			!level ||
			!levelGhost ||
			!heatGradient ||
			!pulseGradient ||
			!percent ||
			!rail
		) {
			return;
		}

		const state = stateRef.current;
		const last: Record<string, string | number> = {};
		const setIf = <T extends string | number>(
			key: string,
			value: T,
			apply: (next: T) => void,
		) => {
			if (last[key] === value) return;
			last[key] = value;
			apply(value);
		};

		const tokens: {
			alpha: number;
			amber: readonly [number, number, number];
			ember: readonly [number, number, number];
		} = { alpha: 0.6, amber: FALLBACK_AMBER, ember: FALLBACK_EMBER };
		const readTokens = () => {
			const style = getComputedStyle(root);
			tokens.alpha =
				Number.parseFloat(style.getPropertyValue("--ls-ember-alpha")) || 0.6;
			tokens.amber =
				hexToRgb(style.getPropertyValue("--ls-amber")) ?? tokens.amber;
			tokens.ember =
				hexToRgb(style.getPropertyValue("--ls-ember")) ?? tokens.ember;
		};

		const setLevel = (y: number) => {
			setIf("level", `translate(0 ${y.toFixed(1)})`, (value) => {
				level.setAttribute("transform", value);
				levelGhost.setAttribute("transform", value);
			});
		};

		const renderFill = (p: number, now: number) => {
			const envelope = reducedRef.current
				? 0
				: smoothstep(0.02, 0.08, p) * (1 - smoothstep(0.94, 0.99, p));
			const wobble =
				envelope * (Math.sin(now / 620) * 3 + Math.sin(now / 1730) * 2);
			const y = levelFor(p) + wobble;
			setLevel(y);
			setIf("heat", `translate(0 ${(y - 40).toFixed(1)})`, (value) =>
				heatGradient.setAttribute("gradientTransform", value),
			);
			const heat = state.ready ? 0 : Math.min(1, p * 8) * (p < 0.995 ? 1 : 0);
			setIf("heatOpacity", heat.toFixed(2), (value) =>
				markWrap.style.setProperty("--ls-heat", value),
			);
			setIf("bloom", (state.ready ? 1 : p).toFixed(3), (value) =>
				markWrap.style.setProperty("--ls-p", value),
			);
		};

		const renderPulse = (now: number) => {
			setLevel(TOP);
			const u = reducedRef.current ? 0.5 : (now % 2800) / 2800;
			const k = 0.62 - 1.24 * u;
			setIf(
				"pulse",
				`translate(${(k * -728).toFixed(1)} ${(k * 802).toFixed(1)})`,
				(value) => pulseGradient.setAttribute("gradientTransform", value),
			);
			setIf("k", (1 - u).toFixed(3), (value) =>
				rail.style.setProperty("--ls-k", value),
			);
			setIf(
				"bloom",
				(reducedRef.current ? 0.5 : 0.42 + 0.14 * Math.sin(now / 900)).toFixed(
					3,
				),
				(value) => markWrap.style.setProperty("--ls-p", value),
			);
		};

		const renderText = (p: number) => {
			setIf(
				"percent",
				state.indeterminate
					? "···"
					: String(state.ready ? 100 : Math.round(p * 100)),
				(value) => {
					percent.textContent = value;
				},
			);
			const shown = state.ready ? 100 : p * 100;
			const steps = stepRefs.current;
			let done = 0;
			let active = -1;
			if (!state.indeterminate) {
				for (let i = 0; i < STEPS.length; i++) {
					if (shown >= THRESHOLDS[i] - 0.001 || state.ready) done++;
					else if (active < 0) active = i;
				}
			}
			setIf("steps", `${done}:${active}`, () => {
				for (let i = 0; i < steps.length; i++) {
					const step = steps[i];
					if (!step) continue;
					if (i < done) step.dataset.state = "done";
					else if (i === active) step.dataset.state = "active";
					else step.removeAttribute("data-state");
				}
			});
			const fraction = done <= 0 ? 0 : (done - 1) / (STEPS.length - 1);
			setIf("wire", (state.ready ? 1 : fraction).toFixed(3), (value) =>
				rail.style.setProperty("--ls-wire", value),
			);
		};

		const context = canvas?.getContext("2d") ?? null;
		const embers: Ember[] = [];
		let width = 0;
		let height = 0;
		let box = { x: 0, y: 0, w: 1, h: 1 };
		let spawnAccumulator = 0;

		const resize = () => {
			if (!canvas || !context) return;
			width = canvas.clientWidth;
			height = canvas.clientHeight;
			const dpr = Math.min(window.devicePixelRatio || 1, 2);
			canvas.width = Math.round(width * dpr);
			canvas.height = Math.round(height * dpr);
			context.setTransform(dpr, 0, 0, dpr, 0, 0);
			box = {
				x: -canvas.offsetLeft,
				y: -canvas.offsetTop,
				w: markWrap.clientWidth,
				h: markWrap.clientHeight,
			};
		};

		const spawnRate = () => {
			if (
				reducedRef.current ||
				state.ready ||
				(!state.indeterminate && state.disp < 0.02)
			) {
				return 0;
			}
			return state.indeterminate ? 2.5 : 1.5 + 5 * state.disp;
		};

		const spawn = (vy: number) => {
			const band =
				EMBER_BANDS.find((b) => vy >= b[0] && vy <= b[1]) ?? EMBER_BANDS[0];
			const vx = band[2] + Math.random() * (band[3] - band[2]);
			embers.push({
				x: box.x + ((vx - 262) * box.w) / 752,
				y: box.y + ((vy - 214) * box.h) / 820,
				vx: (Math.random() - 0.5) * 8,
				vy: -(9 + Math.random() * 18),
				life: 0,
				ttl: 2.2 + Math.random() * 2,
				r: 0.7 + Math.random() * 1.2,
				warm: Math.random() < 0.6,
				phase: Math.random() * 6.28,
			});
		};

		const renderEmbers = (dt: number) => {
			if (!context) return;
			if (reducedRef.current) {
				if (embers.length) {
					embers.length = 0;
					context.clearRect(0, 0, width, height);
				}
				return;
			}
			spawnAccumulator += spawnRate() * dt;
			while (spawnAccumulator >= 1 && embers.length < MAX_EMBERS) {
				spawnAccumulator -= 1;
				const vy = state.indeterminate
					? 224 + Math.random() * 800
					: levelFor(state.disp) + 20 + (Math.random() - 0.5) * 24;
				spawn(Math.max(224, Math.min(1024, vy)));
			}
			if (spawnAccumulator > 2) spawnAccumulator = 0;
			context.clearRect(0, 0, width, height);
			for (let i = embers.length - 1; i >= 0; i--) {
				const q = embers[i];
				q.life += dt;
				if (q.life >= q.ttl) {
					embers.splice(i, 1);
					continue;
				}
				q.x += (q.vx + Math.sin(q.life * 1.6 + q.phase) * 6) * dt;
				q.y += q.vy * dt;
				const progressed = q.life / q.ttl;
				const alpha =
					tokens.alpha *
					Math.sin(Math.PI * progressed) *
					(0.5 + 0.5 * (1 - progressed));
				const rgb = q.warm ? tokens.amber : tokens.ember;
				const channels = `${rgb[0]},${rgb[1]},${rgb[2]}`;
				context.fillStyle = `rgba(${channels},${(alpha * 0.16).toFixed(3)})`;
				context.beginPath();
				context.arc(q.x, q.y, q.r * 2.6, 0, 6.2832);
				context.fill();
				context.fillStyle = `rgba(${channels},${alpha.toFixed(3)})`;
				context.beginPath();
				context.arc(q.x, q.y, q.r, 0, 6.2832);
				context.fill();
			}
		};

		let frame = 0;
		let previous = 0;

		const needsFrame = () => {
			if (reducedRef.current) return state.disp !== state.target;
			if (state.indeterminate) return true;
			const p = state.disp;
			return (
				p !== state.target ||
				(p > 0.02 && p < 0.99) ||
				embers.length > 0 ||
				spawnRate() > 0
			);
		};

		const tick = (now: number) => {
			frame = 0;
			const dt = Math.min(64, now - (previous || now));
			previous = now;
			if (reducedRef.current) state.disp = state.target;
			if (state.indeterminate) {
				renderPulse(now);
			} else {
				if (!reducedRef.current) {
					const k = 1 - Math.exp(-dt / 240);
					state.disp += (state.target - state.disp) * k;
					if (Math.abs(state.target - state.disp) < 0.0005) {
						state.disp = state.target;
					}
				}
				renderFill(state.disp, now);
			}
			renderText(state.disp);
			renderEmbers(dt / 1000);
			if (needsFrame()) frame = requestAnimationFrame(tick);
			else previous = 0;
		};

		const kick = () => {
			if (!frame) frame = requestAnimationFrame(tick);
		};
		kickRef.current = kick;

		const motionQuery =
			typeof window.matchMedia === "function"
				? window.matchMedia("(prefers-reduced-motion: reduce)")
				: null;
		const applyMotion = () => {
			reducedRef.current = motionQuery?.matches ?? false;
			setReduced(reducedRef.current);
			if (reducedRef.current) state.disp = state.target;
			kick();
		};
		motionQuery?.addEventListener("change", applyMotion);

		const themeObserver = new MutationObserver(readTokens);
		themeObserver.observe(document.documentElement, {
			attributes: true,
			attributeFilter: ["class"],
		});

		let resizeObserver: ResizeObserver | null = null;
		if (typeof ResizeObserver === "function") {
			resizeObserver = new ResizeObserver(resize);
			resizeObserver.observe(markWrap);
		} else {
			window.addEventListener("resize", resize);
		}

		readTokens();
		resize();
		applyMotion();
		if (state.indeterminate) renderPulse(0);
		else renderFill(state.disp, 0);
		renderText(state.disp);
		kick();

		return () => {
			kickRef.current = () => {};
			if (frame) cancelAnimationFrame(frame);
			motionQuery?.removeEventListener("change", applyMotion);
			themeObserver.disconnect();
			if (resizeObserver) resizeObserver.disconnect();
			else window.removeEventListener("resize", resize);
		};
	}, []);

	useEffect(() => {
		setTipIndex((current) => pickTip(current));
	}, []);

	useEffect(() => {
		if (reduced) return;
		let swap: ReturnType<typeof setTimeout> | undefined;
		const rotate = setInterval(() => {
			setTipSwap(true);
			swap = setTimeout(() => {
				setTipIndex((current) => pickTip(current));
				setTipSwap(false);
			}, TIP_SWAP_MS);
		}, TIP_INTERVAL_MS);
		return () => {
			clearInterval(rotate);
			if (swap) clearTimeout(swap);
		};
	}, [reduced]);

	const card = CARDS[tipIndex];
	const headline =
		message ?? t("loadingYourWorkspace", "Loading your workspace");
	const tipLabel = card.hint
		? t("didYouKnow", "Did you know?")
		: t("tip", "Tip");

	return (
		<div
			ref={rootRef}
			className={cn(
				"ls-root ls-boot fixed inset-0 z-50 overflow-hidden bg-background font-sans text-foreground",
				indeterminate && "ls-indet",
				ready && "ls-ready",
				className,
			)}
		>
			<div
				className="ls-grid pointer-events-none absolute inset-0"
				aria-hidden="true"
			/>

			<div className="ls-screen relative z-[1]">
				<header>
					<span className="ls-wordmark text-base font-medium tracking-[0.02em]">
						Flow Like
					</span>
				</header>

				<section className="ls-stage">
					<div ref={markWrapRef} className="ls-mark-wrap">
						<div
							className="ls-bloom pointer-events-none absolute z-0 rounded-full"
							aria-hidden="true"
						/>
						<div
							className="ls-flare pointer-events-none absolute z-0 rounded-full"
							aria-hidden="true"
						/>
						<svg
							className="relative z-[1] block h-full w-full overflow-visible"
							viewBox={FLOW_MARK_VIEW_BOX}
							role="img"
							aria-label="Flow Like"
						>
							<defs>
								<linearGradient
									id={id("gFill1")}
									x1="819"
									y1="513.618"
									x2="366.5"
									y2="974.118"
									gradientUnits="userSpaceOnUse"
								>
									<stop stopColor="#FD6F21" />
									<stop offset="0.331971" stopColor="#FA4C29" />
									<stop offset="1" stopColor="#EA183D" />
								</linearGradient>
								<linearGradient
									id={id("gStroke1")}
									x1="859"
									y1="514"
									x2="272"
									y2="1026"
									gradientUnits="userSpaceOnUse"
								>
									<stop stopColor="#FC8E2E" />
									<stop offset="1" stopColor="#F81428" />
								</linearGradient>
								<linearGradient
									id={id("gFill2")}
									x1="947.5"
									y1="238.5"
									x2="353.5"
									y2="693.5"
									gradientUnits="userSpaceOnUse"
								>
									<stop stopColor="#FD8523" />
									<stop offset="0.156691" stopColor="#FD7520" />
									<stop offset="0.360842" stopColor="#FC6427" />
									<stop offset="0.8059" stopColor="#FB4827" />
									<stop offset="1" stopColor="#FB2F33" />
								</linearGradient>
								<linearGradient
									id={id("gStroke2")}
									x1="963.5"
									y1="224"
									x2="326"
									y2="747"
									gradientUnits="userSpaceOnUse"
								>
									<stop stopColor="#FDAB38" />
									<stop offset="1" stopColor="#F01B1A" />
								</linearGradient>
								<mask id={id("mInside")} fill="white">
									<path d={FLOW_MARK_INSET} />
								</mask>
								<linearGradient id={id("gEdge")} x1="0" y1="0" x2="0" y2="1">
									<stop offset="0.5" stopColor="#000" />
									<stop offset="0.535" stopColor="#fff" />
								</linearGradient>
								<mask
									id={id("mLevel")}
									maskUnits="userSpaceOnUse"
									x="200"
									y="150"
									width="900"
									height="950"
								>
									<g ref={levelRef} transform={`translate(0 ${BOTTOM})`}>
										<rect
											x="200"
											y="-1200"
											width="900"
											height="2400"
											fill={url("gEdge")}
										/>
									</g>
								</mask>
								<linearGradient id={id("gEdgeInv")} x1="0" y1="0" x2="0" y2="1">
									<stop offset="0.5" stopColor="#fff" />
									<stop offset="0.535" stopColor="#000" />
								</linearGradient>
								<mask
									id={id("mGhost")}
									maskUnits="userSpaceOnUse"
									x="200"
									y="150"
									width="900"
									height="950"
								>
									<g ref={levelGhostRef} transform={`translate(0 ${BOTTOM})`}>
										<rect
											x="200"
											y="-1200"
											width="900"
											height="2400"
											fill={url("gEdgeInv")}
										/>
									</g>
								</mask>
								<linearGradient
									ref={heatGradientRef}
									id={id("gHeat")}
									x1="0"
									y1="0"
									x2="0"
									y2="130"
									gradientUnits="userSpaceOnUse"
									gradientTransform="translate(0 1000)"
								>
									<stop offset="0" stopColor="#FDAB38" stopOpacity="0" />
									<stop offset="0.3" stopColor="#FDAB38" stopOpacity="0.45" />
									<stop
										offset="0.48"
										className="ls-heat-core"
										stopOpacity="0.82"
									/>
									<stop offset="0.66" stopColor="#FDAB38" stopOpacity="0.4" />
									<stop offset="1" stopColor="#FB4827" stopOpacity="0" />
								</linearGradient>
								<linearGradient
									ref={pulseGradientRef}
									id={id("gPulse")}
									x1="1000"
									y1="224"
									x2="272"
									y2="1026"
									gradientUnits="userSpaceOnUse"
								>
									<stop offset="0" stopColor="#000" />
									<stop offset="0.36" stopColor="#000" />
									<stop offset="0.5" stopColor="#fff" />
									<stop offset="0.64" stopColor="#000" />
									<stop offset="1" stopColor="#000" />
								</linearGradient>
								<mask
									id={id("mPulse")}
									maskUnits="userSpaceOnUse"
									x="200"
									y="150"
									width="900"
									height="950"
								>
									<rect
										x="200"
										y="150"
										width="900"
										height="950"
										fill={url("gPulse")}
									/>
								</mask>
							</defs>

							<g className="ls-ghost" mask={url("mGhost")}>
								<path d={FLOW_MARK_INSET} />
								<path d={FLOW_MARK_BODY} />
							</g>
							<g className="ls-trace">
								<path pathLength="1" d={FLOW_MARK_INSET} />
								<path pathLength="1" d={FLOW_MARK_BODY} />
							</g>
							<g className="ls-lit" mask={url("mLevel")}>
								<path
									d={FLOW_MARK_BODY}
									fill={url("gFill1")}
									stroke={url("gStroke1")}
									strokeWidth="4"
								/>
								<path
									d={FLOW_MARK_INSET}
									fill={url("gFill2")}
									stroke={url("gStroke2")}
									strokeWidth="8"
									strokeLinejoin="round"
									mask={url("mInside")}
								/>
							</g>
							<g className="ls-heat">
								<path d={FLOW_MARK_BODY} fill={url("gHeat")} />
								<path d={FLOW_MARK_INSET} fill={url("gHeat")} />
							</g>
							<g className="ls-pulse" mask={url("mPulse")}>
								<path
									d={FLOW_MARK_BODY}
									fill={url("gFill1")}
									stroke={url("gStroke1")}
									strokeWidth="4"
								/>
								<path
									d={FLOW_MARK_INSET}
									fill={url("gFill2")}
									stroke={url("gStroke2")}
									strokeWidth="8"
									strokeLinejoin="round"
									mask={url("mInside")}
								/>
							</g>
						</svg>
						<canvas
							ref={canvasRef}
							className="ls-embers pointer-events-none absolute z-[2]"
							aria-hidden="true"
							tabIndex={-1}
						/>
					</div>

					<div className="ls-copy">
						<h1 className="ls-status">
							<span className="ls-status-stack">
								<span className="ls-sizer" aria-hidden="true">
									{t("loadingYourWorkspace", "Loading your workspace")}
								</span>
								<span id={id("headline")} aria-live="polite">
									{headline}
								</span>
							</span>
							<span
								className="ls-pct font-mono tabular-nums text-muted-foreground"
								role="progressbar"
								tabIndex={-1}
								aria-labelledby={id("headline")}
								aria-valuemin={0}
								aria-valuemax={100}
								aria-valuenow={indeterminate ? undefined : Math.round(clamped)}
								aria-valuetext={
									indeterminate ? t("loading", "Loading…") : undefined
								}
							>
								<span ref={percentRef} className="ls-pct-n">
									0
								</span>
								<span className="ls-pct-unit">%</span>
							</span>
						</h1>

						<div ref={railRef} className="ls-rail">
							<span className="ls-rail-track" aria-hidden="true" />
							<span className="ls-rail-wire" aria-hidden="true" />
							<ol aria-label={t("loadingStartupSteps", "Startup steps")}>
								{STEPS.map(([key, fallback], index) => (
									<li
										key={key}
										className="ls-step"
										ref={(element) => {
											stepRefs.current[index] = element;
										}}
									>
										<span className="ls-pin" aria-hidden="true" />
										<span className="ls-ord font-mono tabular-nums">
											{String(index + 1).padStart(2, "0")}
										</span>
										<span className="ls-step-name">{t(key, fallback)}</span>
									</li>
								))}
							</ol>
						</div>
					</div>
				</section>

				<footer className="ls-bottom">
					<p className="ls-tip text-muted-foreground" data-swap={tipSwap}>
						<span className="ls-tip-label font-mono">
							<span className="ls-sizer" aria-hidden="true">
								{t("didYouKnow", "Did you know?")}
							</span>
							<span>{tipLabel}</span>
						</span>
						<span className="ls-tip-text">{card.text}</span>
					</p>
				</footer>
			</div>

			<style>{LOADING_SCREEN_CSS}</style>
		</div>
	);
}
