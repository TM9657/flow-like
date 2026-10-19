import * as THREE from "three";
import {
	type BoardLayout,
	type MeasureText,
	type Point,
	type Rect,
	layoutBoard,
} from "../board/board-layout";
import {
	BOARD_PALETTE_DARK,
	type BoardEmphasis,
	css,
	paintBoard,
} from "../board/board-paint";
import type {
	BoardGrid,
	BoardScenarioId,
	BoardSpec,
} from "../board/board-spec";
import type { BoardPlacement } from "./internal";

/** Board units of dark ground painted around everything. */
const PAD = 28;
/**
 * The board tells one story at a time: off the active route every node is a
 * whisper of the product's board, or nothing on a phone, which frames the story
 * alone, and no wire is drawn. The base painting is that whisper alone; the lit
 * painting adds the route at full strength.
 */
const STORY: Record<BoardGrid, BoardEmphasis> = {
	landscape: { inactive: 0.12, text: 0.9, wires: 0 },
	portrait: { inactive: 0, text: 0.9, wires: 0 },
};
const NO_ROUTE = new Set<never>();
/** Uniform values that leave the whole board shown and the whole route lit. */
const ALL = { story: [-0.1, -0.2], window: [-0.5, 1.5, 0.1] } as const;

const BOARD_VERT = /* glsl */ `
varying vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

// Two paintings of the board on its own dark ground, which is also the colour
// around it beneath: the whisper board and the same board with its route, mixed
// by uLit. uFade brings it up from that ground with depth: the top row, nearest
// the glass, first, the deepest row last. uStory hides the board below the
// story (v: shown above x, gone below y). uWindow lights the route only inside
// a phone's frame (u: lit between x and y, out by z beyond). uEdge fades it out
// toward the frame's sides (x: the fade's width in drawing pixels, 0 for none;
// y: the drawing's width), so a node a phone's frame cuts dissolves.
const BOARD_FRAG = /* glsl */ `
const float DEPTH_FADE = 1.2;
uniform sampler2D uBase;
uniform sampler2D uLitMap;
uniform float uLit;
uniform float uFade;
uniform vec3 uGround;
uniform vec2 uEdge;
uniform vec2 uStory;
uniform vec3 uWindow;
varying vec2 vUv;
void main() {
  float inside = smoothstep(uWindow.x - uWindow.z, uWindow.x, vUv.x) * (1.0 - smoothstep(uWindow.y, uWindow.y + uWindow.z, vUv.x));
  vec3 col = mix(texture2D(uBase, vUv).rgb, texture2D(uLitMap, vUv).rgb, uLit * inside);
  float keep = smoothstep(0.0, 1.0, uFade * (1.0 + DEPTH_FADE) - (1.0 - vUv.y) * DEPTH_FADE);
  keep *= smoothstep(uStory.y, uStory.x, vUv.y);
  if (uEdge.x > 0.0) keep *= smoothstep(0.0, uEdge.x, min(gl_FragCoord.x, uEdge.y - gl_FragCoord.x));
  gl_FragColor = vec4(mix(uGround, col, keep), 1.0);
}
`;

export interface BoardFonts {
	/** CSS font-family list of the page's Inter. */
	readonly sans: string;
}

export interface BoardSurface {
	readonly group: THREE.Group;
	layout(): BoardLayout;
	/** What is drawn on the board, in board units: the layout's bounds. */
	content(): Rect;
	/** Painted area in board units: the content and the dark ground around it. */
	area(): Rect;
	placement(): BoardPlacement;
	place(next: BoardPlacement): void;
	/** World position of a board point. */
	world(point: Point, out: THREE.Vector3): THREE.Vector3;
	/** Paints `route` (null for none) into the route painting; a no-op when it shows already. */
	paint(route: BoardScenarioId | null): void;
	/** The route the route painting shows. */
	painted(): BoardScenarioId | null;
	/** How far the board has faded up (0–1) and how strongly its route shows over the whisper (0–1). */
	show(fade: number, route: number): void;
	/** Texture pixels per board unit; repaints when it changes. */
	density(scale: number): void;
	/** Lays the board out again with the fonts the page has now, then repaints. */
	relayout(): void;
	/** Lays out another grid of the board (a phone's or the wide one), with its emphasis, then repaints. */
	regrid(spec: BoardSpec, grid: BoardGrid): void;
	/** Hides the board below board y `bottom`, fading out over the `feather` board units under it. */
	clip(bottom: number, feather: number): void;
	/**
	 * Lights the route only between board x `left` and `right`, fading out over
	 * `feather` board units beyond them (a phone's frame); null lights it everywhere.
	 */
	spotlight(
		window: { left: number; right: number; feather: number } | null,
	): void;
	/** Fades the board out over `fade` drawing pixels at the sides of a drawing `width` wide (0: no fade). */
	edgeFade(fade: number, width: number): void;
	dispose(): void;
}

export function readBoardFonts(container: HTMLElement): BoardFonts {
	const style = getComputedStyle(container);
	return {
		sans: style.fontFamily || "Inter, ui-sans-serif, system-ui, sans-serif",
	};
}

function createMeasure(): MeasureText {
	const canvas = document.createElement("canvas");
	const ctx = canvas.getContext("2d");
	if (!ctx) throw new Error("Descent: no 2D context to measure board text");
	ctx.fontKerning = "normal";
	return (text, font) => {
		ctx.font = font;
		return ctx.measureText(text).width;
	};
}

function createBoardMaterial() {
	const [r, g, b] = BOARD_PALETTE_DARK.background;
	return new THREE.ShaderMaterial({
		uniforms: {
			uBase: { value: null },
			uLitMap: { value: null },
			uLit: { value: 0 },
			uFade: { value: 0 },
			uGround: {
				value: new THREE.Color().setRGB(r / 255, g / 255, b / 255, "srgb"),
			},
			uEdge: { value: new THREE.Vector2() },
			uStory: { value: new THREE.Vector2(...ALL.story) },
			uWindow: { value: new THREE.Vector3(...ALL.window) },
		},
		vertexShader: BOARD_VERT,
		fragmentShader: BOARD_FRAG,
	});
}

/** One painting of the board: its canvas and the texture it uploads to. */
interface Painting {
	readonly canvas: HTMLCanvasElement;
	texture: THREE.CanvasTexture | null;
}

export function createBoardSurface(
	renderer: THREE.WebGLRenderer,
	initial: BoardSpec,
	fonts: BoardFonts,
): BoardSurface {
	let spec = initial;
	let emphasis = STORY.landscape;
	const measure = createMeasure();
	const group = new THREE.Group();
	group.name = "board";
	const material = createBoardMaterial();
	const mesh = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), material);
	mesh.name = "board-plane";
	group.add(mesh);
	const maxSize = renderer.capabilities.maxTextureSize;
	const anisotropy = renderer.capabilities.getMaxAnisotropy();
	const base: Painting = {
		canvas: document.createElement("canvas"),
		texture: null,
	};
	const lit: Painting = {
		canvas: document.createElement("canvas"),
		texture: null,
	};
	let layout: BoardLayout;
	let area: Rect;
	let placed: BoardPlacement = { unit: 0.015, x: 0, y: 0, z: 0, tilt: 0 };
	let litRoute: BoardScenarioId | null = null;
	let scale = 2;

	function build() {
		layout = layoutBoard(spec, { measure, fontFamily: fonts.sans });
		const { bounds } = layout;
		area = {
			x: bounds.x - PAD,
			y: bounds.y - PAD,
			width: bounds.width + 2 * PAD,
			height: bounds.height + 2 * PAD,
		};
		reshape();
	}

	// One unit plane, scaled to the area: placing allocates nothing.
	function reshape() {
		const { unit } = placed;
		mesh.scale.set(area.width * unit, area.height * unit, 1);
		mesh.position.set(
			(area.x + area.width / 2) * unit,
			-(area.y + area.height / 2) * unit,
			0,
		);
	}

	function draw(canvas: HTMLCanvasElement, route: BoardScenarioId | null) {
		const ctx = canvas.getContext("2d");
		if (!ctx) throw new Error("Descent: no 2D context for the board texture");
		const zoom = canvas.width / area.width;
		ctx.setTransform(1, 0, 0, 1, 0, 0);
		ctx.fillStyle = css(BOARD_PALETTE_DARK.background);
		ctx.fillRect(0, 0, canvas.width, canvas.height);
		const on = route ? layout.routes[route] : null;
		paintBoard(ctx, layout, BOARD_PALETTE_DARK, {
			view: { x: -area.x * zoom, y: -area.y * zoom, zoom },
			activeNodes: on ? new Set(on.nodes) : NO_ROUTE,
			activeEdges: on ? new Set(on.edges) : NO_ROUTE,
			emphasis,
		});
	}

	function upload(
		painting: Painting,
		uniform: "uBase" | "uLitMap",
		route: BoardScenarioId | null,
	) {
		const fit = Math.min(scale, maxSize / area.width, maxSize / area.height);
		const width = Math.ceil(area.width * fit);
		const height = Math.ceil(area.height * fit);
		const { canvas } = painting;
		const resized = canvas.width !== width || canvas.height !== height;
		if (resized) {
			canvas.width = width;
			canvas.height = height;
		}
		draw(canvas, route);
		if (resized && painting.texture) {
			painting.texture.dispose();
			painting.texture = null;
		}
		if (!painting.texture) {
			const texture = new THREE.CanvasTexture(canvas);
			texture.colorSpace = THREE.SRGBColorSpace;
			texture.generateMipmaps = true;
			texture.minFilter = THREE.LinearMipmapLinearFilter;
			texture.magFilter = THREE.LinearFilter;
			texture.anisotropy = anisotropy;
			painting.texture = texture;
			material.uniforms[uniform].value = texture;
		}
		painting.texture.needsUpdate = true;
		renderer.initTexture(painting.texture);
	}

	function repaint() {
		upload(base, "uBase", null);
		upload(lit, "uLitMap", litRoute);
	}

	build();
	repaint();

	return {
		group,
		layout: () => layout,
		content: () => layout.bounds,
		area: () => area,
		placement: () => placed,
		place(next) {
			const rescaled = next.unit !== placed.unit;
			placed = next;
			group.position.set(next.x, next.y, next.z);
			group.rotation.set(-next.tilt, 0, 0);
			if (rescaled) reshape();
			group.updateMatrixWorld(true);
		},
		world(point, out) {
			const { unit } = placed;
			return out
				.set(point[0] * unit, -point[1] * unit, 0)
				.applyMatrix4(group.matrixWorld);
		},
		paint(route) {
			if (route === litRoute) return;
			litRoute = route;
			upload(lit, "uLitMap", route);
		},
		painted: () => litRoute,
		show(fade, route) {
			material.uniforms.uFade.value = fade;
			material.uniforms.uLit.value = route;
			mesh.visible = fade > 0.002;
		},
		density(next) {
			if (next === scale) return;
			scale = next;
			repaint();
		},
		relayout() {
			build();
			repaint();
		},
		regrid(next, grid) {
			spec = next;
			emphasis = STORY[grid];
			build();
			repaint();
		},
		clip(bottom, feather) {
			const v = (y: number) => 1 - (y - area.y) / area.height;
			material.uniforms.uStory.value.set(v(bottom), v(bottom + feather));
		},
		spotlight(window) {
			const u = (x: number) => (x - area.x) / area.width;
			const uniform = material.uniforms.uWindow.value;
			if (!window) uniform.set(...ALL.window);
			else {
				uniform.set(
					u(window.left),
					u(window.right),
					window.feather / area.width,
				);
			}
		},
		edgeFade(fade, width) {
			material.uniforms.uEdge.value.set(fade, width);
		},
		dispose() {
			mesh.geometry.dispose();
			material.dispose();
			for (const painting of [base, lit]) {
				painting.texture?.dispose();
				painting.canvas.width = 0;
				painting.canvas.height = 0;
			}
		},
	};
}
