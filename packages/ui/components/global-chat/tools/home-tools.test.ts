import { describe, expect, test } from "vitest";
import type { IBackendState } from "../../../state/backend-state";
import type { IHomeLayout } from "../../home/types";
import {
	getHomeWidgetCatalog,
	listHomeDataSources,
	validateHomeLayoutCandidate,
	validateHomeLayoutReferences,
	validateUnknownHomeWidgetPreservation,
} from "./home-tools";

const layout = (widgets: IHomeLayout["widgets"]): IHomeLayout => ({
	version: 1,
	widgets,
});

const widget = (
	id: string,
	type = "information",
	config: Record<string, unknown> = {},
): IHomeLayout["widgets"][number] => ({
	id,
	type,
	title: id,
	size: { columns: 6, rows: 3 },
	appearance: { variant: "card", accent: "neutral" },
	config,
});

describe("Home FlowPilot tools", () => {
	test("catalog filtering returns exact creation templates", () => {
		const result = getHomeWidgetCatalog({
			category: "assistant",
			query: "hero",
		});
		expect(result).toMatchObject({
			status: "ok",
			total: 1,
			layout_contract: {
				version: 1,
				max_widgets: 80,
				max_bytes: 131_072,
				grid_columns: { mobile: 1, tablet: 6, desktop: 12 },
				height_modes: ["auto", "content", "fixed"],
			},
			data_options: {
				aggregations: expect.arrayContaining(["count", "sum", "median"]),
				filter_operators: expect.arrayContaining(["eq", "contains", "empty"]),
			},
			renderings: {
				apps: expect.arrayContaining([
					expect.objectContaining({ id: "carousel" }),
				]),
				models: expect.arrayContaining([
					expect.objectContaining({ id: "list" }),
				]),
				packages: expect.arrayContaining([
					expect.objectContaining({ id: "featured" }),
				]),
			},
		});
		expect(result.presets?.[0]).toMatchObject({
			preset_id: "flowpilot-hero",
			widget: {
				type: "flowpilot",
				size: { columns: 12, rows: 4 },
				config: { mode: "hero" },
			},
		});
	});

	test("every catalog template satisfies local presentation constraints", () => {
		const catalog = getHomeWidgetCatalog({});
		if (!("presets" in catalog)) throw new Error("Catalog was unavailable");
		const bindingErrors = new Set([
			"app_embed_missing_app",
			"app_embed_missing_event",
			"data_source_missing_app",
			"data_source_missing",
			"ontology_object_type_missing",
		]);
		for (const preset of catalog.presets) {
			const validation = validateHomeLayoutCandidate(
				layout([{ id: preset.preset_id, ...preset.widget }]),
			);
			const presentationErrors = validation.issues.filter(
				(entry) => entry.severity === "error" && !bindingErrors.has(entry.code),
			);
			expect(presentationErrors, preset.preset_id).toEqual([]);
		}
	});

	test("candidate validation canonicalizes optional fields and rejects bad presentation values", () => {
		const normalized = validateHomeLayoutCandidate({
			version: 1,
			widgets: [{ id: "welcome", type: "greeting" }],
		});
		expect(normalized).toMatchObject({
			status: "ok",
			valid: true,
			canonical_layout: {
				widgets: [
					{
						id: "welcome",
						size: { columns: 6, rows: 3 },
						appearance: { variant: "card", accent: "neutral" },
					},
				],
			},
		});
		expect(normalized.fingerprint).toMatch(/^fp1:[0-9a-f]{32}$/);

		const invalid = validateHomeLayoutCandidate(
			layout([
				{
					...widget("bad"),
					appearance: { variant: "glass", accent: "magenta" },
				},
			]),
		);
		expect(invalid.valid).toBe(false);
		expect(invalid.issues.map((entry) => entry.code)).toEqual(
			expect.arrayContaining([
				"unknown_widget_variant",
				"unknown_widget_accent",
			]),
		);

		for (const [type, variant] of [
			["app-collection", "icons"],
			["app-collection", "grid"],
			["packages", "grid"],
			["models", "list"],
			["quick-links", "list"],
		] as const) {
			const legacy = widget(`legacy-${type}-${variant}`, type);
			legacy.appearance.variant = variant;
			expect(validateHomeLayoutCandidate(layout([legacy])).valid).toBe(true);
		}
	});

	test("unsupported widgets are only allowed when preserved exactly", () => {
		const existing = layout([
			widget("known"),
			widget("future", "future-widget", { value: 1 }),
		]);
		const reordered = layout([existing.widgets[1], existing.widgets[0]]);
		const local = validateHomeLayoutCandidate(
			layout([
				{
					...existing.widgets[1],
					size: { columns: 1, rows: 1 },
					appearance: { variant: "future-surface", accent: "future-accent" },
				},
			]),
		);
		expect(local.valid).toBe(true);
		expect(local.issues.map((entry) => entry.code)).toEqual([
			"unknown_widget_type",
		]);
		expect(validateUnknownHomeWidgetPreservation(reordered, existing)).toEqual(
			[],
		);

		const changed = structuredClone(reordered);
		changed.widgets[0].config.value = 2;
		expect(
			validateUnknownHomeWidgetPreservation(changed, existing),
		).toMatchObject([{ code: "unknown_widget_changed" }]);

		const removed = layout([existing.widgets[0]]);
		expect(
			validateUnknownHomeWidgetPreservation(removed, existing),
		).toMatchObject([{ code: "unknown_widget_removed" }]);
	});

	test("data-source listing is profile-bound and returns compact schemas", async () => {
		let reads = 0;
		const backend = {
			dbState: {
				listTables: async () => {
					reads++;
					return ["orders"];
				},
				getSchema: async () => ({
					fields: [
						{ name: "region", data_type: "Utf8" },
						{ name: "revenue", data_type: "Float64" },
					],
				}),
			},
			graphState: {},
			queryState: {},
		} as unknown as Pick<
			IBackendState,
			"dbState" | "graphState" | "queryState"
		>;

		expect(
			await listHomeDataSources(
				backend,
				{ app_id: "private", kinds: ["table"] },
				async () => false,
			),
		).toMatchObject({ code: "home_data_app_not_in_profile" });
		expect(reads).toBe(0);

		const result = await listHomeDataSources(
			backend,
			{ app_id: "visible", kinds: ["table"] },
			async () => true,
		);
		expect(result).toMatchObject({
			status: "ok",
			app_id: "visible",
			sources: {
				tables: [
					{
						name: "orders",
						columns: [
							{ name: "region", type_name: "Utf8" },
							{ name: "revenue", type_name: "Float64" },
						],
					},
				],
			},
		});
	});

	test("data-source listing marks every nested truncation as incomplete", async () => {
		const fields = Array.from({ length: 81 }, (_, index) => ({
			name: `field_${index}`,
			data_type: "Utf8",
		}));
		const backend = {
			dbState: {
				listTables: async () => ["large_table"],
				getSchema: async () => ({ fields }),
			},
			graphState: {
				listOverlays: async () => [
					{
						id: "ontology",
						name: "Ontology",
						description: "",
						nodes: [
							{
								label: "Thing",
								table: "things",
								id_column: "field_0",
								display_column: "field_1",
								property_columns: fields,
							},
						],
					},
				],
			},
			queryState: {},
		} as unknown as Pick<
			IBackendState,
			"dbState" | "graphState" | "queryState"
		>;

		const result = await listHomeDataSources(
			backend,
			{ app_id: "visible", kinds: ["table", "ontology"] },
			async () => true,
		);
		expect(result).toMatchObject({
			status: "partial",
			complete: false,
			truncated: true,
			sources: {
				tables: [{ columns_truncated: true }],
				ontologies: [{ object_types: [{ columns_truncated: true }] }],
			},
		});
	});

	test("data-source query filters before inventory caps", async () => {
		const tables = Array.from({ length: 70 }, (_, index) => `table_${index}`);
		tables.push("needle_table");
		const ontologies = Array.from({ length: 70 }, (_, index) => ({
			id: `ontology_${index}`,
			name: `Ontology ${index}`,
			description: "",
			nodes: [],
		}));
		ontologies.push({
			id: "needle_ontology",
			name: "Needle ontology",
			description: "",
			nodes: [],
		});
		const queries = Array.from({ length: 110 }, (_, index) => ({
			id: `query_${index}`,
			name: `Query ${index}`,
			description: "",
			kind: "sql",
			surface: "table",
			sql: "select 1",
		}));
		queries.push({
			id: "needle_query",
			name: "Needle query",
			description: "",
			kind: "sql",
			surface: "table",
			sql: "select 1",
		});
		const backend = {
			dbState: {
				listTables: async () => tables,
				getSchema: async () => ({ fields: [] }),
			},
			graphState: { listOverlays: async () => ontologies },
			queryState: { listSavedQueries: async () => queries },
		} as unknown as Pick<
			IBackendState,
			"dbState" | "graphState" | "queryState"
		>;

		const result = await listHomeDataSources(
			backend,
			{
				app_id: "visible",
				kinds: ["table", "ontology", "query"],
				query: "needle",
			},
			async () => true,
		);
		expect(result).toMatchObject({
			status: "ok",
			complete: true,
			query: "needle",
			matched_totals: { tables: 1, ontologies: 1, queries: 1 },
			sources: {
				tables: [{ name: "needle_table" }],
				ontologies: [{ ontology_id: "needle_ontology" }],
				queries: [{ query_id: "needle_query" }],
			},
		});
	});

	test("reference validation rejects missing app interfaces and table fields", async () => {
		const backend = {
			eventState: { getEvents: async () => [] },
			dbState: {
				listTables: async () => ["orders"],
				getSchema: async () => ({
					fields: [{ name: "revenue", data_type: "Float64" }],
				}),
			},
			graphState: {},
			queryState: {},
		} as unknown as Pick<
			IBackendState,
			"dbState" | "eventState" | "graphState" | "queryState"
		>;
		const candidate = layout([
			widget("chat", "app-embed", {
				appId: "app",
				target: "event",
				eventId: "missing",
			}),
			widget("chart", "data", {
				appId: "app",
				sourceKind: "table",
				scope: "project",
				table: "orders",
				visualization: "bar",
				groupBy: "region",
			}),
		]);
		const issues = await validateHomeLayoutReferences(backend, candidate, {
			profileAppIds: new Set(["app"]),
		});
		expect(issues.map((entry) => entry.code)).toEqual(
			expect.arrayContaining([
				"home_app_event_missing",
				"home_data_field_missing",
			]),
		);

		const pseudoSortIssues = await validateHomeLayoutReferences(
			backend,
			layout([
				widget("line", "data", {
					appId: "app",
					sourceKind: "table",
					scope: "project",
					table: "orders",
					visualization: "line",
					groupBy: "revenue",
					sortBy: "group",
				}),
			]),
			{ profileAppIds: new Set(["app"]) },
		);
		expect(
			pseudoSortIssues.some(
				(entry) => entry.code === "home_data_field_missing",
			),
		).toBe(false);
	});
});
