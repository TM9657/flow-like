/**
 * NivoChart Data Format and Props Tests
 *
 * Tests validate that:
 * 1. NIVO_SAMPLE_DATA has correct structure for each chart type
 * 2. Data format validators work correctly
 * 3. Critical edge cases are handled (bullet ranges, chord matrix, hierarchical id fields)
 */
import { describe, test, expect } from "bun:test";
import {
	NIVO_SAMPLE_DATA,
	NIVO_DATA_SPECS,
	NIVO_PROPS_SPECS,
	NIVO_CHART_TYPES,
	NIVO_CHART_DEFAULTS,
	validateNivoData,
} from "./nivo-data";

// ============================================================================
// FORMAT VALIDATION TESTS
// ============================================================================

describe("NivoChart SAMPLE_DATA Format Validation", () => {
	for (const chartType of NIVO_CHART_TYPES) {
		test(`${chartType}: data structure matches specification`, () => {
			const data = NIVO_SAMPLE_DATA[chartType];
			const spec = NIVO_DATA_SPECS[chartType];

			expect(data).toBeDefined();
			expect(spec).toBeDefined();

			// Validate type
			if (spec.type === "array") {
				expect(Array.isArray(data)).toBe(true);
				const arr = data as unknown[];
				expect(arr.length).toBeGreaterThan(0);

				// Validate each item
				for (const item of arr) {
					expect(spec.validator(item)).toBe(true);
				}
			} else {
				expect(spec.validator(data)).toBe(true);
			}
		});
	}
});

// ============================================================================
// HIERARCHICAL DATA VALIDATION
// Treemap, Sunburst, CirclePacking use 'id' field (NOT 'name')
// ============================================================================

describe("NivoChart Hierarchical Data uses 'id' field (not 'name')", () => {
	const hierarchicalTypes = ["treemap", "sunburst", "circlePacking"];

	test("treemap/sunburst/circlePacking: all nodes use 'id', no 'name' field", () => {
		for (const chartType of hierarchicalTypes) {
			const data = NIVO_SAMPLE_DATA[chartType] as Record<string, unknown>;

			const checkNode = (node: Record<string, unknown>) => {
				expect(node.id).toBeDefined();
				expect(typeof node.id).toBe("string");
				expect((node as { name?: unknown }).name).toBeUndefined();

				if (node.children) {
					for (const child of node.children as Record<string, unknown>[]) {
						checkNode(child);
					}
				}
			};

			checkNode(data);
		}
	});
});

// ============================================================================
// BULLET CHART VALIDATION
// - Uses 'id' not 'title'
// - ranges needs 3+ values (not 2)
// ============================================================================

describe("NivoChart Bullet Data Validation", () => {
	test("bullet: ranges has 3+ values (not 2)", () => {
		const data = NIVO_SAMPLE_DATA.bullet as { id: string; ranges: number[] }[];

		for (const item of data) {
			expect(item.ranges.length).toBeGreaterThanOrEqual(3);
		}
	});

	test("bullet: does not have 'title' field (uses 'id' instead)", () => {
		const data = NIVO_SAMPLE_DATA.bullet as Record<string, unknown>[];

		for (const item of data) {
			expect(item.id).toBeDefined();
			expect((item as { title?: unknown }).title).toBeUndefined();
		}
	});
});

// ============================================================================
// CHORD CHART VALIDATION
// - Matrix format
// - Diagonal is 0 (no self-connections)
// ============================================================================

describe("NivoChart Chord Data Validation", () => {
	test("chord: is a square matrix", () => {
		const data = NIVO_SAMPLE_DATA.chord as number[][];

		expect(Array.isArray(data)).toBe(true);
		const size = data.length;

		for (const row of data) {
			expect(row.length).toBe(size);
		}
	});

	test("chord: diagonal is zero (no self-connections)", () => {
		const data = NIVO_SAMPLE_DATA.chord as number[][];

		for (let i = 0; i < data.length; i++) {
			expect(data[i][i]).toBe(0);
		}
	});
});

// ============================================================================
// CALENDAR DATA VALIDATION
// ============================================================================

describe("NivoChart Calendar Data Validation", () => {
	test("calendar: dates are in YYYY-MM-DD format", () => {
		const data = NIVO_SAMPLE_DATA.calendar as { day: string }[];
		const dateRegex = /^\d{4}-\d{2}-\d{2}$/;

		for (const item of data) {
			expect(dateRegex.test(item.day)).toBe(true);
		}
	});
});

// ============================================================================
// NETWORK/SANKEY DATA VALIDATION
// Links must reference valid nodes
// ============================================================================

describe("NivoChart Network/Sankey Data Validation", () => {
	test("network: all link sources/targets exist in nodes", () => {
		const data = NIVO_SAMPLE_DATA.network as { nodes: { id: string }[]; links: { source: string; target: string }[] };
		const nodeIds = new Set(data.nodes.map(n => n.id));

		for (const link of data.links) {
			expect(nodeIds.has(link.source)).toBe(true);
			expect(nodeIds.has(link.target)).toBe(true);
		}
	});

	test("sankey: all link sources/targets exist in nodes", () => {
		const data = NIVO_SAMPLE_DATA.sankey as { nodes: { id: string }[]; links: { source: string; target: string; value: number }[] };
		const nodeIds = new Set(data.nodes.map(n => n.id));

		for (const link of data.links) {
			expect(nodeIds.has(link.source)).toBe(true);
			expect(nodeIds.has(link.target)).toBe(true);
			expect(link.value).toBeGreaterThan(0);
		}
	});
});

// ============================================================================
// PROPS SPECIFICATIONS
// ============================================================================

describe("NivoChart Props Specifications", () => {
	test("bullet: should not receive standard 'colors' prop", () => {
		const spec = NIVO_PROPS_SPECS.bullet;
		expect(spec.forbidden).toContain("colors");
	});

	test("chord: should not receive standard 'legends' prop", () => {
		const spec = NIVO_PROPS_SPECS.chord;
		expect(spec.forbidden).toContain("legends");
	});

	test("all chart types have required and forbidden arrays", () => {
		for (const chartType of NIVO_CHART_TYPES) {
			const spec = NIVO_PROPS_SPECS[chartType];
			expect(spec).toBeDefined();
			expect(Array.isArray(spec.required)).toBe(true);
			expect(Array.isArray(spec.forbidden)).toBe(true);
		}
	});
});

// ============================================================================
// VALIDATION HELPER TESTS
// ============================================================================

describe("validateNivoData helper", () => {
	test("returns valid: true for correct data", () => {
		for (const chartType of NIVO_CHART_TYPES) {
			const result = validateNivoData(chartType, NIVO_SAMPLE_DATA[chartType]);
			expect(result.valid).toBe(true);
			expect(result.error).toBeUndefined();
		}
	});

	test("returns valid: false for invalid array type", () => {
		const result = validateNivoData("bar", "not an array");
		expect(result.valid).toBe(false);
		expect(result.error).toContain("Expected array");
	});

	test("returns valid: false for unknown chart type", () => {
		const result = validateNivoData("unknown", []);
		expect(result.valid).toBe(false);
		expect(result.error).toContain("Unknown chart type");
	});

	test("returns valid: false for invalid item in array", () => {
		const result = validateNivoData("bar", [{ invalid: "data" }]);
		expect(result.valid).toBe(false);
		expect(result.error).toContain("Invalid item");
	});
});

// ============================================================================
// CHART DEFAULTS
// ============================================================================

describe("NivoChart Defaults", () => {
	test("bar chart has indexBy and keys defaults", () => {
		const defaults = NIVO_CHART_DEFAULTS.bar;
		expect(defaults?.indexBy).toBe("country");
		expect(defaults?.keys).toEqual(["burgers", "fries", "sandwiches"]);
	});

	test("chord chart has keys default", () => {
		const defaults = NIVO_CHART_DEFAULTS.chord;
		expect(defaults?.keys).toBeDefined();
		expect(defaults?.keys?.length).toBe(5); // Match matrix size
	});

	test("defaults keys match sample data structure", () => {
		// Bar: keys should be properties in sample data
		const barData = NIVO_SAMPLE_DATA.bar as Record<string, unknown>[];
		const barDefaults = NIVO_CHART_DEFAULTS.bar;
		for (const key of barDefaults?.keys ?? []) {
			expect(barData[0]).toHaveProperty(key);
		}

		// Stream: keys should be properties in sample data
		const streamData = NIVO_SAMPLE_DATA.stream as Record<string, unknown>[];
		const streamDefaults = NIVO_CHART_DEFAULTS.stream;
		for (const key of streamDefaults?.keys ?? []) {
			expect(streamData[0]).toHaveProperty(key);
		}
	});
});

// ============================================================================
// DATA COMPLETENESS
// ============================================================================

describe("NivoChart Data Completeness", () => {
	test("all chart types have sample data", () => {
		for (const chartType of NIVO_CHART_TYPES) {
			expect(NIVO_SAMPLE_DATA[chartType]).toBeDefined();
		}
	});

	test("all chart types have data spec", () => {
		for (const chartType of NIVO_CHART_TYPES) {
			expect(NIVO_DATA_SPECS[chartType]).toBeDefined();
		}
	});

	test("all chart types have props spec", () => {
		for (const chartType of NIVO_CHART_TYPES) {
			expect(NIVO_PROPS_SPECS[chartType]).toBeDefined();
		}
	});

	test("NIVO_CHART_TYPES has 25 chart types", () => {
		expect(NIVO_CHART_TYPES.length).toBe(25);
	});
});
