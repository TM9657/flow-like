/**
 * Nivo Chart Data Specifications
 *
 * This file is the single source of truth for:
 * 1. Sample/default data for all chart types
 * 2. Data format specifications/validators
 * 3. Props specifications (required/forbidden)
 * 4. Chart defaults (keys, indexBy, etc.)
 *
 * Used by: NivoChart.tsx, Inspector.tsx, tests
 */

// ============================================================================
// TYPES
// ============================================================================

export interface NivoDataSpec {
	type: "array" | "object";
	validator: (data: unknown) => boolean;
	description: string;
}

export interface NivoPropsSpec {
	required: string[];
	forbidden: string[];
}

export interface NivoChartDefaults {
	indexBy?: string;
	keys?: string[];
	groups?: string[];
}

// ============================================================================
// SAMPLE DATA
// Single source of truth for all chart preview/test data
// ============================================================================

export const NIVO_SAMPLE_DATA: Record<string, unknown> = {
	// Bar: array of objects with indexBy field + keys as properties
	bar: [
		{ country: "USA", burgers: 131, fries: 85, sandwiches: 72 },
		{ country: "Germany", burgers: 95, fries: 108, sandwiches: 86 },
		{ country: "France", burgers: 72, fries: 102, sandwiches: 95 },
	],
	// Line: array of { id, data: [{ x, y }] }
	line: [
		{ id: "Series A", data: [{ x: "Jan", y: 20 }, { x: "Feb", y: 45 }, { x: "Mar", y: 30 }, { x: "Apr", y: 80 }, { x: "May", y: 55 }] },
		{ id: "Series B", data: [{ x: "Jan", y: 50 }, { x: "Feb", y: 35 }, { x: "Mar", y: 60 }, { x: "Apr", y: 40 }, { x: "May", y: 75 }] },
	],
	// Pie: array of { id, value, label? }
	pie: [
		{ id: "javascript", label: "JavaScript", value: 450 },
		{ id: "python", label: "Python", value: 320 },
		{ id: "rust", label: "Rust", value: 180 },
		{ id: "go", label: "Go", value: 120 },
	],
	// Radar: array of objects with indexBy field + keys as properties
	radar: [
		{ taste: "fruity", chardonay: 110, carmenere: 92, syrah: 75 },
		{ taste: "bitter", chardonay: 65, carmenere: 82, syrah: 95 },
		{ taste: "heavy", chardonay: 80, carmenere: 110, syrah: 88 },
		{ taste: "strong", chardonay: 55, carmenere: 70, syrah: 115 },
		{ taste: "sunny", chardonay: 95, carmenere: 60, syrah: 78 },
	],
	// Heatmap: array of { id, data: [{ x, y }] }
	heatmap: [
		{ id: "Mon", data: [{ x: "9am", y: 45 }, { x: "12pm", y: 78 }, { x: "3pm", y: 62 }, { x: "6pm", y: 38 }] },
		{ id: "Tue", data: [{ x: "9am", y: 58 }, { x: "12pm", y: 95 }, { x: "3pm", y: 71 }, { x: "6pm", y: 42 }] },
		{ id: "Wed", data: [{ x: "9am", y: 52 }, { x: "12pm", y: 88 }, { x: "3pm", y: 68 }, { x: "6pm", y: 45 }] },
	],
	// Scatter: array of { id, data: [{ x, y }] } with numeric x
	scatter: [
		{ id: "Group A", data: [{ x: 10, y: 20 }, { x: 25, y: 45 }, { x: 40, y: 30 }, { x: 55, y: 65 }, { x: 70, y: 50 }] },
		{ id: "Group B", data: [{ x: 15, y: 35 }, { x: 30, y: 25 }, { x: 45, y: 55 }, { x: 60, y: 40 }, { x: 75, y: 70 }] },
	],
	// Funnel: array of { id, value, label }
	funnel: [
		{ id: "step_sent", value: 80542, label: "Visitors" },
		{ id: "step_viewed", value: 45230, label: "Signups" },
		{ id: "step_clicked", value: 22890, label: "Trials" },
		{ id: "step_purchased", value: 9875, label: "Customers" },
	],
	// Treemap: hierarchical { id, children: [{ id, value }] }
	// IMPORTANT: Use 'id' field, NOT 'name'
	treemap: {
		id: "root",
		children: [
			{ id: "Frontend", children: [{ id: "React", value: 145 }, { id: "Vue", value: 98 }, { id: "Angular", value: 72 }] },
			{ id: "Backend", children: [{ id: "Node", value: 112 }, { id: "Python", value: 135 }, { id: "Go", value: 78 }] },
		],
	},
	// Sunburst: hierarchical { id, children: [{ id, value }] }
	// IMPORTANT: Use 'id' field, NOT 'name'
	sunburst: {
		id: "root",
		children: [
			{ id: "Design", children: [{ id: "UI", value: 85 }, { id: "UX", value: 72 }] },
			{ id: "Development", children: [{ id: "Frontend", value: 125 }, { id: "Backend", value: 142 }] },
			{ id: "Testing", children: [{ id: "Unit", value: 65 }, { id: "E2E", value: 48 }] },
		],
	},
	// Sankey: { nodes: [{ id }], links: [{ source, target, value }] }
	sankey: {
		nodes: [{ id: "A" }, { id: "B" }, { id: "C" }, { id: "D" }, { id: "E" }],
		links: [
			{ source: "A", target: "B", value: 62 },
			{ source: "A", target: "C", value: 45 },
			{ source: "B", target: "D", value: 38 },
			{ source: "C", target: "D", value: 32 },
			{ source: "D", target: "E", value: 55 },
		],
	},
	// Calendar: array of { day: "YYYY-MM-DD", value }
	calendar: [
		{ day: "2024-01-05", value: 125 },
		{ day: "2024-01-12", value: 245 },
		{ day: "2024-01-20", value: 182 },
		{ day: "2024-02-03", value: 298 },
		{ day: "2024-02-14", value: 165 },
		{ day: "2024-03-01", value: 220 },
		{ day: "2024-03-15", value: 175 },
	],
	// Waffle: array of { id, label, value }
	waffle: [
		{ id: "completed", label: "Completed", value: 68 },
		{ id: "pending", label: "Pending", value: 22 },
		{ id: "failed", label: "Failed", value: 10 },
	],
	// Bump: array of { id, data: [{ x, y }] } where y is ranking (1, 2, 3...)
	bump: [
		{ id: "Team A", data: [{ x: 2020, y: 1 }, { x: 2021, y: 2 }, { x: 2022, y: 1 }, { x: 2023, y: 3 }] },
		{ id: "Team B", data: [{ x: 2020, y: 2 }, { x: 2021, y: 1 }, { x: 2022, y: 3 }, { x: 2023, y: 1 }] },
		{ id: "Team C", data: [{ x: 2020, y: 3 }, { x: 2021, y: 3 }, { x: 2022, y: 2 }, { x: 2023, y: 2 }] },
	],
	// AreaBump: array of { id, data: [{ x, y }] } where y is actual value (larger numbers)
	areaBump: [
		{ id: "JavaScript", data: [{ x: 2020, y: 185 }, { x: 2021, y: 210 }, { x: 2022, y: 245 }, { x: 2023, y: 278 }] },
		{ id: "TypeScript", data: [{ x: 2020, y: 95 }, { x: 2021, y: 142 }, { x: 2022, y: 198 }, { x: 2023, y: 265 }] },
		{ id: "Python", data: [{ x: 2020, y: 165 }, { x: 2021, y: 195 }, { x: 2022, y: 225 }, { x: 2023, y: 252 }] },
	],
	// CirclePacking: hierarchical { id, children/value }
	// IMPORTANT: Use 'id' field, NOT 'name'
	circlePacking: {
		id: "root",
		children: [
			{ id: "Data", children: [{ id: "Analytics", value: 145 }, { id: "ML", value: 112 }] },
			{ id: "Infra", children: [{ id: "Cloud", value: 98 }, { id: "DevOps", value: 85 }] },
		],
	},
	// Network: { nodes: [{ id }], links: [{ source, target }] }
	network: {
		nodes: [{ id: "A" }, { id: "B" }, { id: "C" }, { id: "D" }, { id: "E" }],
		links: [
			{ source: "A", target: "B" },
			{ source: "A", target: "C" },
			{ source: "B", target: "D" },
			{ source: "C", target: "D" },
			{ source: "D", target: "E" },
		],
	},
	// Stream: array of objects with keys as properties (stacked data)
	stream: [
		{ React: 145, Vue: 82, Angular: 65 },
		{ React: 168, Vue: 95, Angular: 58 },
		{ React: 192, Vue: 110, Angular: 52 },
		{ React: 215, Vue: 125, Angular: 48 },
		{ React: 238, Vue: 142, Angular: 45 },
	],
	// Swarmplot: array of { id, group, price/value }
	swarmplot: [
		{ id: "0", group: "Category A", price: 125 },
		{ id: "1", group: "Category A", price: 245 },
		{ id: "2", group: "Category A", price: 182 },
		{ id: "3", group: "Category B", price: 98 },
		{ id: "4", group: "Category B", price: 312 },
		{ id: "5", group: "Category B", price: 165 },
		{ id: "6", group: "Category C", price: 278 },
		{ id: "7", group: "Category C", price: 145 },
	],
	// Voronoi: array of { id, x, y }
	voronoi: [
		{ id: "1", x: 15, y: 25 },
		{ id: "2", x: 42, y: 58 },
		{ id: "3", x: 68, y: 32 },
		{ id: "4", x: 85, y: 72 },
		{ id: "5", x: 28, y: 88 },
		{ id: "6", x: 55, y: 15 },
	],
	// Marimekko: array of { id/statement, value/participation, dimension values }
	marimekko: [
		{ statement: "Quality", participation: 85, stronglyAgree: 45, agree: 28, disagree: 18, stronglyDisagree: 9 },
		{ statement: "Price", participation: 72, stronglyAgree: 32, agree: 35, disagree: 22, stronglyDisagree: 11 },
		{ statement: "Service", participation: 68, stronglyAgree: 52, agree: 25, disagree: 15, stronglyDisagree: 8 },
	],
	// ParallelCoordinates: array of objects with variables as properties
	parallelCoordinates: [
		{ temp: 25, cost: 8500, volume: 125, efficiency: 78 },
		{ temp: 42, cost: 15200, volume: 85, efficiency: 92 },
		{ temp: 58, cost: 12800, volume: 145, efficiency: 65 },
		{ temp: 35, cost: 9800, volume: 110, efficiency: 85 },
		{ temp: 48, cost: 18500, volume: 72, efficiency: 95 },
	],
	// RadialBar: array of { id, data: [{ x, y }] }
	radialBar: [
		{ id: "Store A", data: [{ x: "Vegetables", y: 72 }, { x: "Fruits", y: 85 }, { x: "Meat", y: 45 }] },
		{ id: "Store B", data: [{ x: "Vegetables", y: 58 }, { x: "Fruits", y: 65 }, { x: "Meat", y: 78 }] },
		{ id: "Store C", data: [{ x: "Vegetables", y: 82 }, { x: "Fruits", y: 48 }, { x: "Meat", y: 62 }] },
	],
	// Boxplot: array of { group, subgroup?, value }
	boxplot: [
		{ group: "Alpha", subgroup: "A", value: 12 },
		{ group: "Alpha", subgroup: "A", value: 28 },
		{ group: "Alpha", subgroup: "A", value: 45 },
		{ group: "Alpha", subgroup: "A", value: 62 },
		{ group: "Alpha", subgroup: "A", value: 85 },
		{ group: "Alpha", subgroup: "B", value: 18 },
		{ group: "Alpha", subgroup: "B", value: 35 },
		{ group: "Alpha", subgroup: "B", value: 52 },
		{ group: "Beta", subgroup: "A", value: 25 },
		{ group: "Beta", subgroup: "A", value: 48 },
		{ group: "Beta", subgroup: "A", value: 72 },
		{ group: "Beta", subgroup: "B", value: 15 },
		{ group: "Beta", subgroup: "B", value: 38 },
		{ group: "Beta", subgroup: "B", value: 58 },
	],
	// Bullet: array of { id, ranges: number[], measures: number[], markers?: number[] }
	// IMPORTANT: ranges needs 3+ values, NOT 2. Uses 'id' not 'title'
	bullet: [
		{ id: "temp.", ranges: [10, 36, 100], measures: [56], markers: [76] },
		{ id: "power", ranges: [0.2, 0.6, 1], measures: [0.76], markers: [0.88] },
		{ id: "volume", ranges: [25, 50, 100], measures: [60], markers: [70] },
	],
	// Chord: matrix format (2D array) - values represent relationships between keys
	// Diagonal should be 0 (no self-connections)
	chord: [
		[0, 83, 29, 50, 117],
		[83, 0, 76, 44, 61],
		[29, 76, 0, 81, 95],
		[50, 44, 81, 0, 72],
		[117, 61, 95, 72, 0],
	],
};

// ============================================================================
// CHART DEFAULTS
// Keys, indexBy, groups for charts that need them
// ============================================================================

export const NIVO_CHART_DEFAULTS: Record<string, NivoChartDefaults> = {
	bar: { indexBy: "country", keys: ["burgers", "fries", "sandwiches"] },
	radar: { indexBy: "taste", keys: ["chardonay", "carmenere", "syrah"] },
	stream: { keys: ["React", "Vue", "Angular"] },
	swarmplot: { groups: ["Category A", "Category B", "Category C"] },
	chord: { keys: ["John", "Raoul", "Jane", "Marcel", "Ibrahim"] },
	marimekko: { keys: ["stronglyAgree", "agree", "disagree", "stronglyDisagree"] },
	parallelCoordinates: { keys: ["temp", "cost", "volume", "efficiency"] },
};

// ============================================================================
// DATA SPECIFICATIONS
// Validators for each chart type's data format
// ============================================================================

export const NIVO_DATA_SPECS: Record<string, NivoDataSpec> = {
	bar: {
		type: "array",
		validator: (item) => typeof item === "object" && item !== null && Object.values(item).some(v => typeof v === "number"),
		description: "Array of { [indexBy]: string, [key1]: number, [key2]: number, ... }",
	},
	line: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && Array.isArray(i?.data) &&
				i.data.every((d: unknown) => {
					const point = d as Record<string, unknown>;
					return point?.x !== undefined && typeof point?.y === "number";
				});
		},
		description: "Array of { id: string, data: [{ x: string|number, y: number }] }",
	},
	pie: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && typeof i?.value === "number";
		},
		description: "Array of { id: string, label?: string, value: number }",
	},
	radar: {
		type: "array",
		validator: (item) => typeof item === "object" && item !== null && Object.values(item).some(v => typeof v === "number"),
		description: "Array of { [indexBy]: string, [key1]: number, [key2]: number, ... }",
	},
	heatmap: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && Array.isArray(i?.data) &&
				i.data.every((d: unknown) => {
					const cell = d as Record<string, unknown>;
					return cell?.x !== undefined && typeof cell?.y === "number";
				});
		},
		description: "Array of { id: string, data: [{ x: string|number, y: number }] }",
	},
	scatter: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && Array.isArray(i?.data) &&
				i.data.every((d: unknown) => {
					const point = d as Record<string, unknown>;
					return typeof point?.x === "number" && typeof point?.y === "number";
				});
		},
		description: "Array of { id: string, data: [{ x: number, y: number }] }",
	},
	funnel: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return (typeof i?.id === "string" || typeof i?.id === "number") && typeof i?.value === "number";
		},
		description: "Array of { id: string|number, value: number, label?: string }",
	},
	treemap: {
		type: "object",
		validator: (data) => {
			const d = data as Record<string, unknown>;
			const hasValidStructure = typeof d?.id === "string" && (Array.isArray(d?.children) || typeof d?.value === "number");
			if (!hasValidStructure) return false;
			const checkChildren = (node: Record<string, unknown>): boolean => {
				if (!node.children) return true;
				return (node.children as Record<string, unknown>[]).every(child =>
					typeof child.id === "string" && checkChildren(child)
				);
			};
			return checkChildren(d);
		},
		description: "{ id: string, children?: [...], value?: number } - hierarchical with 'id' field",
	},
	sunburst: {
		type: "object",
		validator: (data) => {
			const d = data as Record<string, unknown>;
			const hasValidStructure = typeof d?.id === "string" && (Array.isArray(d?.children) || typeof d?.value === "number");
			if (!hasValidStructure) return false;
			const checkChildren = (node: Record<string, unknown>): boolean => {
				if (!node.children) return true;
				return (node.children as Record<string, unknown>[]).every(child =>
					typeof child.id === "string" && checkChildren(child)
				);
			};
			return checkChildren(d);
		},
		description: "{ id: string, children?: [...], value?: number } - hierarchical with 'id' field",
	},
	circlePacking: {
		type: "object",
		validator: (data) => {
			const d = data as Record<string, unknown>;
			const hasValidStructure = typeof d?.id === "string" && (Array.isArray(d?.children) || typeof d?.value === "number");
			if (!hasValidStructure) return false;
			const checkChildren = (node: Record<string, unknown>): boolean => {
				if (!node.children) return true;
				return (node.children as Record<string, unknown>[]).every(child =>
					typeof child.id === "string" && checkChildren(child)
				);
			};
			return checkChildren(d);
		},
		description: "{ id: string, children?: [...], value?: number } - hierarchical with 'id' field",
	},
	sankey: {
		type: "object",
		validator: (data) => {
			const d = data as { nodes?: { id: string }[]; links?: { source: string; target: string; value: number }[] };
			if (!Array.isArray(d?.nodes) || !Array.isArray(d?.links)) return false;
			const nodeIds = new Set(d.nodes.map(n => n.id));
			return d.links.every(link =>
				nodeIds.has(link.source) && nodeIds.has(link.target) && typeof link.value === "number"
			);
		},
		description: "{ nodes: [{ id }], links: [{ source, target, value }] }",
	},
	network: {
		type: "object",
		validator: (data) => {
			const d = data as { nodes?: { id: string }[]; links?: { source: string; target: string }[] };
			if (!Array.isArray(d?.nodes) || !Array.isArray(d?.links)) return false;
			const nodeIds = new Set(d.nodes.map(n => n.id));
			return d.links.every(link => nodeIds.has(link.source) && nodeIds.has(link.target));
		},
		description: "{ nodes: [{ id }], links: [{ source, target }] }",
	},
	calendar: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.day === "string" && /^\d{4}-\d{2}-\d{2}$/.test(i.day) && typeof i?.value === "number";
		},
		description: "Array of { day: 'YYYY-MM-DD', value: number }",
	},
	waffle: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && typeof i?.value === "number";
		},
		description: "Array of { id: string, label?: string, value: number }",
	},
	bump: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && Array.isArray(i?.data) &&
				i.data.every((d: unknown) => {
					const point = d as Record<string, unknown>;
					return typeof point?.x === "number" && typeof point?.y === "number";
				});
		},
		description: "Array of { id: string, data: [{ x: number, y: number (rank) }] }",
	},
	areaBump: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && Array.isArray(i?.data) &&
				i.data.every((d: unknown) => {
					const point = d as Record<string, unknown>;
					return typeof point?.x === "number" && typeof point?.y === "number";
				});
		},
		description: "Array of { id: string, data: [{ x: number, y: number (value) }] }",
	},
	stream: {
		type: "array",
		validator: (item) => typeof item === "object" && item !== null && Object.values(item).every(v => typeof v === "number"),
		description: "Array of { [key1]: number, [key2]: number, ... }",
	},
	swarmplot: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && typeof i?.group === "string" && typeof i?.price === "number";
		},
		description: "Array of { id: string, group: string, price: number }",
	},
	voronoi: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && typeof i?.x === "number" && typeof i?.y === "number";
		},
		description: "Array of { id: string, x: number, y: number }",
	},
	marimekko: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.statement === "string" && typeof i?.participation === "number";
		},
		description: "Array of { statement: string, participation: number, [dimension]: number, ... }",
	},
	parallelCoordinates: {
		type: "array",
		validator: (item) => typeof item === "object" && item !== null && Object.values(item).every(v => typeof v === "number"),
		description: "Array of { [var1]: number, [var2]: number, ... }",
	},
	radialBar: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.id === "string" && Array.isArray(i?.data) &&
				i.data.every((d: unknown) => {
					const point = d as Record<string, unknown>;
					return typeof point?.x === "string" && typeof point?.y === "number";
				});
		},
		description: "Array of { id: string, data: [{ x: string, y: number }] }",
	},
	boxplot: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			return typeof i?.group === "string" && typeof i?.value === "number";
		},
		description: "Array of { group: string, subgroup?: string, value: number }",
	},
	bullet: {
		type: "array",
		validator: (item) => {
			const i = item as Record<string, unknown>;
			const ranges = i?.ranges as number[] | undefined;
			const measures = i?.measures as number[] | undefined;
			return typeof i?.id === "string" &&
				Array.isArray(ranges) && ranges.length >= 3 &&
				Array.isArray(measures) && measures.length > 0;
		},
		description: "Array of { id: string, ranges: number[] (3+), measures: number[], markers?: number[] }",
	},
	chord: {
		type: "object",
		validator: (data) => {
			if (!Array.isArray(data)) return false;
			const matrix = data as number[][];
			const size = matrix.length;
			if (size === 0) return false;
			return matrix.every((row, i) =>
				Array.isArray(row) && row.length === size && row[i] === 0
			);
		},
		description: "2D matrix where matrix[i][i] = 0 (no self-connections)",
	},
};

// ============================================================================
// PROPS SPECIFICATIONS
// Required and forbidden props per chart type
// ============================================================================

export const NIVO_PROPS_SPECS: Record<string, NivoPropsSpec> = {
	bar: { required: ["data", "indexBy", "keys"], forbidden: [] },
	line: { required: ["data"], forbidden: [] },
	pie: { required: ["data"], forbidden: [] },
	radar: { required: ["data", "indexBy", "keys"], forbidden: [] },
	heatmap: { required: ["data"], forbidden: [] },
	scatter: { required: ["data"], forbidden: [] },
	funnel: { required: ["data"], forbidden: [] },
	treemap: { required: ["data"], forbidden: [] },
	sunburst: { required: ["data"], forbidden: [] },
	circlePacking: { required: ["data"], forbidden: [] },
	sankey: { required: ["data"], forbidden: [] },
	network: { required: ["data"], forbidden: [] },
	calendar: { required: ["data", "from", "to"], forbidden: [] },
	waffle: { required: ["data", "total", "rows", "columns"], forbidden: [] },
	bump: { required: ["data"], forbidden: [] },
	areaBump: { required: ["data"], forbidden: [] },
	stream: { required: ["data", "keys"], forbidden: [] },
	swarmplot: { required: ["data", "groups", "value"], forbidden: [] },
	voronoi: { required: ["data"], forbidden: [] },
	marimekko: { required: ["data", "id", "value", "dimensions"], forbidden: [] },
	parallelCoordinates: { required: ["data", "variables"], forbidden: [] },
	radialBar: { required: ["data"], forbidden: [] },
	boxplot: { required: ["data"], forbidden: [] },
	// Bullet: Uses rangeColors/measureColors/markerColors, NOT standard 'colors'
	bullet: { required: ["data"], forbidden: ["colors", "legends"] },
	// Chord: Matrix format, does NOT support 'legends' prop
	chord: { required: ["matrix", "keys"], forbidden: ["legends"] },
};

// ============================================================================
// SUPPORTED CHART TYPES
// ============================================================================

export const NIVO_CHART_TYPES = [
	"bar", "line", "pie", "radar", "heatmap", "scatter", "funnel",
	"treemap", "sunburst", "calendar", "bump", "areaBump", "circlePacking",
	"network", "sankey", "stream", "swarmplot", "voronoi", "waffle",
	"marimekko", "parallelCoordinates", "radialBar", "boxplot", "bullet", "chord"
] as const;

export type NivoChartType = typeof NIVO_CHART_TYPES[number];

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

export function validateNivoData(chartType: string, data: unknown): { valid: boolean; error?: string } {
	const spec = NIVO_DATA_SPECS[chartType];
	if (!spec) {
		return { valid: false, error: `Unknown chart type: ${chartType}` };
	}

	if (spec.type === "array") {
		if (!Array.isArray(data)) {
			return { valid: false, error: `Expected array, got ${typeof data}` };
		}
		for (let i = 0; i < data.length; i++) {
			if (!spec.validator(data[i])) {
				return { valid: false, error: `Invalid item at index ${i}. Expected: ${spec.description}` };
			}
		}
	} else {
		if (!spec.validator(data)) {
			return { valid: false, error: `Invalid data structure. Expected: ${spec.description}` };
		}
	}

	return { valid: true };
}
