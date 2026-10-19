// Client side of the "01 / Company applications" section: boots the Descent
// scene lazily, maps the pinned scroll to scene progress, keeps the
// server-rendered controls and copy in step with the scene's scenario events,
// and falls back to the static composition when WebGL is unavailable.
import type {
	CreateScene,
	DescentData,
	DescentSafeArea,
	DescentScenarioEvent,
	DescentScene,
} from "./types";

export type DescentMode = "pending" | "motion" | "reduced" | "fallback";
export type ActiveMode = Exclude<DescentMode, "pending">;
type SceneMode = Exclude<ActiveMode, "fallback">;

export interface DescentController {
	readonly root: HTMLElement;
	readonly track: HTMLElement;
	readonly board: HTMLElement;
	/** Resolves once the first scene (or the fallback) is in place and drawn with final fonts. */
	readonly settled: Promise<void>;
	mode(): DescentMode;
	scene(): DescentScene | null;
	enterMode(next: ActiveMode): Promise<void>;
	selectApp(index: number): void;
	selectScenario(index: number): void;
	applyProgress(p: number, intro: number): void;
	/** Ends autoplay and lets the layer copy announce changes, as a first user action does. */
	interact(): void;
	refresh(): void;
	/** The insets last handed to the scene, or null before the first scene. */
	safeArea(): DescentSafeArea | null;
}

type SceneModule = typeof import("./scene");

const HEADER_LINE = 64;
/** The header's bottom edge once the page has scrolled (it is 64px tall only at the top). */
const SCROLLED_HEADER_LINE = 56;
/** The light-to-dark gradient over the reduced-motion still is mostly dark this far above it (v5-apps.astro). */
const SEAM_DARK = 76;
const LOAD_MARGIN = "150% 0px";
/** Reading time (ms) after a scenario completes; the scene holds its end state longer (beneath-plan HOLD). */
const AUTOPLAY_DELAY = 3500;
/** Progress a keyboard focus entering the hidden band scrolls to, so it lands revealed. */
const REVEAL_AT = 0.92;
const CONTEXT_GRACE = 3000;
const SAFE_GAP = 16;
/** CSS px above the band over which its scrim fades in. */
const SCRIM_LEAD = 44;
/** Milliseconds a changed story's copy waits, faded out, while the old route dims (beneath SWAP.out). */
const COPY_SWAP_MS = 140;
/**
 * Clear CSS px between the picture and the copy band (the composition adds
 * its own margin): a share of the stage's height, less on narrow stages.
 */
const AIR = { height: 0.062, width: 0.045, min: 20, max: 56 } as const;
export const bandAir = (width: number, height: number) =>
	clamp(Math.min(height * AIR.height, width * AIR.width), AIR.min, AIR.max);
/** Progress over which the copy band fades in; the scene settles its picture up over the same range (shared.ts TIMELINE.band). */
export const BAND_IN = [0.46, 0.62] as const;
/** Share of the settle the picture covers before the band starts to show, so the two never meet. */
export const BAND_SHOW = 0.4;
/** CSS px the band rises as it fades in. */
const BAND_TRAVEL = 16;
/**
 * Progress over which the dock leaves with the sub-head, before the floor
 * turns to glass: beneath, the windows are the app selector.
 */
const DOCK_OUT = [0.08, 0.17] as const;
/** CSS px the dock sinks as it leaves. */
const DOCK_TRAVEL = 14;
const PROGRESS_ID = "descent-progress";
const PASSIVE_KEYS = new Set(["Tab", "Shift", "Control", "Alt", "Meta"]);
const LANDED: Keyframe[] = [
	{ boxShadow: "0 0 0 1px rgb(251 86 45 / .9), 0 0 22px rgb(251 86 45 / .55)" },
	{ boxShadow: "0 0 0 1px rgb(251 86 45 / 0), 0 0 22px rgb(251 86 45 / 0)" },
];

const clamp = (v: number, a = 0, b = 1) => Math.min(b, Math.max(a, v));
const smooth = (a: number, b: number, v: number) => {
	const t = clamp((v - a) / (b - a));
	return t * t * (3 - 2 * t);
};

let sceneModule: Promise<SceneModule> | null = null;
const loadSceneModule = (): Promise<SceneModule> => {
	sceneModule ??= import("./scene");
	return sceneModule;
};

function setHeaderTheme(dark: boolean): void {
	const root = document.documentElement;
	if (dark) root.dataset.headerTheme = "dark";
	else delete root.dataset.headerTheme;
}

function setText(el: HTMLElement, value: string): void {
	if (el.textContent !== value) el.textContent = value;
}

function readData(root: HTMLElement): DescentData | null {
	const raw = root.querySelector("[data-v5-descent-data]")?.textContent;
	if (!raw) return null;
	try {
		return JSON.parse(raw) as DescentData;
	} catch {
		return null;
	}
}

const PARTS = {
	stage: "[data-stage]",
	host: "[data-scene]",
	above: "[data-above]",
	subHead: "[data-sub-head]",
	cue: "[data-cue]",
	below: "[data-below]",
	dock: "[data-dock]",
	dockRail: "[data-dock-rail]",
	gallery: "[data-gallery]",
	veilTop: "[data-veil-top]",
	veilBottom: "[data-veil-bottom]",
	scrim: "[data-scrim]",
	board: "[data-board]",
	graph: "[data-graph]",
	layerText: "[data-layer-text]",
	copy: "[data-copy]",
	labelEl: "[data-role='label']",
	titleEl: "[data-role='title']",
	detailEl: "[data-role='detail']",
	routeEl: "[data-role='route']",
	statusEl: "[data-role='status']",
	scenarioWrap: "[data-scenarios]",
} as const;

type Parts = Record<keyof typeof PARTS, HTMLElement>;

function findParts(root: HTMLElement): Parts | null {
	const parts: Partial<Parts> = {};
	for (const [name, selector] of Object.entries(PARTS) as [
		keyof Parts,
		string,
	][]) {
		const el = root.querySelector<HTMLElement>(selector);
		if (!el) {
			console.warn(`Descent: missing ${selector} in the apps section`);
			return null;
		}
		parts[name] = el;
	}
	return parts as Parts;
}

interface RouteView {
	show(steps: readonly string[], lit: boolean): void;
	/** Lights the pills up to `step`, as the packet reaches it. */
	light(step: number): void;
	/** Flashes the destination once it is lit. */
	arrive(): void;
	fill(): void;
	clear(): void;
}

// The route pills: lit step by step while a scenario runs, all lit in the
// still and static compositions.
function createRouteView(el: HTMLElement): RouteView {
	let pills = [...el.querySelectorAll<HTMLElement>(":scope > b")];
	let lit = pills.length;
	const paint = () => {
		for (const [i, pill] of pills.entries()) {
			pill.toggleAttribute("data-lit", i < lit);
		}
	};
	return {
		show(steps, on) {
			el.textContent = "";
			pills = steps.map((step, i) => {
				if (i > 0) {
					const arrow = document.createElement("i");
					arrow.textContent = "→";
					arrow.setAttribute("aria-hidden", "true");
					el.append(arrow);
				}
				const pill = document.createElement("b");
				pill.textContent = step;
				el.append(pill);
				return pill;
			});
			lit = on ? pills.length : 0;
			paint();
		},
		light(step) {
			lit = Math.max(lit, Math.min(step + 1, pills.length));
			paint();
		},
		arrive() {
			// The last hop and the arrival can land in the same frame, in either order.
			requestAnimationFrame(() => {
				const last = pills.at(-1);
				if (last && lit >= pills.length) {
					last.animate(LANDED, { duration: 900, easing: "ease-out" });
				}
			});
		},
		fill() {
			lit = pills.length;
			paint();
		},
		clear() {
			lit = 0;
			paint();
		},
	};
}

interface StatusView {
	running(): void;
	done(): void;
}

function createStatus(
	el: HTMLElement,
	running: string,
	done: string,
): StatusView {
	const text = el.querySelector<HTMLElement>("span") ?? el;
	const set = (state: "running" | "done", value: string) => {
		el.dataset.state = state;
		setText(text, value);
	};
	return {
		running: () => set("running", running),
		done: () => set("done", done),
	};
}

interface Progress {
	start(button: HTMLElement | undefined, seconds: number): void;
	finish(): void;
	hold(paused: boolean): void;
	stop(): void;
}

// The active tab fills for the scenario's duration: its tint slides in from
// the left with a soft leading edge, so the tab never shows two flat colours.
function createProgress(): Progress {
	let bar: HTMLElement | null = null;
	let run: Animation | null = null;
	const stop = () => {
		run?.cancel();
		run = null;
		bar?.removeAttribute("data-on");
		bar = null;
	};
	return {
		start(button, seconds) {
			stop();
			const next = button?.querySelector<HTMLElement>(".sc-bar");
			const fill = next?.firstElementChild;
			if (!next || !(fill instanceof HTMLElement) || !(seconds > 0)) return;
			bar = next;
			bar.setAttribute("data-on", "");
			run = fill.animate(
				[{ transform: "translateX(-100%)" }, { transform: "translateX(0)" }],
				{
					duration: seconds * 1000,
					easing: "linear",
					fill: "forwards",
					id: PROGRESS_ID,
				},
			);
		},
		finish() {
			if (run && run.playState !== "finished") run.finish();
		},
		hold(paused) {
			if (paused && run?.playState === "running") run.pause();
			else if (!paused && run?.playState === "paused") run.play();
		},
		stop,
	};
}

interface Autoplay {
	readonly stopped: boolean;
	/** The first user action ends autoplay for good. */
	stop(): void;
	/** Focus or a pointer in the band or the dock holds the current scenario. */
	hold(reason: string, on: boolean): void;
	/** Moves on a reading pause after a scenario completes, until every scenario has played once. */
	completed(): void;
	cancel(): void;
}

function createAutoplay(
	ready: () => boolean,
	advance: () => void,
	rounds: number,
): Autoplay {
	let stopped = false;
	let timer = 0;
	let advanced = 0;
	const holds = new Set<string>();
	const cancel = () => {
		window.clearTimeout(timer);
		timer = 0;
	};
	const allowed = () => !stopped && holds.size === 0 && ready();
	return {
		get stopped() {
			return stopped;
		},
		stop() {
			stopped = true;
			cancel();
		},
		hold(reason, on) {
			if (!on) {
				holds.delete(reason);
				return;
			}
			holds.add(reason);
			cancel();
		},
		completed() {
			cancel();
			if (!allowed() || advanced >= rounds) return;
			timer = window.setTimeout(() => {
				timer = 0;
				if (!allowed()) return;
				advanced += 1;
				advance();
			}, AUTOPLAY_DELAY);
		},
		cancel,
	};
}

// The inline script in the section pins the track as "pending" before the
// stage is parsed and drops it to the static composition on window load
// unless this module has marked the track as booted.
export function bootDescent(root: HTMLElement): DescentController | null {
	const found = root.querySelector<HTMLElement>("[data-descent]");
	if (!found || "booted" in found.dataset) return null;
	found.dataset.booted = "";
	const parsed = readData(root);
	const parts = findParts(root);
	if (!parsed || !parts) {
		found.dataset.mode = "fallback";
		return null;
	}
	const track: HTMLElement = found;
	const data: DescentData = parsed;
	const {
		stage,
		host,
		above,
		subHead,
		cue,
		below,
		dock,
		dockRail,
		gallery,
		veilTop,
		veilBottom,
		scrim,
		board,
		graph,
		layerText,
		copy,
		labelEl,
		titleEl,
		detailEl,
		routeEl,
		statusEl,
		scenarioWrap,
	} = parts;
	// The dock lists the apps in the arc's order, left to right like their
	// windows; each button names its app.
	const appButtons = [...dock.querySelectorAll<HTMLButtonElement>("button")];
	const appOf = (b: HTMLElement) => Number(b.dataset.app);
	const buttonOf = (app: number) => appButtons.find((b) => appOf(b) === app);
	const scenarioButtons = [
		...scenarioWrap.querySelectorAll<HTMLButtonElement>("button"),
	];
	const windows = [...gallery.querySelectorAll<HTMLElement>("[data-app]")];
	const styled = [above, subHead, below, scrim, veilTop, veilBottom, dock];
	const reducedQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
	const route = createRouteView(routeEl);
	const status = createStatus(
		statusEl,
		data.strings.processRunning ?? "",
		data.strings.governed ?? "",
	);
	const progress = createProgress();

	const state = { app: 0, scenario: data.appFlowIndex[0] ?? 0 };
	let mode: DescentMode =
		track.dataset.mode?.startsWith("pending") &&
		typeof WebGL2RenderingContext !== "undefined"
			? "pending"
			: "fallback";
	let scene: DescentScene | null = null;
	let sceneDark = false;
	let eventsSeen = false;
	let headerDark = false;
	let revealed = false;
	let inView = false;
	let ticking = false;
	let generation = 0;
	let requested = false;
	let lostTimer = 0;
	let layoutFrame = 0;
	let copySwap = 0;
	/** Route pill changes that arrive while the copy is swapped out, applied with the new copy. */
	let routeQueue: (() => void)[] = [];
	let lastArea = "";
	let applied: DescentSafeArea | null = null;
	let resolveSettled: () => void = () => {};
	const settled = new Promise<void>((resolve) => {
		resolveSettled = resolve;
	});
	const preferredMode = (): SceneMode =>
		reducedQuery.matches ? "reduced" : "motion";
	const autoplay = createAutoplay(
		() => mode === "motion" && revealed && inView && !document.hidden,
		() => selectScenario((state.scenario + 1) % data.scenarios.length),
		data.scenarios.length - 1,
	);

	const publishHeader = (dark: boolean) => {
		if (dark === headerDark) return;
		headerDark = dark;
		setHeaderTheme(dark);
	};

	// The pin travels the track height minus the sticky stage, which is 100svh
	// and so can be shorter than the viewport while a phone's toolbar is hidden.
	const pinSpan = (r: DOMRect) => Math.max(1, r.height - stage.offsetHeight);

	const measure = () => {
		const r = track.getBoundingClientRect();
		return {
			intro: clamp(1 - r.top / window.innerHeight),
			p: clamp(-r.top / pinSpan(r)),
		};
	};

	// Whether viewport height y lies between `first`'s top edge, raised by
	// `lead`, and `last`'s bottom edge.
	const spans = (first: Element, last: Element, y: number, lead = 0) =>
		first.getBoundingClientRect().top <= y + lead &&
		last.getBoundingClientRect().bottom > y;

	// Whether the dark operating layer is what shows at viewport height y: the
	// static board and band, the still with the dark end of the gradient above
	// it, or the scene once its picture has turned dark.
	function darkAt(y: number): boolean {
		if (mode === "fallback") return spans(board, below, y);
		if (mode === "reduced") return spans(host, host, y, SEAM_DARK);
		return sceneDark && spans(track, track, y);
	}

	// The header and the page scrollbar follow the surface under them. The
	// landing layout reads --apps-scrollbar on scroll, so a change nudges it.
	// The static board meets the gallery at a hard edge, so it is tested at the
	// scrolled header's own bottom edge; the scene's dark view keeps the line
	// the header bridges to the dark section after it.
	function publishSurface() {
		publishHeader(
			darkAt(mode === "fallback" ? SCROLLED_HEADER_LINE : HEADER_LINE),
		);
		const surface = darkAt(window.innerHeight / 2) ? "dark" : "light";
		if (root.dataset.surface === surface) return;
		root.dataset.surface = surface;
		window.dispatchEvent(new Event("scroll"));
	}

	// The scene reports the frame its image turns dark (onTheme), so the
	// controls and the surface signals switch exactly when the picture does.
	function applyTheme() {
		stage.dataset.theme = sceneDark ? "dark" : "light";
		publishSurface();
	}

	// The band takes pointer input once revealed. Its copy and scenario rows
	// stay in the tab order and the accessibility tree throughout, so a dock
	// choice is announced and Tab reaches the scenarios from the dock.
	function setRevealed(on: boolean) {
		revealed = on;
		below.classList.toggle("on", on);
		if (!on) autoplay.cancel();
	}

	/** Scrolls the pinned descent to progress `at`. */
	function scrollToProgress(at: number) {
		const r = track.getBoundingClientRect();
		window.scrollTo({
			top: window.scrollY + r.top + at * pinSpan(r),
			behavior: "instant",
		});
	}

	/** Whether the stage has scrolled up past the end of its pin. */
	const pastPin = () => stage.getBoundingClientRect().top < -1;

	// Keyboard focus entering the band before it shows scrolls the descent to
	// its end, so the focus ring lands on visible controls; focus coming back
	// from below the section pins the stage again, so its window row is seen.
	function revealForFocus() {
		if (mode !== "motion") return;
		if (pastPin()) scrollToProgress(1);
		else if (!revealed) scrollToProgress(REVEAL_AT);
	}

	// Beneath, an app's button is not seen: its window is, and carries the
	// focus ring. The browser scrolls the button itself into view, which can
	// leave the window row above the screen, so once it has, the whole still,
	// or the pinned stage, is brought into view.
	function showWindowsForFocus() {
		requestAnimationFrame(() => {
			if (mode === "motion") {
				if (pastPin()) scrollToProgress(1);
				return;
			}
			if (mode !== "reduced") return;
			const r = host.getBoundingClientRect();
			if (r.top < -1 || r.bottom > window.innerHeight + 1) {
				window.scrollBy({ top: r.top, behavior: "instant" });
			}
		});
	}

	// The band fades in and rises a little once the picture has mostly settled
	// up to make room for it.
	function raiseBand(belowIn: number) {
		below.style.opacity = String(smooth(BAND_SHOW, 1, belowIn));
		below.style.transform = `translateY(${BAND_TRAVEL * (1 - belowIn)}px)`;
		setRevealed(belowIn > 0.7);
	}

	// The dock serves the surface. Beneath, the windows are the app selector:
	// the dock's buttons stay in the tab order and ring their window on focus.
	function sinkDock(p: number) {
		const out = smooth(DOCK_OUT[0], DOCK_OUT[1], p);
		dock.style.opacity = String(1 - out);
		dock.style.translate = `0 ${DOCK_TRAVEL * out}px`;
		dock.toggleAttribute("data-away", out > 0.5);
	}

	function applyProgress(p: number, intro: number) {
		const aboveOut = smooth(0.03, 0.14, p);
		above.style.opacity = String(1 - aboveOut);
		above.style.transform = `translateY(${-28 * aboveOut}px)`;
		subHead.style.opacity = String(smooth(0.25, 0.9, intro));
		sinkDock(p);
		const belowIn = smooth(BAND_IN[0], BAND_IN[1], p);
		raiseBand(belowIn);
		scrim.style.opacity = String(smooth(BAND_SHOW / 2, 1, belowIn));
		veilTop.style.opacity = String(1 - smooth(0, 0.06, p));
		veilBottom.style.opacity = String(smooth(0.8, 1, p));
		applyTheme();
	}

	const press = (
		buttons: readonly HTMLButtonElement[],
		on: (b: HTMLButtonElement, i: number) => boolean,
	) => {
		for (const [i, b] of buttons.entries()) {
			b.setAttribute("aria-pressed", String(on(b, i)));
		}
	};

	// A dock wider than its room scrolls and fades out at its edges; measured on
	// the buttons, so the wider padding the fade adds cannot feed back into it.
	function syncDock() {
		const first = appButtons[0];
		const last = appButtons.at(-1);
		if (!first || !last) return;
		const need = last.offsetLeft + last.offsetWidth - first.offsetLeft + 12;
		const over = need > dockRail.clientWidth + 1;
		if (over === dock.hasAttribute("data-overflow")) return;
		dock.toggleAttribute("data-overflow", over);
		centre(dockRail, buttonOf(state.app));
	}

	// Scrolls a horizontal scroller so the item sits in its middle.
	function centre(scroller: HTMLElement, item: HTMLElement | undefined) {
		if (!item || scroller.scrollWidth <= scroller.clientWidth + 2) return;
		scroller.scrollTo({
			left: item.offsetLeft - (scroller.clientWidth - item.offsetWidth) / 2,
			behavior: reducedQuery.matches ? "auto" : "smooth",
		});
	}

	// The copy sits in a live region: only real changes may touch it, so the
	// server-rendered state is not announced again when the scene boots.
	function renderCopy() {
		const sc = data.scenarios[state.scenario];
		const app = data.apps[state.app];
		if (!sc || !app) return;
		setText(labelEl, `${app.name} / ${sc.label}`);
		setText(titleEl, sc.title);
		setText(detailEl, sc.detail);
		const shown = String(state.scenario);
		if (graph.dataset.scenario !== shown) {
			graph.dataset.scenario = shown;
			route.show(sc.route, mode !== "motion" || !eventsSeen);
		}
	}

	// Route pill changes wait for the copy they belong to while it is swapped out.
	const onRoute = (change: () => void) => {
		if (copySwap) routeQueue.push(change);
		else change();
	};

	// A story change on the shown band crossfades the copy with the board: the
	// old copy fades while the old route dims, the new one arrives with the new
	// route, so no frame pairs one story's words with another's picture.
	function swapCopy() {
		window.clearTimeout(copySwap);
		routeQueue = [];
		copy.dataset.swap = "";
		copySwap = window.setTimeout(() => {
			copySwap = 0;
			renderCopy();
			const queued = routeQueue;
			routeQueue = [];
			for (const change of queued) change();
			delete copy.dataset.swap;
		}, COPY_SWAP_MS);
	}

	function renderDom() {
		press(appButtons, (b) => appOf(b) === state.app);
		press(scenarioButtons, (_, i) => i === state.scenario);
		const selected = windows.find(
			(win) => win.dataset.app === String(state.app),
		);
		for (const win of windows)
			win.toggleAttribute("data-selected", win === selected);
		const story = graph.dataset.scenario !== String(state.scenario);
		if (story && mode === "motion" && revealed && !reducedQuery.matches) {
			swapCopy();
		} else if (!copySwap) {
			renderCopy();
		}
		centre(dockRail, buttonOf(state.app));
		if (mode === "fallback") {
			centre(gallery, selected);
			centreRoute();
		}
	}

	// A static board wider than its viewport (phones) scrolls to the scenario's
	// route, or to the node the route lands on when the route is wider still.
	function centreRoute() {
		const viewport = graph.parentElement;
		if (!viewport || viewport.scrollWidth <= viewport.clientWidth + 2) return;
		const on = `[data-on~="${state.scenario}"]`;
		const nodes = [...graph.querySelectorAll<HTMLElement>(`.fb-node${on}`)];
		if (nodes.length === 0) return;
		const origin = viewport.getBoundingClientRect().left - viewport.scrollLeft;
		const spanOf = (list: readonly HTMLElement[]) => {
			const rects = list.map((node) => node.getBoundingClientRect());
			return {
				left: Math.min(...rects.map((r) => r.left)) - origin,
				right: Math.max(...rects.map((r) => r.right)) - origin,
			};
		};
		let span = spanOf(nodes);
		const room = viewport.clientWidth;
		const landing = graph.querySelector<HTMLElement>(
			`.fb-node[data-land~="${state.scenario}"]`,
		);
		if (span.right - span.left > room && landing) span = spanOf([landing]);
		viewport.scrollTo({
			left: (span.left + span.right) / 2 - room / 2,
			behavior: reducedQuery.matches ? "auto" : "smooth",
		});
	}

	// A changed story stops the old one's progress at once, so two tabs are
	// never filled together.
	function play(story: boolean) {
		autoplay.cancel();
		if (story) progress.stop();
		renderDom();
		scene?.selectApp(state.app);
		scene?.setScenario(state.scenario);
		if (mode === "reduced") scene?.render();
	}

	function selectApp(index: number) {
		const before = state.scenario;
		state.app = index;
		state.scenario = data.appFlowIndex[index] ?? state.scenario;
		play(state.scenario !== before);
	}

	function selectScenario(index: number) {
		const before = state.scenario;
		state.scenario = index;
		state.app = data.scenarioAppIndex[index] ?? state.app;
		play(state.scenario !== before);
	}

	function onScenarioEvent(event: DescentScenarioEvent) {
		if (event.scenario !== state.scenario) return;
		eventsSeen = true;
		if (event.type === "start") {
			onRoute(() => route.clear());
			status.running();
			progress.start(scenarioButtons[event.scenario], event.duration);
		} else if (event.type === "hop") {
			const { step } = event;
			onRoute(() => route.light(step));
		} else if (event.type === "arrive") {
			onRoute(() => route.arrive());
		} else {
			onRoute(() => route.fill());
			status.done();
			progress.finish();
			autoplay.completed();
		}
	}

	// From the first user action on, changes are the visitor's own: autoplay
	// stops and the copy is announced.
	function interact() {
		if (autoplay.stopped) return;
		autoplay.stop();
		copy.setAttribute("aria-live", "polite");
	}

	// Invisible copies of every scenario's copy keep the band as tall as its
	// longest scenario, so its top edge and the safe area above it stay put.
	function buildGhosts() {
		const longest = data.apps.reduce(
			(name, app) => (app.name.length > name.length ? app.name : name),
			"",
		);
		for (const sc of data.scenarios) {
			const ghost = copy.cloneNode(true) as HTMLElement;
			const fill = (role: string, value: string) => {
				const el = ghost.querySelector<HTMLElement>(`[data-role='${role}']`);
				if (el) el.textContent = value;
			};
			fill("label", `${longest} / ${sc.label}`);
			fill("title", sc.title);
			fill("detail", sc.detail);
			const ghostRoute = ghost.querySelector<HTMLElement>(
				"[data-role='route']",
			);
			if (ghostRoute) createRouteView(ghostRoute).show(sc.route, true);
			for (const el of ghost.querySelectorAll("[data-role]")) {
				el.removeAttribute("data-role");
			}
			ghost.removeAttribute("data-copy");
			ghost.removeAttribute("aria-live");
			ghost.setAttribute("aria-hidden", "true");
			ghost.classList.add("layer-ghost");
			layerText.append(ghost);
		}
	}

	// Top of an element's layout box within the stage; transforms do not move it.
	function layoutTop(el: HTMLElement): number {
		let y = 0;
		for (
			let node: Element | null = el;
			node instanceof HTMLElement && node !== stage;
			node = node.offsetParent
		) {
			y += node.offsetTop;
		}
		return y;
	}

	// Insets of the header, the bottom chrome and the band, read from layout
	// boxes so the scroll-driven transforms of the copy do not move them.
	function measureSafeArea(): DescentSafeArea {
		const top = layoutTop(host);
		const height = host.offsetHeight;
		const fromTop = (y: number) => Math.round(clamp(y - top, 0, height));
		const fromBottom = (y: number) =>
			Math.round(clamp(top + height - y, 0, height));
		const subHeadBottom = layoutTop(subHead) + subHead.offsetHeight;
		const dockTop = layoutTop(dock);
		const chromeTop = cue.offsetParent
			? Math.min(layoutTop(cue), dockTop)
			: dockTop;
		const head = HEADER_LINE + SAFE_GAP / 2;
		const bandTop = layoutTop(below);
		const air = bandAir(host.offsetWidth, height);
		return {
			surface: {
				top: Math.max(head, fromTop(subHeadBottom) + SAFE_GAP),
				bottom: fromBottom(chromeTop) + SAFE_GAP,
			},
			open: {
				top: head,
				bottom: fromBottom(bandTop + below.offsetHeight) + SAFE_GAP,
			},
			beneath: { top: head, bottom: fromBottom(bandTop) + Math.round(air) },
		};
	}

	// The scrim darkens only what the band covers, from a short lead-in above
	// it, so the lit route above the band keeps its full brightness.
	function sizeScrim() {
		const depth = layoutTop(host) + host.offsetHeight - layoutTop(below);
		stage.style.setProperty("--scrim-h", `${Math.round(depth + SCRIM_LEAD)}px`);
	}

	function syncSafeArea() {
		if (!scene || (mode !== "motion" && mode !== "reduced")) return;
		sizeScrim();
		const area = measureSafeArea();
		const key = JSON.stringify(area);
		if (key === lastArea) return;
		lastArea = key;
		applied = area;
		scene.setSafeArea(area);
		if (mode === "reduced") scene.render();
	}

	const scheduleLayout = () => {
		if (layoutFrame) return;
		layoutFrame = requestAnimationFrame(() => {
			layoutFrame = 0;
			syncDock();
			syncSafeArea();
		});
	};

	function update() {
		ticking = false;
		if (mode === "motion" && scene) {
			const { p, intro } = measure();
			scene.setProgress(p);
			scene.setIntro(intro);
			applyProgress(p, intro);
			return;
		}
		publishSurface();
	}

	function onScroll() {
		if (ticking) return;
		ticking = true;
		requestAnimationFrame(update);
	}

	function onResize() {
		if (scene) {
			scene.resize();
			if (mode === "reduced") scene.render();
		}
		onScroll();
	}

	function setRunning(on: boolean) {
		if (!scene) return;
		if (on) scene.resume();
		else scene.pause();
		progress.hold(!on);
	}

	const visibility = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				inView = entry.isIntersecting;
				if (scene && mode === "motion") setRunning(inView && !document.hidden);
			}
		},
		{ rootMargin: "120px 0px" },
	);

	function onVisibility() {
		if (scene && mode === "motion") setRunning(inView && !document.hidden);
	}

	function disposeScene() {
		window.clearTimeout(lostTimer);
		lostTimer = 0;
		progress.stop();
		autoplay.cancel();
		scene?.dispose();
		scene = null;
		sceneDark = false;
	}

	function startScene(mod: SceneModule, reduced: boolean) {
		disposeScene();
		eventsSeen = false;
		lastArea = "";
		const create: CreateScene = mod.createScene;
		let created: DescentScene | null = null;
		try {
			created = create(host, data, {
				reducedMotion: reduced,
				onSelectApp: selectApp,
				// Beneath, the windows are the app selector: resting on one holds
				// the story, as the dock does at the surface.
				onHoverApp: (index) => autoplay.hold("windows-pointer", index >= 0),
				onTheme: (theme) => {
					sceneDark = theme === "dark";
					applyTheme();
				},
				onScenarioEvent,
			});
		} catch (error) {
			console.warn(
				"Descent: scene creation failed, using the static version",
				error,
			);
		}
		scene = created;
		created?.selectApp(state.app);
		created?.setScenario(state.scenario);
		return created;
	}

	async function importScene() {
		try {
			return await loadSceneModule();
		} catch (error) {
			console.warn("Descent: scene module failed to load", error);
			return null;
		}
	}

	// Pinned motion drives the band from the scroll; the still and the static
	// compositions show it whole, at the scenario's end state.
	function shapeMode() {
		track.dataset.mode = mode;
		if (mode === "motion") {
			setRevealed(false);
			return;
		}
		for (const el of styled) el.removeAttribute("style");
		dock.removeAttribute("data-away");
		setRevealed(true);
		route.fill();
		status.done();
		progress.stop();
		if (mode === "fallback") stage.dataset.theme = "light";
	}

	// A scene created mid-scroll starts where the page is instead of easing in
	// from the top of the pin; the still always shows the operating layer.
	function activate(next: SceneMode, created: DescentScene) {
		mode = next;
		shapeMode();
		if (next === "reduced") {
			created.setProgress(1, true);
			created.setIntro(1, true);
			created.snap();
		}
		update();
		if (next === "motion") {
			created.snap();
			visibility.observe(stage);
		}
		renderDom();
		syncSafeArea();
		created.render();
		return created.ready.then(syncSafeArea, (error: unknown) => {
			console.warn(
				"Descent: redrawing faces with the page fonts failed",
				error,
			);
		});
	}

	function showFallback() {
		disposeScene();
		mode = "fallback";
		shapeMode();
		renderDom();
		publishSurface();
	}

	// The track takes the pinned or still layout before the scene is created,
	// so the canvas is sized to the box it will fill.
	async function enterMode(next: ActiveMode): Promise<void> {
		generation += 1;
		const run = generation;
		visibility.disconnect();
		if (next !== "fallback") {
			const mod = await importScene();
			if (run !== generation) return;
			track.dataset.mode = next;
			const created = mod ? startScene(mod, next === "reduced") : null;
			if (created) {
				await activate(next, created);
				resolveSettled();
				return;
			}
		}
		showFallback();
		resolveSettled();
	}

	// A context that is lost and not restored within the grace period is gone
	// for good (GPU reset, driver crash): the static composition takes over.
	function onContextLost(event: Event) {
		if (!scene || event.target !== host.querySelector("canvas")) return;
		window.clearTimeout(lostTimer);
		lostTimer = window.setTimeout(() => {
			lostTimer = 0;
			console.warn(
				"Descent: the WebGL context did not come back, using the static version",
			);
			generation += 1;
			visibility.disconnect();
			showFallback();
		}, CONTEXT_GRACE);
	}

	function onContextRestored() {
		window.clearTimeout(lostTimer);
		lostTimer = 0;
	}

	const onKey = (event: KeyboardEvent) => {
		if (!PASSIVE_KEYS.has(event.key)) interact();
	};
	// Keyboard focus on an app's button rings its window, which beneath is the
	// only visible selector; a still repaints for it.
	function focusWindow(index: number) {
		if (!scene) return;
		scene.focusApp(index);
		if (mode === "reduced") scene.render();
	}
	for (const b of appButtons) {
		const app = appOf(b);
		b.addEventListener("click", () => {
			interact();
			selectApp(app);
		});
		b.addEventListener("keydown", onKey);
		// The browser's own focus scroll can leave a button half under the
		// rail's faded edge; this brings it inside the rail's padding. Beneath,
		// where the dock is not seen, its window's row is brought into view.
		b.addEventListener("focus", () => {
			if (dock.hasAttribute("data-away") || mode === "reduced") {
				showWindowsForFocus();
			} else {
				b.scrollIntoView({ block: "nearest", inline: "nearest" });
			}
			if (b.matches(":focus-visible")) focusWindow(app);
		});
		b.addEventListener("blur", () => focusWindow(-1));
	}
	for (const [i, b] of scenarioButtons.entries()) {
		b.addEventListener("click", () => {
			interact();
			selectScenario(i);
		});
		b.addEventListener("keydown", onKey);
	}
	// A touch that starts a scroll is not a choice; taps arrive as clicks.
	stage.addEventListener(
		"pointerdown",
		(event) => {
			if (event.pointerType !== "touch") interact();
		},
		{ capture: true, passive: true },
	);
	stage.addEventListener("click", interact, { capture: true });
	// Focus or a pointer in the band or the dock holds the current scenario
	// while the visitor reads or chooses.
	function holdWithin(el: HTMLElement, name: string) {
		el.addEventListener("focusin", () => autoplay.hold(`${name}-focus`, true));
		el.addEventListener("focusout", (event) => {
			if (!el.contains(event.relatedTarget as Node | null)) {
				autoplay.hold(`${name}-focus`, false);
			}
		});
		el.addEventListener("pointerenter", () =>
			autoplay.hold(`${name}-pointer`, true),
		);
		el.addEventListener("pointerleave", () =>
			autoplay.hold(`${name}-pointer`, false),
		);
	}
	holdWithin(below, "band");
	holdWithin(dock, "dock");
	below.addEventListener("focusin", revealForFocus);
	// The controls are disabled in the markup, so without JavaScript they are
	// not focusable controls that do nothing.
	for (const b of [...appButtons, ...scenarioButtons]) b.disabled = false;
	host.addEventListener("webglcontextlost", onContextLost, true);
	host.addEventListener("webglcontextrestored", onContextRestored, true);
	window.addEventListener("scroll", onScroll, { passive: true });
	window.addEventListener("resize", onResize);
	document.addEventListener("visibilitychange", onVisibility);
	document.fonts.addEventListener("loadingdone", scheduleLayout);
	const layoutWatch = new ResizeObserver(scheduleLayout);
	for (const el of [stage, subHead, below, dock, cue]) layoutWatch.observe(el);
	reducedQuery.addEventListener("change", () => {
		if (requested && mode !== "fallback") void enterMode(preferredMode());
	});

	buildGhosts();
	renderDom();
	if (mode === "fallback") {
		showFallback();
		resolveSettled();
	} else {
		const loader = new IntersectionObserver(
			(entries) => {
				if (!entries.some((entry) => entry.isIntersecting)) return;
				loader.disconnect();
				requested = true;
				void enterMode(preferredMode());
			},
			{ rootMargin: LOAD_MARGIN },
		);
		loader.observe(root);
	}

	return {
		root,
		track,
		board,
		settled,
		mode: () => mode,
		scene: () => scene,
		enterMode,
		selectApp,
		selectScenario,
		applyProgress,
		interact,
		refresh: update,
		safeArea: () => applied,
	};
}
