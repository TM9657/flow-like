import "@xyflow/react/dist/style.css";
import "../../packages/ui/global.css";
import "./style.css";
import { isWebkitLite } from "../../packages/ui/lib/platform";
import mediumFontUrl from "./assets/Inter-Medium.ttf?url";
import regularFontUrl from "./assets/Inter-Regular.ttf?url";
import { createFixture, fixtureChecksum, initialViewport } from "./fixture";
import { measureScenario, startInputMeasurement } from "./measurement.mjs";
import { paletteColor } from "./palette";
import type {
	BoardStyle,
	InteractionTargets,
	RendererHandle,
	Viewport,
} from "./types";

const query = new URLSearchParams(location.search);
const requestedRenderer = query.get("renderer");
const rendererName =
	requestedRenderer === "egui" || requestedRenderer === "egui-lod"
		? requestedRenderer
		: "react-flow";
let view = query.get("view") === "overview" ? "overview" : "detail";
let motion = query.get("motion") === "1";
const fixture = createFixture(Number(query.get("nodes") ?? 10_000));
const checksum = fixtureChecksum(fixture);
function requiredElement<T extends Element>(
	selector: string,
	parent: ParentNode = document,
): T {
	const element = parent.querySelector<T>(selector);
	if (!element) throw new Error(`Missing benchmark element: ${selector}`);
	return element;
}
const root = requiredElement<HTMLDivElement>("#root");
root.innerHTML = `<main class="benchmark-shell">
  <header class="benchmark-toolbar">
    <strong class="benchmark-title">${fixture.nodes.length.toLocaleString()}-node board</strong>
    <a id="react-link">React Flow</a><a id="egui-link">egui</a><a id="egui-lod-link">egui + LOD</a>
    <select aria-label="Viewport" id="view"><option value="detail">Editing view</option><option value="overview">All 10,000 nodes</option></select>
    <label><input type="checkbox" id="motion" /> Connection animations</label>
    <label><input type="checkbox" id="activity" /> Activity replay</label>
    <button id="reset">Reset view</button><button id="measure">Measure pan (3 s)</button>
  </header>
  <div class="benchmark-caption">${fixture.nodes.length.toLocaleString()} nodes · ${fixture.edges.length.toLocaleString()} connections · Drag nodes, pan and zoom. To reconnect, select a wire and drag its target endpoint to an input pin. Renderer experiment with current board styling; activity is simulated.</div>
  <section class="benchmark-surface" aria-label="Interactive workflow board"><output class="benchmark-status">Loading ${rendererName}…</output></section>
</main>`;

const host = requiredElement<HTMLElement>(".benchmark-surface", root);
const output = requiredElement<HTMLOutputElement>("output", root);
const viewInput = requiredElement<HTMLSelectElement>("#view", root);
const motionInput = requiredElement<HTMLInputElement>("#motion", root);
const activityInput = requiredElement<HTMLInputElement>("#activity", root);
viewInput.value = view;
motionInput.checked = motion;
function updateLinks() {
	for (const [id, renderer] of [
		["react-link", "react-flow"],
		["egui-link", "egui"],
		["egui-lod-link", "egui-lod"],
	]) {
		const link = requiredElement<HTMLAnchorElement>(`#${id}`, root);
		const params = new URLSearchParams(query);
		params.set("renderer", renderer);
		params.set("nodes", String(fixture.nodes.length));
		params.set("view", view);
		params.set("motion", motion ? "1" : "0");
		link.href = `?${params}`;
		if (rendererName === renderer) link.setAttribute("aria-current", "page");
	}
}
updateLinks();

function readStyle(): BoardStyle {
	const computed = getComputedStyle(host);
	const rgba = (name: string) =>
		paletteColor(computed.getPropertyValue(name).trim());
	return {
		background: rgba("--background"),
		card: rgba("--card"),
		foreground: rgba("--foreground"),
		border: rgba("--border"),
		primary: rgba("--primary"),
		tertiary: rgba("--tertiary"),
		pinExecution: rgba("--foreground"),
		pinString: rgba("--pin-string"),
		pinFloat: rgba("--pin-float"),
		pinInteger: rgba("--pin-integer"),
	};
}

let renderer: RendererHandle;
let viewport: Viewport;
let stopInputMeasurement: (() => Promise<unknown>) | undefined;
let activityTimer: ReturnType<typeof setInterval> | undefined;
let tick = 0;
let busy = false;
function setActivityReplay(enabled: boolean) {
	clearInterval(activityTimer);
	activityTimer = undefined;
	activityInput.checked = enabled;
	tick = 0;
	if (enabled) {
		renderer.setActivity(tick++);
		activityTimer = setInterval(() => renderer.setActivity(tick++), 100);
	} else renderer.setActivity(0xffffffff);
}
const twoFrames = () =>
	new Promise<void>((resolve) =>
		requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
	);
function canonicalViewport() {
	return initialViewport(fixture, host.clientWidth, host.clientHeight, view);
}
async function resetView() {
	viewport = canonicalViewport();
	await renderer.setViewport(viewport);
	await twoFrames();
}

async function mountEgui(style: BoardStyle): Promise<RendererHandle> {
	const wasm = await import("./egui/pkg/board_bench.js");
	const exports = await wasm.default();
	const canvas = document.createElement("canvas");
	canvas.tabIndex = 0;
	canvas.setAttribute("aria-label", "egui workflow board");
	host.prepend(canvas);
	const handle = await wasm.start(
		canvas,
		JSON.stringify(fixture),
		JSON.stringify(style),
	);
	handle.set_webkit(isWebkitLite());
	handle.set_lod(rendererName === "egui-lod");
	const [regular, medium] = await Promise.all(
		[regularFontUrl, mediumFontUrl].map(
			async (url) => new Uint8Array(await (await fetch(url)).arrayBuffer()),
		),
	);
	handle.set_fonts(regular, medium);
	return {
		setViewport: (v) => handle.set_viewport(v.x, v.y, v.zoom),
		setMotion: (enabled) => handle.set_motion(enabled),
		setActivity: (value) => handle.set_activity(value),
		moveNodes: (indices, dx, dy) =>
			handle.move_nodes(JSON.stringify(indices), dx, dy),
		stats: () => ({
			...JSON.parse(handle.stats()),
			linearMemoryBytes: exports.memory.buffer.byteLength,
		}),
		prepareInteraction: async () => {
			const targets = JSON.parse(handle.interaction_targets());
			handle.select_edge(targets.reconnect.edgeIndex);
			await twoFrames();
			return targets;
		},
		getNodePosition: (index) => JSON.parse(handle.node_position(index)),
		getEdgeTarget: (index) => JSON.parse(handle.edge_target(index)),
		destroy: () => {
			handle.destroy();
			handle.free();
		},
	};
}

function dragIndices() {
	const cx = (host.clientWidth / 2 - viewport.x) / viewport.zoom;
	const cy = (host.clientHeight / 2 - viewport.y) / viewport.zoom;
	return fixture.nodes
		.map((node, index) => ({
			index,
			distance: Math.hypot(node.x - cx, node.y - cy),
		}))
		.sort((a, b) => a.distance - b.distance)
		.slice(0, 20)
		.map((entry) => entry.index);
}

const api = {
	status: "loading",
	error: "",
	snapshot() {
		const stats = renderer?.stats() ?? {};
		return {
			renderer: rendererName,
			production: import.meta.env.PROD,
			expectedNodeCount: fixture.nodes.length,
			expectedEdgeCount: fixture.edges.length,
			nodeCount: fixture.nodes.length,
			edgeCount: fixture.edges.length,
			fixtureChecksum: checksum,
			workload:
				"renderer-only; shared static fixture; synthetic activity replay; no backend execution",
			view,
			motion,
			activityReplay: activityTimer !== undefined,
			viewport,
			canvasCount: host.querySelectorAll("canvas").length,
			renderedNodeCount: stats.renderedNodes ?? stats.renderedNodeCount,
			...stats,
			browserViewport: {
				width: host.clientWidth,
				height: host.clientHeight,
				dpr: devicePixelRatio,
			},
		};
	},
	async setViewport(value: Viewport) {
		viewport = value;
		await renderer.setViewport(value);
	},
	setMotion(enabled: boolean) {
		motion = enabled;
		motionInput.checked = enabled;
		renderer.setMotion(enabled);
		updateLinks();
	},
	setActivity(value: number) {
		renderer.setActivity(value);
	},
	setActivityReplay,
	moveNodes(indices: number[], dx: number, dy: number) {
		renderer.moveNodes(indices, dx, dy);
	},
	async prepareInteraction(): Promise<InteractionTargets> {
		const targets = await renderer.prepareInteraction();
		const rect = host.getBoundingClientRect();
		const client = (point: { x: number; y: number }) => ({
			x: point.x + rect.left,
			y: point.y + rect.top,
		});
		return {
			node: { ...targets.node, ...client(targets.node) },
			reconnect: {
				...targets.reconnect,
				from: client(targets.reconnect.from),
				to: client(targets.reconnect.to),
			},
		};
	},
	interactionSnapshot(nodeIndex: number, edgeIndex: number) {
		return {
			node: renderer.getNodePosition(nodeIndex),
			edge: renderer.getEdgeTarget(edgeIndex),
		};
	},
	beginInputMeasurement() {
		stopInputMeasurement = startInputMeasurement("pointer-input");
	},
	async endInputMeasurement() {
		if (!stopInputMeasurement)
			throw new Error("No input measurement is running");
		return stopInputMeasurement();
	},
	async runScenario(name: string, durationMs: number) {
		if (busy) throw new Error("A scenario is already running");
		busy = true;
		let indices: number[] = [];
		let dx = 0;
		let dy = 0;
		let lastTick = -1;
		try {
			await resetView();
			const base = { ...viewport };
			if (name === "drag") indices = dragIndices();
			const centerX = host.clientWidth / 2;
			const centerY = host.clientHeight / 2;
			const result = await measureScenario({
				name,
				durationMs,
				step(elapsed: number) {
					const phase = (elapsed / durationMs) * Math.PI * 2;
					if (name === "pan")
						renderer.setViewport({
							...base,
							x: base.x + Math.sin(phase) * 180,
							y: base.y + Math.cos(phase) * 90,
						});
					else if (name === "zoom") {
						const zoom = base.zoom * (1 + Math.sin(phase) * 0.25);
						renderer.setViewport({
							zoom,
							x: centerX - ((centerX - base.x) * zoom) / base.zoom,
							y: centerY - ((centerY - base.y) * zoom) / base.zoom,
						});
					} else if (name === "drag") {
						const nextX = Math.sin(phase) * 60;
						const nextY = Math.cos(phase) * 30;
						renderer.moveNodes(indices, nextX - dx, nextY - dy);
						dx = nextX;
						dy = nextY;
					} else if (name === "activity") {
						const nextTick = Math.floor(elapsed / 100);
						if (nextTick !== lastTick) {
							renderer.setActivity(nextTick);
							lastTick = nextTick;
						}
					}
				},
			});
			return { ...result, renderer: rendererName, stats: renderer.stats() };
		} finally {
			if (dx || dy) renderer.moveNodes(indices, -dx, -dy);
			if (name === "activity") renderer.setActivity(0xffffffff);
			try {
				await resetView();
			} finally {
				busy = false;
			}
		}
	},
};
(window as unknown as { boardBenchmark: typeof api }).boardBenchmark = api;

try {
	// WebKit can evaluate the module before these stylesheets finish loading.
	await Promise.all(
		Array.from(
			document.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]'),
		).map(
			(link) =>
				link.sheet ||
				new Promise<void>((resolve, reject) => {
					link.addEventListener("load", () => resolve(), { once: true });
					link.addEventListener(
						"error",
						() =>
							reject(new Error(`Could not load board styles: ${link.href}`)),
						{ once: true },
					);
				}),
		),
	);
	await document.fonts.load('400 10px "Inter"');
	await document.fonts.load('500 10px "Inter"');
	const started = performance.now();
	renderer =
		rendererName !== "react-flow"
			? await mountEgui(readStyle())
			: await (await import("./react-renderer")).mountReactRenderer(
					host,
					fixture,
					readStyle(),
				);
	renderer.setMotion(motion);
	await resetView();
	api.status = "ready";
	output.textContent = `Ready in ${Math.round(performance.now() - started)} ms. ${motion ? "Animations forced on" : "Large-board motion policy"}. Drag a node or select a wire to reconnect.`;
	requiredElement<HTMLButtonElement>("#reset", root).addEventListener(
		"click",
		() => void resetView(),
	);
	viewInput.addEventListener("change", () => {
		view = viewInput.value;
		updateLinks();
		void resetView();
	});
	motionInput.addEventListener("change", () =>
		api.setMotion(motionInput.checked),
	);
	activityInput.addEventListener("change", () =>
		setActivityReplay(activityInput.checked),
	);
	window.addEventListener(
		"pagehide",
		() => {
			clearInterval(activityTimer);
			renderer.destroy();
		},
		{ once: true },
	);
	requiredElement<HTMLButtonElement>("#measure", root).addEventListener(
		"click",
		async () => {
			const button = requiredElement<HTMLButtonElement>("#measure", root);
			button.disabled = true;
			output.textContent = "Measuring pan for three seconds…";
			try {
				const result = await api.runScenario("pan", 3000);
				output.textContent = `Pan frame interval: p50 ${result.p50Ms?.toFixed(1)} ms · p95 ${result.p95Ms?.toFixed(1)} ms · ${Math.round(result.missedFrameRatio * 100)}% estimated missed frames. RAF timing, not GPU duration.`;
			} finally {
				button.disabled = false;
			}
		},
	);
} catch (error) {
	api.status = "error";
	api.error =
		error instanceof Error
			? error.stack?.includes(error.message)
				? error.stack
				: `${error.message}\n${error.stack ?? ""}`
			: String(error);
	output.className = "benchmark-error";
	output.textContent = api.error;
	console.error(error);
}
