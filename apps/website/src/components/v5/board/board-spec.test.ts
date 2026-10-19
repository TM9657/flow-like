import { describe, expect, test } from "bun:test";
import {
	type BoardRoute,
	type Point,
	type Rect,
	layoutBoard,
} from "./board-layout";
import { pulseAt } from "./board-pulse";
import {
	BOARD_STRING_KEYS,
	type BoardNodeId,
	type BoardStrings,
	REFERENCE_PIN,
	createBoard,
} from "./board-spec";

const strings: BoardStrings = {
	"v5.apps.connected.email.title": "Email",
	"v5.apps.node.extract": "Extract",
	"v5.apps.node.enrich": "Enrich",
	"v5.apps.node.approve": "Approve",
	"v5.apps.node.governance": "Governance",
	"v5.platform.f.data.title": "Data Studio",
	"v5.apps.node.ontology": "Ontology",
	"v5.apps.connected.notes.title": "Notes",
	"v5.apps.node.flowpilot": "FlowPilot",
	"v5.apps.connected.calendar.title": "Calendar",
	"v5.apps.mini.agent.mail": "Mail",
	"v5.apps.mini.agent.data": "Data",
	"v5.apps.board.pin.message": "Message",
	"v5.apps.board.pin.request": "Request",
	"v5.apps.board.pin.companyData": "Company Data",
	"v5.apps.board.pin.policy": "Policy",
	"v5.apps.board.pin.records": "Records",
	"v5.apps.board.pin.meetingNotes": "Meeting Notes",
	"v5.apps.board.pin.briefing": "Briefing",
	"v5.apps.board.pin.agenda": "Agenda",
};
const board = createBoard(strings);
const measure = (text: string, font: string) =>
	Array.from(text).length *
	0.55 *
	Number(/([\d.]+)px/.exec(font)?.[1] ?? Number.NaN);
const layout = layoutBoard(board, { measure, fontFamily: "Inter" });

const node = (id: BoardNodeId) => {
	const found = board.nodes.find((entry) => entry.id === id);
	if (!found) throw new Error(`missing node ${id}`);
	return found;
};
const pin = (id: BoardNodeId, pinId: string) => {
	const found = node(id).pins.find((entry) => entry.id === pinId);
	if (!found) throw new Error(`missing pin ${id}.${pinId}`);
	return found;
};

const inset = (r: Rect, by: number): Rect => ({
	x: r.x + by,
	y: r.y + by,
	width: r.width - 2 * by,
	height: r.height - 2 * by,
});

const overlaps = (a: Rect, b: Rect) =>
	a.x < b.x + b.width &&
	b.x < a.x + a.width &&
	a.y < b.y + b.height &&
	b.y < a.y + a.height;

/** Liang–Barsky: does the segment a→b enter the rectangle? */
function segmentHits(a: Point, b: Point, r: Rect): boolean {
	const dx = b[0] - a[0];
	const dy = b[1] - a[1];
	const sides: [number, number][] = [
		[-dx, a[0] - r.x],
		[dx, r.x + r.width - a[0]],
		[-dy, a[1] - r.y],
		[dy, r.y + r.height - a[1]],
	];
	let enter = 0;
	let leave = 1;
	for (const [p, q] of sides) {
		if (p === 0 && q < 0) return false;
		if (p < 0) enter = Math.max(enter, q / p);
		if (p > 0) leave = Math.min(leave, q / p);
	}
	return enter <= leave;
}

/** Nodes the route's wires connect to its first node. */
function reachable(route: BoardRoute): BoardNodeId[] {
	const links = route.edges.map((id) => {
		const edge = board.edges.find((entry) => entry.id === id);
		if (!edge) throw new Error(`route ${route.id} names missing edge ${id}`);
		return [edge.source.node, edge.target.node];
	});
	const seen = new Set<BoardNodeId>([route.nodes[0]]);
	let grown = true;
	while (grown) {
		const before = seen.size;
		for (const [a, b] of links)
			if (seen.has(a) || seen.has(b)) seen.add(a).add(b);
		grown = seen.size > before;
	}
	return [...seen].sort();
}

const bodies = layout.nodes.map((entry) => ({
	id: entry.id,
	rect: inset(entry, 1),
}));

describe("board graph", () => {
	test("every edge joins an output to an input of the same kind and type", () => {
		const mismatches = board.edges.filter((edge) => {
			const source = pin(edge.source.node, edge.source.pin);
			const target = pin(edge.target.node, edge.target.pin);
			return (
				source.direction !== "output" ||
				target.direction !== "input" ||
				source.kind !== target.kind ||
				source.type !== target.type ||
				edge.kind !== source.kind
			);
		});
		expect(mismatches.map((edge) => edge.id)).toEqual([]);
	});

	test("every wire runs left to right", () => {
		const backwards = board.edges.filter(
			(edge) => node(edge.source.node).column >= node(edge.target.node).column,
		);
		expect(backwards.map((edge) => edge.id)).toEqual([]);
	});

	test("every input takes one wire and every exec output drives one input", () => {
		const inputs = board.edges.map(
			(edge) => `${edge.target.node}.${edge.target.pin}`,
		);
		expect(new Set(inputs).size).toBe(inputs.length);
		const execOutputs = board.edges
			.filter((edge) => edge.kind === "execution")
			.map((edge) => `${edge.source.node}.${edge.source.pin}`);
		expect(new Set(execOutputs).size).toBe(execOutputs.length);
	});

	test("every impure node runs: exec inputs are driven, entries are events or app-invoked", () => {
		const driven = new Set(
			board.edges
				.filter((edge) => edge.kind === "execution")
				.map((edge) => `${edge.target.node}.${edge.target.pin}`),
		);
		const execInputs = (id: BoardNodeId) =>
			node(id)
				.pins.filter((p) => p.kind === "execution" && p.direction === "input")
				.map((p) => `${id}.${p.id}`);
		const idle = board.nodes.flatMap((entry) =>
			execInputs(entry.id).filter((id) => !driven.has(id)),
		);
		expect(idle).toEqual([]);
		const orphans = board.nodes.filter(
			(entry) =>
				entry.kind !== "pure" &&
				entry.kind !== "event" &&
				execInputs(entry.id).length === 0 &&
				!board.threads.some((thread) => thread.node === entry.id),
		);
		expect(orphans.map((entry) => entry.id)).toEqual([]);
	});

	test("node kinds match their pins the way the product derives them", () => {
		for (const entry of board.nodes) {
			const exec = entry.pins.some((p) => p.kind === "execution");
			expect([entry.id, exec]).toEqual([entry.id, entry.kind !== "pure"]);
			const ids = entry.pins.map((p) => p.id);
			expect(new Set(ids).size).toBe(ids.length);
			const unlabeled = entry.pins.filter(
				(p) => (p.label === "") !== (p.kind !== "data"),
			);
			expect(unlabeled).toEqual([]);
		}
		const eventInputs = board.nodes
			.filter((entry) => entry.kind === "event")
			.flatMap((entry) =>
				entry.pins.filter(
					(p) => p.direction === "input" && p.kind !== "reference",
				),
			);
		expect(eventInputs).toEqual([]);
	});

	test("threads land on top-centre reference pins and every app reaches the board", () => {
		for (const thread of board.threads) {
			expect(thread.pin).toBe(REFERENCE_PIN);
			expect(pin(thread.node, thread.pin).kind).toBe("reference");
		}
		expect(new Set(board.threads.map((thread) => thread.app))).toEqual(
			new Set([0, 1, 2, 3, 4, 5, 6, 7]),
		);
		const references = layout.nodes.flatMap((entry) =>
			entry.pins
				.filter((p) => p.kind === "reference")
				.map((p) => [p.x - entry.width / 2, p.y]),
		);
		expect(references.every(([dx, y]) => dx === 0 && y === 0)).toBe(true);
	});

	test("each scenario route is one connected path ending in an app", () => {
		for (const route of Object.values(layout.routes)) {
			expect([route.id, reachable(route)]).toEqual([
				route.id,
				[...route.nodes].sort(),
			]);
			expect(route.beats.at(-1)?.threads.some((hop) => hop.toApp)).toBe(true);
		}
	});

	test("every board string is used", () => {
		const used = new Set(
			board.nodes.flatMap((entry) => [
				entry.title,
				...entry.pins.map((p) => p.label),
			]),
		);
		expect(BOARD_STRING_KEYS.filter((key) => !used.has(strings[key]))).toEqual(
			[],
		);
	});
});

describe("board layout", () => {
	test("wires are the 33-sample egui curve from pin centre to pin centre", () => {
		for (const edge of layout.edges) {
			expect(edge.points.length).toBe(33);
			expect(edge.points[0]).toEqual(
				layout.pinPosition(edge.source.node, edge.source.pin),
			);
			expect(edge.points[32]).toEqual(
				layout.pinPosition(edge.target.node, edge.target.pin),
			);
		}
	});

	test("no wire crosses a node and no nodes overlap", () => {
		const crossings = layout.edges.flatMap((edge) =>
			bodies
				.filter((body) =>
					edge.points
						.slice(1)
						.some((p, i) => segmentHits(edge.points[i], p, body.rect)),
				)
				.map((body) => `${edge.id} × ${body.id}`),
		);
		expect(crossings).toEqual([]);
		const stacked = layout.nodes.flatMap((a, i) =>
			layout.nodes
				.slice(i + 1)
				.filter((b) => overlaps(a, b))
				.map((b) => `${a.id} × ${b.id}`),
		);
		expect(stacked).toEqual([]);
	});

	test("threads drop onto their pins without crossing another node", () => {
		const blocked = layout.threads.flatMap((thread) => {
			const anchor = layout.threadAnchor(thread.id);
			return bodies
				.filter(
					(body) =>
						body.id !== thread.node &&
						segmentHits([anchor[0], layout.bounds.y], anchor, body.rect),
				)
				.map((body) => `${thread.id} × ${body.id}`);
		});
		expect(blocked).toEqual([]);
	});

	test("labels fit without truncation inside their clip", () => {
		const clipped = layout.nodes.flatMap((entry) => [
			...(entry.titleText === entry.title ? [] : [entry.id]),
			...entry.pins
				.filter(
					(p) =>
						p.text !== p.label ||
						measure(p.text, "400 9.6px Inter") > p.labelMaxWidth,
				)
				.map((p) => `${entry.id}.${p.id}`),
		]);
		expect(clipped).toEqual([]);
	});

	test("only text past the widest node ends in an ellipsis", () => {
		const long = layoutBoard(
			createBoard({
				...strings,
				"v5.apps.node.governance": "Unternehmensrichtlinienverwaltung",
				"v5.apps.board.pin.companyData":
					"Unternehmensstammdatenanreicherungsergebnis",
			}),
			{ measure, fontFamily: "Inter" },
		);
		const gov = long.node("gov");
		expect(gov.width).toBe(264);
		expect(gov.titleText.endsWith("…")).toBe(true);
		expect(
			18 + measure(gov.titleText, `500 ${gov.titleSize}px Inter`),
		).toBeLessThanOrEqual(gov.width / 2 - 10);
		const pinText = (id: BoardNodeId) =>
			long.node(id).pins.find((p) => p.id === "company_data")?.text;
		expect(pinText("ontology")?.endsWith("…")).toBe(true);
		expect(pinText("enrich")).toBe(
			"Unternehmensstammdatenanreicherungsergebnis",
		);
	});

	test("on the phone's grid every story is drawn whole and clean, the insight story in two columns", () => {
		const phone = createBoard(strings, "portrait");
		const grid = layoutBoard(phone, { measure, fontFamily: "Inter" });
		const column = (id: BoardNodeId) => grid.node(id).column;
		expect(
			phone.edges.filter(
				(edge) => column(edge.source.node) >= column(edge.target.node),
			),
		).toEqual([]);
		const stacked = grid.nodes.flatMap((a, i) =>
			grid.nodes
				.slice(i + 1)
				.filter((b) => overlaps(a, b))
				.map((b) => `${a.id} × ${b.id}`),
		);
		expect(stacked).toEqual([]);
		// A phone draws only the running story, so only its own nodes may block it.
		for (const route of Object.values(grid.routes)) {
			const own = grid.nodes
				.filter((n) => route.nodes.includes(n.id))
				.map((n) => ({ id: n.id, rect: inset(n, 1) }));
			const crossings = route.edges.flatMap((id) => {
				const points = grid.edgePath(id);
				return own
					.filter((body) =>
						points
							.slice(1)
							.some((p, i) => segmentHits(points[i], p, body.rect)),
					)
					.map((body) => `${route.id}: ${id} × ${body.id}`);
			});
			const blocked = route.threads.flatMap((id) => {
				const thread = grid.threads.find((entry) => entry.id === id);
				const anchor = grid.threadAnchor(id);
				return own
					.filter(
						(body) =>
							body.id !== thread?.node &&
							segmentHits([anchor[0], grid.bounds.y], anchor, body.rect),
					)
					.map((body) => `${route.id}: ${id} × ${body.id}`);
			});
			expect([...crossings, ...blocked]).toEqual([]);
		}
		const insight = grid.routes.insight.nodes.map(column);
		expect(Math.max(...insight) - Math.min(...insight)).toBe(1);
	});

	test("the pulse follows egui: sin fade, two samples of trail, gone at the end", () => {
		const path = layout.edgePath("email-extract-exec");
		const frame = pulseAt(path, 0.5);
		expect(frame?.opacity).toBeCloseTo(1);
		expect(frame?.head).toEqual([...path[16]]);
		expect(frame?.trail.length).toBe(4);
		expect(pulseAt(path, 1)).toBeNull();
	});
});
