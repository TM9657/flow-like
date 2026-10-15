import type { Fixture, FixtureNode, Viewport } from "./types";

/** One shared snapshot feeds both renderers, including every edge and pin anchor. */
export function createFixture(count = 10_000): Fixture {
	if (!Number.isSafeInteger(count) || count < 100 || count > 50_000)
		throw new Error("Node count must be an integer between 100 and 50,000");
	const columns = Math.ceil(Math.sqrt(count));
	const kinds: FixtureNode["kind"][] = [
		"event",
		"function",
		"pure",
		"standard",
	];
	const nodes = Array.from({ length: count }, (_, index) => ({
		id: `node-${index}`,
		x: (index % columns) * 230,
		y: Math.floor(index / columns) * 132,
		label: `${["Chat Event", "Process Record", "Transform Data", "Write Result"][index % 4]} ${index + 1}`,
		kind: kinds[index % 4],
	}));
	const edges: Fixture["edges"] = [];
	for (let index = 0; index < count; index++) {
		if (index % columns < columns - 1 && index + 1 < count)
			edges.push({
				id: `exec-${index}`,
				source: index,
				target: index + 1,
				sourcePin: 0,
				targetPin: 0,
				kind: "execution",
			});
		if (index + columns < count)
			edges.push({
				id: `data-${index}`,
				source: index,
				target: index + columns,
				sourcePin: 1 + (index % 3),
				targetPin: 1 + (index % 3),
				kind: "data",
			});
	}
	return {
		nodes,
		edges,
		nodeWidth: 150,
		nodeHeight: 88,
		pinStart: 28,
		pinStep: 15,
		pinLabels: ["exec", "value", "count", "result"],
		width: (columns - 1) * 230 + 150,
		height: (Math.ceil(count / columns) - 1) * 132 + 88,
		seed: 0,
	};
}

export function initialViewport(
	fixture: Fixture,
	width: number,
	height: number,
	view: string,
): Viewport {
	const zoom =
		view === "detail"
			? 0.8
			: Math.min((width - 48) / fixture.width, (height - 48) / fixture.height);
	return {
		x: (width - fixture.width * zoom) / 2,
		y: (height - fixture.height * zoom) / 2,
		zoom,
	};
}

/** An inexpensive identity check runs outside all timed intervals. */
export function fixtureChecksum(fixture: Fixture): string {
	let hash = 2166136261;
	const input = JSON.stringify(fixture);
	for (let index = 0; index < input.length; index++) {
		hash ^= input.charCodeAt(index);
		hash = Math.imul(hash, 16777619);
	}
	return (hash >>> 0).toString(16).padStart(8, "0");
}
