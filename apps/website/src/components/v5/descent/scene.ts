import * as THREE from "three";
import { createBeneath } from "./beneath";
import { createCameraRig } from "./camera";
import { createComposer } from "./composition";
import { createFrameBudget } from "./frame-budget";
import type {
	Beneath,
	Frame,
	Rate,
	ScenarioClock,
	SceneContext,
	Stage,
	WindowCues,
} from "./internal";
import { createPost } from "./post";
import {
	TIMELINE,
	clamp,
	createReveal,
	createSharedUniforms,
	lerp,
	revealInto,
} from "./shared";
import { createSurface } from "./surface";
import type { DescentData, DescentOptions, DescentScene } from "./types";
import { configureFonts, fontSpecs } from "./ui-draw";
import { createWindows } from "./windows";

const MAX_DPR = 2;
/** Phones (portrait) render at their full density, up to 3, or the board's text goes soft. */
const MAX_DPR_PORTRAIT = 3;
/** The adaptive resolution never renders below this many device pixels per CSS pixel. */
const MIN_DPR = 0.75;
/** Seconds of running before slow frames may lower the resolution. */
const BUDGET_DELAY = 2;
/** The longest the first frame waits for the programs to compile. */
const WARM_LIMIT_MS = 3000;

type Mutable<T> = { -readonly [K in keyof T]: T[K] };

// Everything renders into the composer's target, which has its own depth
// buffer; the canvas only receives the final full-screen pass.
function createRenderer(canvas: HTMLCanvasElement) {
	const attributes: WebGLContextAttributes = {
		antialias: false,
		alpha: false,
		depth: false,
		premultipliedAlpha: true,
		stencil: false,
		powerPreference: "high-performance",
	};
	try {
		// Probing the context first keeps three from logging an error on
		// devices without WebGL2; the section then shows its static version.
		const context = canvas.getContext("webgl2", attributes);
		if (!context) return null;
		return new THREE.WebGLRenderer({ canvas, context, ...attributes });
	} catch {
		return null;
	}
}

function supportsScene(renderer: THREE.WebGLRenderer) {
	const halfFloatTargets =
		renderer.extensions.has("EXT_color_buffer_float") ||
		renderer.extensions.has("EXT_color_buffer_half_float");
	return renderer.capabilities.isWebGL2 && halfFloatTargets;
}

// Browsers cap live WebGL contexts and collect discarded ones late, so the
// context is released at once instead of waiting for garbage collection.
function releaseRenderer(renderer: THREE.WebGLRenderer) {
	renderer.dispose();
	if (!renderer.getContext().isContextLost()) renderer.forceContextLoss();
}

/** Every texture the scene's materials sample, for the warm-up upload. */
function sceneTextures(scene: THREE.Scene) {
	const found = new Set<THREE.Texture>();
	const add = (value: unknown) => {
		if (value instanceof THREE.Texture) found.add(value);
	};
	scene.traverse((object) => {
		const material = (object as Partial<THREE.Mesh>).material;
		if (!material) return;
		for (const m of Array.isArray(material) ? material : [material]) {
			add((m as Partial<THREE.MeshBasicMaterial>).map);
			const uniforms = (m as Partial<THREE.ShaderMaterial>).uniforms;
			if (uniforms) for (const u of Object.values(uniforms)) add(u.value);
		}
	});
	return found;
}

export function createScene(
	container: HTMLElement,
	data: DescentData,
	options: DescentOptions,
): DescentScene | null {
	const canvas = document.createElement("canvas");
	canvas.className = "descent-canvas";
	canvas.setAttribute("aria-hidden", "true");
	const renderer = createRenderer(canvas);
	if (!renderer) return null;
	if (!supportsScene(renderer)) {
		releaseRenderer(renderer);
		return null;
	}
	try {
		return mountScene(container, canvas, renderer, data, options);
	} catch (error) {
		releaseRenderer(renderer);
		canvas.remove();
		throw error;
	}
}

function mountScene(
	container: HTMLElement,
	canvas: HTMLCanvasElement,
	renderer: THREE.WebGLRenderer,
	data: DescentData,
	options: DescentOptions,
): DescentScene {
	const reducedMotion = options.reducedMotion;
	const cs = getComputedStyle(container);
	configureFonts({
		sans: cs.fontFamily,
		mono: cs.getPropertyValue("--mono").trim() || undefined,
		locale:
			container.closest("[lang]")?.getAttribute("lang") ||
			document.documentElement.lang ||
			undefined,
	});
	container.appendChild(canvas);
	renderer.toneMapping = THREE.NoToneMapping;
	renderer.outputColorSpace = THREE.SRGBColorSpace;
	renderer.info.autoReset = false;

	const scene = new THREE.Scene();
	const camera = new THREE.PerspectiveCamera(36, 1, 0.05, 600);
	const stage: Stage = {
		t: 0,
		frozen: false,
		progress: 0,
		progressTarget: 0,
		intro: 1,
		introTarget: 1,
		app: 0,
		scenario: 1,
		scenarioStart: 0,
		hover: -1,
		focus: -1,
		pointer: new THREE.Vector2(0, 0),
		pointerInside: false,
		pointerSmooth: new THREE.Vector2(0, 0),
		yaw: 0,
		arcYaw: 0,
		width: 0,
		height: 0,
		dpr: 0,
		portrait: false,
		safeArea: null,
		disposed: false,
		motion: reducedMotion ? 0 : 1,
	};
	const shared = createSharedUniforms(reducedMotion);
	const world = new THREE.Group();
	const under = new THREE.Group();
	scene.add(world, under);
	const ctx: SceneContext = {
		container,
		canvas,
		renderer,
		scene,
		camera,
		world,
		under,
		data,
		options,
		stage,
		shared,
		maxAniso: Math.min(8, renderer.capabilities.getMaxAnisotropy()),
	};

	const windows = createWindows(ctx);
	const surface = createSurface(ctx);
	let beneath: Beneath | null = null;
	const composer = createComposer(stage, windows, () => {
		if (!beneath) throw new Error("Descent: composed before the board exists");
		return beneath.frame();
	});
	beneath = createBeneath(ctx, windows, composer);
	const layer = beneath;
	const rig = createCameraRig(ctx, windows, composer);
	const post = createPost(renderer, scene, camera);
	layer.onArrive((app) => windows.setWindowState(app, "arrived"));

	const zero2 = new THREE.Vector2();
	const reveal = createReveal();
	const resting: WindowCues = { apps: [], arrivals: [], flash: new Map() };
	// One easing function reads the current frame's dt, so nothing is allocated per frame.
	let frameDt = 0;
	const k: Rate = (rate) => (frameDt <= 0 ? 1 : 1 - Math.exp(-frameDt * rate));
	const frame: Mutable<Frame> = {
		dt: 0,
		k,
		t: 0,
		motion: stage.motion,
		reveal,
		alive: false,
	};
	const clock: Mutable<ScenarioClock> = { index: 0, elapsed: 0 };
	let wasAlive = false;

	function applyLayout(portrait: boolean) {
		stage.portrait = portrait;
		windows.layout(portrait);
		stage.arcYaw = rig.arcYawTarget();
		windows.turn(stage.arcYaw);
		windows.place();
		layer.layout(portrait);
	}

	function update(dt: number) {
		frameDt = dt;
		const motion = stage.motion;
		stage.progress = lerp(stage.progress, stage.progressTarget, k(7));
		stage.intro = lerp(stage.intro, stage.introTarget, k(8));
		stage.pointerSmooth.lerp(stage.pointerInside ? stage.pointer : zero2, k(3));
		const p = stage.progress;
		// The scenario starts over each time the layer comes alive, so its first
		// run plays as the threads light and the page hears a fresh "start".
		const alive = p >= TIMELINE.alive;
		if (alive && !wasAlive && !stage.frozen) restartScenario();
		wasAlive = alive;
		shared.uTime.value = stage.t;
		shared.uMotion.value = motion;
		revealInto(p, reveal);
		rig.steer(p, k);
		windows.turn(stage.arcYaw);
		layer.followWindows();
		rig.aim(p, reveal);
		rig.publishTheme(surface.update(reveal));
		frame.dt = dt;
		frame.t = stage.t;
		frame.motion = motion;
		frame.alive = alive;
		clock.index = stage.scenario;
		clock.elapsed = stage.t - stage.scenarioStart;
		windows.update(frame, alive ? layer.cues(clock) : resting);
		layer.update(frame, clock);
		post.update(frame);
	}

	let lost = false;
	// Nothing is drawn until every program has compiled (warmUp), so no frame
	// waits on a shader link; until then the canvas stays hidden, since an
	// undrawn opaque canvas shows black.
	let warmed = false;
	canvas.style.visibility = "hidden";
	function render(dt = 0) {
		if (stage.disposed || lost || !post.active) return;
		renderer.info.reset();
		update(dt);
		if (!warmed) return;
		post.render();
		if (canvas.style.visibility) canvas.style.visibility = "";
		if (reducedMotion) post.release();
	}

	const budget = createFrameBudget();
	function measureHost() {
		const rect = container.getBoundingClientRect();
		const cap = rect.width < rect.height * 0.8 ? MAX_DPR_PORTRAIT : MAX_DPR;
		const device = Math.min(window.devicePixelRatio || 1, cap);
		return {
			w: Math.max(1, Math.round(rect.width)),
			h: Math.max(1, Math.round(rect.height)),
			device,
			dpr: Math.max(Math.min(device, MIN_DPR), device * budget.scale),
		};
	}

	/** Applies the host's size and pixel ratio; returns whether anything changed. */
	function resize() {
		if (stage.disposed) return false;
		const { w, h, device, dpr } = measureHost();
		const same = w === stage.width && h === stage.height && dpr === stage.dpr;
		if (post.active && same) return false;
		const portrait = w / h < 0.8;
		const layoutChanged = !post.active || stage.portrait !== portrait;
		stage.width = w;
		stage.height = h;
		stage.dpr = dpr;
		renderer.setPixelRatio(dpr);
		renderer.setSize(w, h, false);
		if (layoutChanged) applyLayout(portrait);
		camera.aspect = w / h;
		camera.updateProjectionMatrix();
		post.resize(w, h, dpr, device);
		budget.reset();
		return true;
	}

	let raf = 0;
	let last = 0;
	let running = 0;
	const loop = (now: number) => {
		raf = requestAnimationFrame(loop);
		const ms = last ? now - last : 0;
		const dt = last ? Math.min(0.05, ms / 1000) : 0.016;
		last = now;
		if (!stage.frozen && !reducedMotion) stage.t += dt;
		running += dt;
		if (!stage.frozen && running > BUDGET_DELAY && budget.sample(ms) !== null) {
			resize();
		}
		render(dt);
	};
	function resume() {
		if (reducedMotion || stage.disposed || raf) return;
		last = 0;
		budget.reset();
		raf = requestAnimationFrame(loop);
	}
	// A paused scene keeps its last frame on screen and gives its render
	// targets back; the next frame allocates them again.
	function pause() {
		cancelAnimationFrame(raf);
		raf = 0;
		post.release();
	}
	// A size change while nothing animates (paused, reduced motion) still repaints.
	function refit() {
		if (resize() && !raf) render(0);
	}

	const resizeObserver = new ResizeObserver(refit);
	resizeObserver.observe(container);
	// Moving to a display with another pixel ratio fires no resize event.
	let dprQuery: MediaQueryList | null = null;
	function watchDpr() {
		dprQuery?.removeEventListener("change", onDprChange);
		dprQuery = window.matchMedia(
			`(resolution: ${window.devicePixelRatio || 1}dppx)`,
		);
		dprQuery.addEventListener("change", onDprChange);
	}
	function onDprChange() {
		watchDpr();
		refit();
	}
	watchDpr();

	// three keeps the default-prevented loss so the context can come back; the
	// restored context starts empty, so every face is drawn and uploaded again.
	const onLost = () => {
		lost = true;
	};
	const onRestored = () => {
		lost = false;
		warmed = true;
		windows.redraw();
		layer.redraw();
		render(0);
	};
	canvas.addEventListener("webglcontextlost", onLost);
	canvas.addEventListener("webglcontextrestored", onRestored);

	resize();
	update(0);

	// Programs compile in parallel with the page fonts loading, without
	// blocking where the browser can (KHR_parallel_shader_compile): the scene,
	// including the board that stays hidden until the floor turns to glass, and
	// the bloom chain that first runs on the dark stage. Then,
	// in a later task than the scene's creation, the textures upload and one
	// frame with every object shown uploads the buffers and allocates the
	// targets; it is replaced before it is presented. A context lost meanwhile
	// never reports its programs ready, so the wait is bounded.
	async function warmUp() {
		await Promise.race([
			Promise.all([renderer.compileAsync(scene, camera), post.compile()]),
			new Promise((resolve) => window.setTimeout(resolve, WARM_LIMIT_MS)),
		]);
		if (stage.disposed || lost) return;
		for (const texture of sceneTextures(scene)) renderer.initTexture(texture);
		const hidden: THREE.Object3D[] = [];
		scene.traverse((o) => {
			if (!o.visible) {
				hidden.push(o);
				o.visible = true;
			}
		});
		try {
			post.warm();
		} finally {
			for (const o of hidden) o.visible = false;
		}
	}
	const warming = warmUp()
		.catch((error: unknown) => {
			console.warn("Descent: shader warm-up failed", error);
		})
		.then(() => {
			warmed = true;
		});

	const fontsLoaded =
		"fonts" in document
			? Promise.all(
					fontSpecs().map((f) => document.fonts.load(f).catch(() => null)),
				).then(() => document.fonts.ready)
			: Promise.resolve();
	const ready = Promise.all([
		fontsLoaded.then(() => {
			if (stage.disposed) return;
			windows.redraw();
			layer.redraw();
		}),
		warming,
	]).then(() => {
		if (!stage.disposed) render(0);
	});

	// The director resets the windows the new run touches on its "start".
	function restartScenario() {
		stage.scenarioStart = stage.t;
	}

	return {
		ready,
		setScenario(i) {
			stage.scenario = i;
			restartScenario();
		},
		selectApp(i) {
			stage.app = i;
		},
		focusApp(i) {
			stage.focus = i;
		},
		setProgress(p, immediate = false) {
			stage.progressTarget = clamp(p);
			if (immediate) stage.progress = stage.progressTarget;
		},
		setIntro(v, immediate = false) {
			stage.introTarget = clamp(v);
			if (immediate) stage.intro = stage.introTarget;
		},
		setTime(t) {
			stage.t = t;
			stage.frozen = true;
		},
		setScenarioClock(t) {
			stage.scenarioStart = t;
		},
		snap() {
			stage.progress = stage.progressTarget;
			stage.intro = stage.introTarget;
			stage.pointerSmooth.set(0, 0);
			stage.pointerInside = false;
			stage.hover = -1;
			stage.yaw = rig.yawTarget(stage.progress);
			stage.arcYaw = rig.arcYawTarget();
			update(0);
		},
		render: () => render(0),
		setSafeArea(area) {
			stage.safeArea = area;
		},
		replayScenario: restartScenario,
		resize() {
			refit();
		},
		pause,
		resume,
		info() {
			const r = renderer.info.render;
			return {
				frame: r.frame,
				calls: r.calls,
				triangles: r.triangles,
				points: r.points,
				lines: r.lines,
				geometries: renderer.info.memory.geometries,
				textures: renderer.info.memory.textures,
				programs: renderer.info.programs?.length ?? 0,
				dpr: stage.dpr,
				dprScale: budget.scale,
				size: [stage.width, stage.height],
				layout: stage.portrait ? "portrait" : "landscape",
				sky: reveal.sky,
			};
		},
		// The GPU resources go once a running warm-up has finished: three keeps
		// polling the programs it compiles and fails on disposed materials.
		dispose() {
			if (stage.disposed) return;
			stage.disposed = true;
			cancelAnimationFrame(raf);
			raf = 0;
			resizeObserver.disconnect();
			dprQuery?.removeEventListener("change", onDprChange);
			canvas.removeEventListener("webglcontextlost", onLost);
			canvas.removeEventListener("webglcontextrestored", onRestored);
			canvas.remove();
			void warming.finally(() => {
				windows.dispose();
				surface.dispose();
				layer.dispose();
				post.dispose();
				releaseRenderer(renderer);
			});
		},
	};
}
