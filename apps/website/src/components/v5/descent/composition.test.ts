import { describe, expect, test } from "bun:test";
import * as THREE from "three";
import { layoutBoard } from "../board/board-layout";
import {
	BOARD_STRING_KEYS,
	type BoardStrings,
	createBoard,
} from "../board/board-spec";
import { cameraKeys, keyFov, sampleKeys, zoomFov } from "./camera-keys";
import {
	MARGIN_Y,
	PROPORTIONS,
	SETTLE_MAX,
	compose,
	fitArc,
	marginX,
} from "./composition";
import { bandAir } from "./controller";
import type {
	BoardFrame,
	Composition,
	Framing,
	LayoutKind,
	WindowsView,
} from "./internal";
import type { DescentSafeArea } from "./types";
import { arcWindows, windowCorner, windowDock } from "./windows";

const strings = Object.fromEntries(
	BOARD_STRING_KEYS.map((key) => [key, key.split(".").pop() ?? key]),
) as BoardStrings;
const spec = createBoard(strings);
const measure = (text: string, font: string) =>
	Array.from(text).length * 0.55 * Number(/([\d.]+)px/.exec(font)?.[1] ?? 10);
const layout = layoutBoard(spec, { measure, fontFamily: "Inter" });

/** The arc as the windows lay it out, unturned (phones turn the selected app overhead). */
function arcView(portrait: boolean): WindowsView {
	const arc = arcWindows(portrait);
	const pose = (app: number) => ({
		x: arc[app].x,
		z: arc[app].z,
		rotY: arc[app].rotY,
	});
	return {
		count: arc.length,
		width: (app) => arc[app].w,
		theta: (app) => arc[app].theta,
		pose,
		dock: (app, offset = 0, out = new THREE.Vector3()) =>
			windowDock(pose(app), offset, out),
		corner: (app, i, out) => windowCorner(app, arc[app].w, pose(app), i, out),
	};
}

const wide: BoardFrame = {
	content: layout.bounds,
	centre: layout.bounds.x + layout.bounds.width / 2,
	span: layout.bounds.width,
};
const nodeWidth = layout.nodes[0].width;
const phone: BoardFrame = {
	content: layout.bounds,
	centre: layout.bounds.x + layout.bounds.width / 2,
	span: 2 * nodeWidth + 60 + 24,
};

/**
 * A safe area as the page measures it (window.__proto.info().safeArea): under
 * the sub-head and above the dock at rest, above the band's foot before the
 * band shows, and above the band with air once it shows.
 */
const areaOf = (
	sub: number,
	chrome: number,
	foot: number,
	beneath: number,
): DescentSafeArea => ({
	surface: { top: sub, bottom: chrome },
	open: { top: 72, bottom: foot },
	beneath: { top: 72, bottom: beneath },
});

/** CSS px from the stage's top to the copy band: the beneath inset less the air the page adds. */
const bandTop = (width: number, height: number, area: DescentSafeArea) =>
	height - area.beneath.bottom + Math.round(bandAir(width, height));

/** A camera as the rig places it at the end of the scroll with `framing`. */
function cameraAt(
	kind: LayoutKind,
	width: number,
	height: number,
	framing: Framing,
) {
	const keys = cameraKeys[kind];
	const pos = sampleKeys(keys, 1, "pos");
	const at = sampleKeys(keys, 1, "at");
	const fov = zoomFov(keyFov(kind, 1, width / height), framing.zoom);
	const cam = new THREE.PerspectiveCamera(fov, width / height, 0.05, 600);
	cam.position.set(pos[0] + framing.offset[0], pos[1], pos[2]);
	cam.lookAt(at[0] + framing.offset[0], at[1], at[2]);
	cam.setViewOffset(width, height, 0, framing.shiftPx, width, height);
	cam.updateProjectionMatrix();
	cam.updateMatrixWorld();
	return (p: THREE.Vector3) => {
		const v = p.clone().project(cam);
		return { x: ((v.x + 1) / 2) * width, y: ((1 - v.y) / 2) * height };
	};
}

/** The board's content corners and thread anchors in world space for a placement. */
function boardPoints(comp: Composition, frame: BoardFrame) {
	const { unit, x, y, z, tilt } = comp.board;
	const group = new THREE.Object3D();
	group.position.set(x, y, z);
	group.rotation.set(-tilt, 0, 0);
	group.updateMatrixWorld(true);
	const world = (bx: number, by: number) =>
		new THREE.Vector3(bx * unit, -by * unit, 0).applyMatrix4(group.matrixWorld);
	const { content } = frame;
	const left = frame.centre - frame.span / 2;
	const right = frame.centre + frame.span / 2;
	const top = content.y;
	const bottom = content.y + content.height;
	return {
		corners: [
			world(left, top),
			world(right, top),
			world(left, bottom),
			world(right, bottom),
		],
		world,
	};
}

/** The lowest screen y of the board under a framing. */
const boardBottom = (
	kind: LayoutKind,
	width: number,
	height: number,
	comp: Composition,
	frame: BoardFrame,
	framing: Framing,
) => {
	const px = cameraAt(kind, width, height, framing);
	return Math.max(...boardPoints(comp, frame).corners.map((c) => px(c).y));
};

const LANDSCAPE = [
	{ size: [1454, 898], area: areaOf(259, 146, 73, 274) },
	{ size: [1440, 790], area: areaOf(259, 146, 67, 244) },
	{ size: [1280, 720], area: areaOf(255, 146, 62, 232) },
	{ size: [1180, 800], area: areaOf(252, 146, 67, 261) },
	{ size: [1024, 768], area: areaOf(247, 146, 65, 254) },
] as const;

describe("landscape composition", () => {
	const windows = arcView(false);
	for (const { size, area } of LANDSCAPE) {
		const [width, height] = size;
		const comp = compose({
			width,
			height,
			area,
			kind: "landscape",
			windows,
			board: wide,
		});
		const label = `${width}x${height}`;
		const band = bandTop(width, height, area);
		const air = bandAir(width, height);

		test(`${label}: every window is whole, under the header and inside the side margins, before and with the band`, () => {
			for (const framing of [comp.open, comp.band]) {
				const px = cameraAt("landscape", width, height, framing);
				for (let app = 0; app < windows.count; app++) {
					for (let i = 0; i < 4; i++) {
						const c = px(windows.corner(app, i, new THREE.Vector3()));
						expect(c.x).toBeGreaterThanOrEqual(marginX(width) - 1);
						expect(c.x).toBeLessThanOrEqual(width - marginX(width) + 1);
						expect(c.y).toBeGreaterThanOrEqual(area.beneath.top + MARGIN_Y - 1);
					}
				}
			}
		});

		test(`${label}: the board takes its share of the width and keeps its air above the band, without a dead band`, () => {
			const share = comp.boardWidthPx / width;
			expect(share).toBeGreaterThanOrEqual(
				PROPORTIONS.landscape.board.min - 0.01,
			);
			expect(share).toBeLessThanOrEqual(PROPORTIONS.landscape.board.max + 0.01);
			const clear =
				band - boardBottom("landscape", width, height, comp, wide, comp.band);
			expect(clear).toBeGreaterThanOrEqual(air + MARGIN_Y - 1);
			expect(clear).toBeLessThan(air + MARGIN_Y + 80);
		});

		test(`${label}: before the band the picture is the final one, lowered into the band's room, and the band never meets it`, () => {
			expect(comp.open.zoom).toBeCloseTo(comp.band.zoom, 9);
			const drop = comp.band.shiftPx - comp.open.shiftPx;
			expect(drop).toBeGreaterThanOrEqual(0);
			expect(drop).toBeLessThanOrEqual(SETTLE_MAX + 1e-6);
			const open = boardBottom(
				"landscape",
				width,
				height,
				comp,
				wide,
				comp.open,
			);
			const settled = boardBottom(
				"landscape",
				width,
				height,
				comp,
				wide,
				comp.band,
			);
			expect(open).toBeLessThanOrEqual(
				height - area.open.bottom - MARGIN_Y + 1,
			);
			// The band is still faint for the first third of its fade.
			for (let b = 0.3; b <= 1; b += 0.05) {
				expect(open + (settled - open) * b).toBeLessThanOrEqual(band);
			}
		});

		test(`${label}: the board lies beneath the floor and every thread runs down to it`, () => {
			const { corners, world } = boardPoints(comp, wide);
			for (const c of corners) expect(c.y).toBeLessThan(-0.3);
			const px = cameraAt("landscape", width, height, comp.band);
			for (const thread of spec.threads) {
				const [ax, ay] = layout.threadAnchor(thread.id);
				const anchor = px(world(ax, ay - 6));
				const from = px(windows.dock(thread.app, 0, new THREE.Vector3()));
				expect(anchor.y).toBeGreaterThan(from.y + 20);
				expect(anchor.x).toBeGreaterThan(0);
				expect(anchor.x).toBeLessThan(width);
			}
		});
	}

	test("Felix's viewport keeps at least 48 px of clear air above and below the board", () => {
		const { size, area } = LANDSCAPE[0];
		const [width, height] = size;
		const comp = compose({
			width,
			height,
			area,
			kind: "landscape",
			windows,
			board: wide,
		});
		const px = cameraAt("landscape", width, height, comp.band);
		const docks = Array.from(
			{ length: windows.count },
			(_, app) => px(windows.dock(app, 0, new THREE.Vector3())).y,
		);
		const corners = boardPoints(comp, wide).corners.map((c) => px(c).y);
		expect(Math.min(...corners) - Math.max(...docks)).toBeGreaterThanOrEqual(
			48,
		);
		expect(
			bandTop(width, height, area) - Math.max(...corners),
		).toBeGreaterThanOrEqual(48);
	});
});

describe("phone composition", () => {
	const windows = arcView(true);
	for (const [width, height, area] of [
		[390, 844, areaOf(219, 138, 38, 182)],
		[375, 667, areaOf(219, 138, 38, 170)],
	] as const) {
		const comp = compose({
			width,
			height,
			area,
			kind: "portrait",
			windows,
			board: phone,
		});
		test(`${width}x${height}: the columns in view take their share and end above the band, with air`, () => {
			// A short phone has no room for every value at its best: the picture shrinks a little.
			const least = height < 700 ? 0.7 : PROPORTIONS.portrait.board.min - 0.02;
			const share = comp.boardWidthPx / width;
			expect(share).toBeGreaterThanOrEqual(least);
			expect(share).toBeLessThanOrEqual(PROPORTIONS.portrait.board.max + 0.01);
			const bottom = boardBottom(
				"portrait",
				width,
				height,
				comp,
				phone,
				comp.band,
			);
			expect(bandTop(width, height, area) - bottom).toBeGreaterThanOrEqual(
				bandAir(width, height) + MARGIN_Y - 1,
			);
		});
		test(`${width}x${height}: the selected window overhead sits whole under the header`, () => {
			for (const framing of [comp.open, comp.band]) {
				const px = cameraAt("portrait", width, height, framing);
				const hero = arcWindows(true)[0];
				const overhead = { x: 0, z: 0, rotY: 0 };
				for (let i = 0; i < 4; i++) {
					const c = px(
						windowCorner(0, hero.w, overhead, i, new THREE.Vector3()),
					);
					expect(c.x).toBeGreaterThan(0);
					expect(c.x).toBeLessThan(width);
					expect(c.y).toBeGreaterThanOrEqual(area.beneath.top + MARGIN_Y - 1);
				}
			}
		});
	}
});

describe("reference point", () => {
	test("the rig's pin lands every framing where the composition put it", () => {
		const windows = arcView(false);
		const comp = compose({
			width: 1454,
			height: 898,
			area: LANDSCAPE[0].area,
			kind: "landscape",
			windows,
			board: wide,
		});
		expect(comp.open.ref).toBeGreaterThan(comp.band.ref);
		expect(comp.band.ref).toBeGreaterThanOrEqual(72);
		expect(comp.rest.ref).toBeGreaterThan(comp.open.ref);
	});
});

describe("fitArc", () => {
	const H = 900;
	const ndc = (px: number) => 1 - (2 * px) / H;
	test("an arc inside the band on a wide screen is left alone", () => {
		const r = fitArc(
			{ top: ndc(320), base: ndc(580) },
			{ top: 260, bottom: 146 },
			H,
			false,
		);
		expect(r).toEqual({ zoom: 1, shiftPx: 0 });
	});
	test("an arc under the sub-head moves down to the band", () => {
		const r = fitArc(
			{ top: ndc(230), base: ndc(500) },
			{ top: 260, bottom: 146 },
			H,
			false,
		);
		expect(r.zoom).toBe(1);
		expect(r.shiftPx).toBeCloseTo(-30, 6);
	});
	test("phones pull the arc up under the sub-head", () => {
		const r = fitArc(
			{ top: ndc(370), base: ndc(560) },
			{ top: 234, bottom: 138 },
			H,
			true,
		);
		expect(r.zoom).toBe(1);
		expect(r.shiftPx).toBeCloseTo(136, 6);
	});
	test("a band too short for the arc zooms out and aligns the top", () => {
		const r = fitArc(
			{ top: ndc(200), base: ndc(800) },
			{ top: 260, bottom: 146 },
			H,
			false,
		);
		expect(r.zoom).toBeLessThan(1);
		const top = ((1 - ndc(200) * r.zoom - (2 * r.shiftPx) / H) / 2) * H;
		expect(top).toBeCloseTo(260, 6);
	});
});
