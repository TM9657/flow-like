import { describe, expect, test } from "bun:test";
import { createFixture, fixtureChecksum, initialViewport } from "./fixture";

describe("shared renderer workload", () => {
	test("10,000 nodes have 19,800 valid connections with matching pin anchors", () => {
		const fixture = createFixture();
		expect(fixture.nodes).toHaveLength(10_000);
		expect(fixture.edges).toHaveLength(19_800);
		expect(
			fixture.edges.filter((edge) => edge.kind === "execution"),
		).toHaveLength(9_900);
		expect(new Set(fixture.edges.map((edge) => edge.id)).size).toBe(19_800);
		for (const edge of fixture.edges) {
			expect(fixture.nodes[edge.source]).toBeDefined();
			expect(fixture.nodes[edge.target]).toBeDefined();
			expect(edge.source).not.toBe(edge.target);
			expect(edge.sourcePin).toBe(edge.targetPin);
			expect(edge.sourcePin).toBeGreaterThanOrEqual(0);
			expect(edge.sourcePin).toBeLessThan(4);
		}
		expect(fixtureChecksum(fixture)).toBe(fixtureChecksum(createFixture()));
	});
	test("overview fits the whole graph, detail keeps the same graph", () => {
		const fixture = createFixture();
		const viewport = initialViewport(fixture, 1600, 900, "overview");
		expect(viewport.x).toBeGreaterThanOrEqual(24);
		expect(viewport.y).toBeGreaterThanOrEqual(24);
		expect(viewport.x + fixture.width * viewport.zoom).toBeLessThanOrEqual(
			1576,
		);
		expect(viewport.y + fixture.height * viewport.zoom).toBeLessThanOrEqual(
			876,
		);
		expect(initialViewport(fixture, 1600, 900, "detail").zoom).toBe(0.8);
	});
});
