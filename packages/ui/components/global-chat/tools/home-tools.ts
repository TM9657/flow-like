import { stableStringify } from "../../../lib/stable-stringify";
import type { IBackendState } from "../../../state/backend-state";
import type { GraphOverlay } from "../../../state/backend-state/graph-state";
import type { SavedQuery } from "../../../state/backend-state/query-state";
import { HOME_WIDGET_PRESETS } from "../../home/catalog";
import { HOME_ACCENTS } from "../../home/home-appearance";
import {
	HOME_APP_RENDERINGS,
	HOME_MODEL_RENDERINGS,
	HOME_PACKAGE_RENDERINGS,
	parseHomeEmbedTarget,
} from "../../home/home-content/config";
import {
	HOME_DATA_AGGREGATIONS,
	HOME_DATA_FILTER_OPERATORS,
	HOME_DATA_VISUALIZATIONS,
	extractHomeQueryParameters,
	homeDataColumns,
	homeOntologyColumns,
	normalizeHomeDataConfig,
} from "../../home/home-data-query";
import {
	MAX_HOME_LAYOUT_BYTES,
	MAX_HOME_WIDGETS,
	homeLayoutByteLength,
	minimumHomeWidgetRows,
	responsiveHomeColumns,
} from "../../home/home-layout";
import {
	homeLayoutFingerprint,
	parseHomeLayoutJson,
} from "../../home/home-layout-json";
import type { IHomeLayout, IHomeWidget } from "../../home/types";
import {
	HOME_WIDGET_CONFIG_CONTRACTS,
	type HomeWidgetConfigCondition,
	type HomeWidgetConfigFieldContract,
	type HomeWidgetObjectContract,
	validateKnownHomeWidgetConfig,
} from "./home-widget-contracts";

export type HomeDataSourceKind = "table" | "ontology" | "query";
export type HomeDataScope = "project" | "personal";

export interface HomeToolIssue {
	severity: "error" | "warning";
	code: string;
	path: string;
	message: string;
}

export interface HomeLayoutValidationResult {
	status: "ok" | "validation_error";
	valid: boolean;
	issues: HomeToolIssue[];
	layout?: IHomeLayout;
	canonical_layout?: IHomeLayout;
	fingerprint?: string;
	byte_count?: number;
}

export type PublicHomeLayoutValidationResult = Omit<
	HomeLayoutValidationResult,
	"layout"
>;

/** Remove the private parsed-layout alias before returning validation to a tool caller. */
export function publicHomeLayoutValidation(
	validation: HomeLayoutValidationResult,
): PublicHomeLayoutValidationResult {
	const { layout, ...publicResult } = validation;
	void layout;
	return publicResult;
}

/** Return comparison metadata without duplicating layouts unless the caller asks for them. */
export function homeLayoutComparisonFields(
	currentLayout: IHomeLayout,
	baseLayout: IHomeLayout,
	defaultLayout: IHomeLayout | undefined,
	includeComparisons: boolean,
) {
	const currentFingerprint = homeLayoutFingerprint(currentLayout);
	const baseFingerprint = homeLayoutFingerprint(baseLayout);
	return {
		base_fingerprint: baseFingerprint,
		...(includeComparisons && baseFingerprint !== currentFingerprint
			? { base_layout: baseLayout }
			: {}),
		...(defaultLayout
			? {
					default_fingerprint: homeLayoutFingerprint(defaultLayout),
					...(includeComparisons &&
					homeLayoutFingerprint(defaultLayout) !== currentFingerprint
						? { default_layout: defaultLayout }
						: {}),
				}
			: {}),
	};
}

const HOME_WIDGET_TYPES = new Set(
	HOME_WIDGET_PRESETS.map((preset) => preset.type),
);
const HOME_CATEGORIES = new Set(
	HOME_WIDGET_PRESETS.map((preset) => preset.category),
);
const HOME_VARIANTS = new Set(["card", "borderless", "tinted", "solid"]);
const LEGACY_VARIANTS_BY_WIDGET_TYPE = new Map<string, Set<string>>([
	[
		"app-collection",
		new Set([
			"grid",
			"standard",
			"compact",
			"list",
			"editorial",
			"icons",
			"carousel",
			"spotlight",
		]),
	],
	["packages", new Set(["grid", "standard", "compact", "featured", "list"])],
	["models", new Set(["grid", "standard", "list"])],
	["quick-links", new Set(["grid", "list"])],
]);
const SOLID_WIDGET_TYPES = new Set([
	"app-spotlight",
	"app-ranking",
	"app-collection-feature",
	"model-spotlight",
]);
const DATA_SOURCE_KINDS: HomeDataSourceKind[] = ["table", "ontology", "query"];
const HOME_DATA_MODES = ["aggregate", "records"] as const;
const HOME_DATA_SCOPES = ["project", "personal"] as const;
const HOME_DATA_TIME_BUCKETS = [
	"none",
	"day",
	"week",
	"month",
	"quarter",
	"year",
] as const;
const HOME_DATA_DATE_RANGES = ["all", "7d", "30d", "90d", "year"] as const;
const HOME_DATA_SORT_DIRECTIONS = ["asc", "desc"] as const;
const HOME_DATA_FORMATS = ["number", "currency", "percent"] as const;
const HOME_DATA_VALUE_TYPES = ["text", "number", "boolean", "viewer"] as const;
const MAX_TABLES = 60;
const MAX_COLUMNS = 80;
const MAX_ONTOLOGIES = 60;
const MAX_OBJECT_TYPES = 80;
const MAX_QUERIES = 100;
const MAX_TOTAL_COLUMN_RECORDS = 240;
const MAX_DATA_SOURCE_RESPONSE_BYTES = 128 * 1024;
const MAX_PAYLOAD_FALLBACK_SOURCES = 20;
const MAX_MISSING_PROFILE_APP_IDS = 100;

/** Detect partial app inventories without trusting a backend's empty-list fallback. */
export function profileAppInventoryCoverage(
	profileAppIds: Iterable<string>,
	returnedAppIds: Iterable<string>,
) {
	const returned = new Set(returnedAppIds);
	const missing = [...profileAppIds]
		.filter((appId) => !returned.has(appId))
		.sort((left, right) => left.localeCompare(right));
	return {
		complete: missing.length === 0,
		missing_count: missing.length,
		missing_profile_app_ids: missing.slice(0, MAX_MISSING_PROFILE_APP_IDS),
		missing_ids_truncated: missing.length > MAX_MISSING_PROFILE_APP_IDS,
	};
}

function stringArg(args: Record<string, unknown>, key: string) {
	return typeof args[key] === "string" ? args[key].trim() : "";
}

function matchesSourceQuery(
	query: string,
	...values: Array<string | undefined>
) {
	if (!query) return true;
	return values.some((value) => value?.toLowerCase().includes(query));
}

function issue(
	severity: HomeToolIssue["severity"],
	code: string,
	path: string,
	message: string,
): HomeToolIssue {
	return { severity, code, path, message };
}

function objectRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function asJsonSource(value: unknown): string | undefined {
	try {
		return JSON.stringify(value);
	} catch {
		return undefined;
	}
}

function validateWidget(
	widget: IHomeWidget,
	index: number,
	issues: HomeToolIssue[],
) {
	const path = `$.widgets[${index}]`;
	if (!HOME_WIDGET_TYPES.has(widget.type)) {
		issues.push(
			issue(
				"warning",
				"unknown_widget_type",
				`${path}.type`,
				`Widget type '${widget.type}' is not available in this client and will render as unavailable.`,
			),
		);
		// A later client may own this widget's size, appearance, and config contracts. The
		// caller separately proves that an unsupported widget is preserved byte-for-byte.
		return;
	}
	const minimumRows = minimumHomeWidgetRows(widget);
	if (widget.size.rows < minimumRows) {
		issues.push(
			issue(
				"error",
				"widget_too_short",
				`${path}.size.rows`,
				`This widget needs at least ${minimumRows} rows.`,
			),
		);
	}
	const legacyVariants = LEGACY_VARIANTS_BY_WIDGET_TYPE.get(widget.type);
	if (
		!HOME_VARIANTS.has(widget.appearance.variant) &&
		!legacyVariants?.has(widget.appearance.variant)
	) {
		issues.push(
			issue(
				"error",
				"unknown_widget_variant",
				`${path}.appearance.variant`,
				"Use card, borderless, tinted, or solid.",
			),
		);
	} else if (
		widget.appearance.variant === "solid" &&
		!SOLID_WIDGET_TYPES.has(widget.type)
	) {
		issues.push(
			issue(
				"error",
				"unsupported_solid_variant",
				`${path}.appearance.variant`,
				"The solid surface is only supported by spotlight, ranking, and collection feature widgets.",
			),
		);
	}
	if (!Object.hasOwn(HOME_ACCENTS, widget.appearance.accent)) {
		issues.push(
			issue(
				"error",
				"unknown_widget_accent",
				`${path}.appearance.accent`,
				`Use one of: ${Object.keys(HOME_ACCENTS).join(", ")}.`,
			),
		);
	}
	issues.push(
		...validateKnownHomeWidgetConfig(
			widget.type,
			widget.config,
			`${path}.config`,
		),
	);
}

/** Validate and canonicalize an untrusted Home JSON value without reading external resources. */
export function validateHomeLayoutCandidate(
	value: unknown,
): HomeLayoutValidationResult {
	const source = asJsonSource(value);
	if (source === undefined) {
		return {
			status: "validation_error",
			valid: false,
			issues: [
				issue(
					"error",
					"home_layout_not_json",
					"$",
					"The layout must be a JSON-serializable object.",
				),
			],
		};
	}
	const parsed = parseHomeLayoutJson(source);
	if (!parsed.ok) {
		return {
			status: "validation_error",
			valid: false,
			issues: [issue("error", "home_layout_invalid", "$", parsed.error)],
		};
	}
	const issues: HomeToolIssue[] = [];
	if (stableStringify(value) !== stableStringify(parsed.layout)) {
		issues.push(
			issue(
				"warning",
				"home_layout_normalized",
				"$",
				"Optional widget fields and bounded sizes were normalized in canonical_layout.",
			),
		);
	}
	if (parsed.layout.widgets.length === 0) {
		issues.push(
			issue(
				"warning",
				"home_layout_empty",
				"$.widgets",
				"The Home layout has no widgets.",
			),
		);
	}
	parsed.layout.widgets.forEach((widget, index) =>
		validateWidget(widget, index, issues),
	);
	const valid = !issues.some((entry) => entry.severity === "error");
	return {
		status: valid ? "ok" : "validation_error",
		valid,
		issues,
		layout: parsed.layout,
		canonical_layout: parsed.layout,
		fingerprint: homeLayoutFingerprint(parsed.layout),
		byte_count: homeLayoutByteLength(parsed.layout),
	};
}

/** Return compact creation templates for the exact widget implementations in this client. */
export function getHomeWidgetCatalog(args: Record<string, unknown>) {
	const category = stringArg(args, "category").toLowerCase();
	const type = stringArg(args, "type").toLowerCase();
	const query = stringArg(args, "query").toLowerCase();
	if (category && !HOME_CATEGORIES.has(category as never)) {
		return {
			status: "error",
			code: "home_widget_category_invalid",
			message: `Unknown category '${category}'.`,
			categories: [...HOME_CATEGORIES],
		};
	}
	const matches = HOME_WIDGET_PRESETS.filter((preset) => {
		if (category && preset.category !== category) return false;
		if (type && preset.type.toLowerCase() !== type) return false;
		if (!query) return true;
		return [preset.id, preset.name, preset.description, preset.type]
			.join(" ")
			.toLowerCase()
			.includes(query);
	});
	const matchedTypes = [...new Set(matches.map((preset) => preset.type))];
	return {
		status: "ok",
		total: matches.length,
		widget_type_total: matchedTypes.length,
		widget_types: Object.fromEntries(
			matchedTypes.map((widgetType) => [
				widgetType,
				{
					type: widgetType,
					...structuredClone(
						HOME_WIDGET_CONFIG_CONTRACTS[
							widgetType as keyof typeof HOME_WIDGET_CONFIG_CONTRACTS
						],
					),
				},
			]),
		),
		layout_contract: {
			version: 1,
			max_widgets: MAX_HOME_WIDGETS,
			max_bytes: MAX_HOME_LAYOUT_BYTES,
			grid_columns: {
				mobile: responsiveHomeColumns(0),
				tablet: responsiveHomeColumns(600),
				desktop: responsiveHomeColumns(1050),
			},
			breakpoints: {
				mobile_max_exclusive: 600,
				desktop_min: 1050,
			},
			height_modes: ["auto", "content", "fixed"],
		},
		categories: [...HOME_CATEGORIES],
		accents: Object.keys(HOME_ACCENTS),
		surface_variants: [...HOME_VARIANTS],
		visualizations: HOME_DATA_VISUALIZATIONS.map(([id, name]) => ({
			id,
			name,
		})),
		data_options: {
			source_kinds: DATA_SOURCE_KINDS,
			scopes: [...HOME_DATA_SCOPES],
			modes: [...HOME_DATA_MODES],
			aggregations: [...HOME_DATA_AGGREGATIONS],
			filter_operators: [...HOME_DATA_FILTER_OPERATORS],
			time_buckets: [...HOME_DATA_TIME_BUCKETS],
			date_ranges: [...HOME_DATA_DATE_RANGES],
			sort_directions: [...HOME_DATA_SORT_DIRECTIONS],
			formats: [...HOME_DATA_FORMATS],
			filter_value_types: [...HOME_DATA_VALUE_TYPES],
		},
		renderings: {
			apps: HOME_APP_RENDERINGS.map(([id, name]) => ({ id, name })),
			models: HOME_MODEL_RENDERINGS.map(([id, name]) => ({ id, name })),
			packages: HOME_PACKAGE_RENDERINGS.map(([id, name]) => ({ id, name })),
			quick_links: ["grid", "list"],
		},
		presets: matches.map((preset) => ({
			preset_id: preset.id,
			name: preset.name,
			description: preset.description,
			category: preset.category,
			widget: {
				type: preset.type,
				title: preset.name,
				description: "",
				size: { columns: preset.columns, rows: preset.rows },
				appearance: {
					variant: preset.variant ?? "card",
					accent: preset.accent ?? "neutral",
				},
				config: structuredClone(preset.config),
			},
		})),
		note: "Give every widget a unique id when placing it in a layout.",
	};
}

function parseDataSourceRequest(args: Record<string, unknown>):
	| {
			ok: true;
			appId: string;
			scope: HomeDataScope;
			kinds: HomeDataSourceKind[];
			query: string;
			sourceId: string;
			objectTypeQuery: string;
			columnQuery: string;
	  }
	| { ok: false; result: Record<string, unknown> } {
	const appId = stringArg(args, "app_id") || stringArg(args, "appId");
	if (!appId) {
		return {
			ok: false,
			result: {
				status: "error",
				code: "home_data_app_required",
				message: "list_home_data_sources requires app_id.",
			},
		};
	}
	const requestedScope = stringArg(args, "scope") || "project";
	if (requestedScope !== "project" && requestedScope !== "personal") {
		return {
			ok: false,
			result: {
				status: "error",
				code: "home_data_scope_invalid",
				message: "scope must be project or personal.",
			},
		};
	}
	const rawKinds = args.kinds;
	if (
		rawKinds !== undefined &&
		(!Array.isArray(rawKinds) ||
			rawKinds.some(
				(kind) =>
					typeof kind !== "string" ||
					!DATA_SOURCE_KINDS.includes(kind as HomeDataSourceKind),
			))
	) {
		return {
			ok: false,
			result: {
				status: "error",
				code: "home_data_kinds_invalid",
				message: "kinds may contain table, ontology, and query.",
			},
		};
	}
	return {
		ok: true,
		appId,
		scope: requestedScope,
		query: stringArg(args, "query").toLowerCase(),
		sourceId: stringArg(args, "source_id") || stringArg(args, "sourceId"),
		objectTypeQuery: (
			stringArg(args, "object_type_query") || stringArg(args, "objectTypeQuery")
		).toLowerCase(),
		columnQuery: (
			stringArg(args, "column_query") || stringArg(args, "columnQuery")
		).toLowerCase(),
		kinds:
			Array.isArray(rawKinds) && rawKinds.length
				? ([...new Set(rawKinds)] as HomeDataSourceKind[])
				: DATA_SOURCE_KINDS,
	};
}

/** List only metadata needed to bind a Home data widget to an authorized profile app. */
export async function listHomeDataSources(
	backend: Pick<IBackendState, "dbState" | "graphState" | "queryState">,
	args: Record<string, unknown>,
	isVisibleInProfile: (appId: string) => Promise<boolean>,
): Promise<Record<string, unknown>> {
	const parsed = parseDataSourceRequest(args);
	if (!parsed.ok) return parsed.result;
	const { appId, scope, kinds, query, sourceId, objectTypeQuery, columnQuery } =
		parsed;
	if (!(await isVisibleInProfile(appId))) {
		return {
			status: "error",
			code: "home_data_app_not_in_profile",
			message: `App '${appId}' is not available in the current profile.`,
		};
	}
	const personal = scope === "personal";
	const sources: {
		tables: unknown[];
		ontologies: unknown[];
		queries: unknown[];
	} = { tables: [], ontologies: [], queries: [] };
	const matchedTotals = { tables: 0, ontologies: 0, queries: 0 };
	const unreadable = new Set<HomeDataSourceKind>();
	let truncated = false;
	let detailsOmitted = false;
	let emittedColumnRecords = 0;
	const selectColumns = <T>(columns: T[]) => {
		const remaining = Math.max(
			0,
			MAX_TOTAL_COLUMN_RECORDS - emittedColumnRecords,
		);
		const selected = columns.slice(0, Math.min(MAX_COLUMNS, remaining));
		emittedColumnRecords += selected.length;
		const columnsTruncated = selected.length < columns.length;
		truncated ||= columnsTruncated;
		detailsOmitted ||= columnsTruncated;
		return { selected, columnsTruncated };
	};

	await Promise.all(
		kinds.map(async (kind) => {
			try {
				if (kind === "table") {
					const names = (
						personal
							? await backend.dbState.listTablesUser(appId)
							: await backend.dbState.listTables(appId)
					)
						.filter((name) =>
							sourceId ? name === sourceId : matchesSourceQuery(query, name),
						)
						.sort((left, right) => left.localeCompare(right));
					matchedTotals.tables = names.length;
					truncated ||= names.length > MAX_TABLES;
					const returnedNames = names.slice(0, MAX_TABLES);
					if (!sourceId) {
						detailsOmitted ||= returnedNames.length > 0;
						sources.tables = returnedNames.map((name) => ({
							name,
							columns_omitted: true,
						}));
					} else {
						sources.tables = await Promise.all(
							returnedNames.map(async (name) => {
								try {
									const schema = await backend.dbState.getSchema(
										appId,
										name,
										personal,
									);
									const columns = homeDataColumns(schema).filter((column) =>
										matchesSourceQuery(
											columnQuery,
											column.name,
											column.type_name,
										),
									);
									const { selected, columnsTruncated } = selectColumns(columns);
									return {
										name,
										columns: selected,
										columns_matched_total: columns.length,
										columns_truncated: columnsTruncated,
									};
								} catch {
									unreadable.add("table");
									return { name, columns: [], schema_unreadable: true };
								}
							}),
						);
					}
				} else if (kind === "ontology") {
					const overlays = (
						await backend.graphState.listOverlays(appId, personal)
					)
						.slice()
						.filter((overlay) =>
							sourceId
								? overlay.id === sourceId
								: matchesSourceQuery(query, overlay.id, overlay.name),
						)
						.sort((left, right) => left.name.localeCompare(right.name));
					matchedTotals.ontologies = overlays.length;
					truncated ||= overlays.length > MAX_ONTOLOGIES;
					sources.ontologies = overlays
						.slice(0, MAX_ONTOLOGIES)
						.map((overlay) => {
							if (!sourceId) {
								detailsOmitted = true;
								return {
									ontology_id: overlay.id,
									name: overlay.name,
									description: overlay.description,
									object_type_count: overlay.nodes.length,
									object_types_omitted: true,
								};
							}
							const objectTypes = overlay.nodes.filter((node) =>
								matchesSourceQuery(objectTypeQuery, node.label, node.table),
							);
							const objectTypesTruncated =
								objectTypes.length > MAX_OBJECT_TYPES;
							const includeColumns =
								Boolean(objectTypeQuery) && objectTypes.length <= 12;
							detailsOmitted ||= !includeColumns && objectTypes.length > 0;
							truncated ||= objectTypesTruncated;
							return {
								ontology_id: overlay.id,
								name: overlay.name,
								description: overlay.description,
								object_types: objectTypes
									.slice(0, MAX_OBJECT_TYPES)
									.map((node) => {
										if (!includeColumns) {
											return {
												name: node.label,
												table: node.table,
												id_column: node.id_column,
												display_column: node.display_column,
												column_count: homeOntologyColumns(overlay, node.label)
													.length,
												columns_omitted: true,
											};
										}
										const columns = homeOntologyColumns(
											overlay,
											node.label,
										).filter((column) =>
											matchesSourceQuery(
												columnQuery,
												column.name,
												column.type_name,
											),
										);
										const { selected, columnsTruncated } =
											selectColumns(columns);
										return {
											name: node.label,
											table: node.table,
											id_column: node.id_column,
											display_column: node.display_column,
											columns: selected,
											columns_matched_total: columns.length,
											columns_truncated: columnsTruncated,
										};
									}),
								object_types_matched_total: objectTypes.length,
								object_types_truncated: objectTypesTruncated,
							};
						});
				} else {
					const queries = (
						await backend.queryState.listSavedQueries(appId, personal)
					)
						.slice()
						.filter((savedQuery) =>
							sourceId
								? savedQuery.id === sourceId
								: matchesSourceQuery(query, savedQuery.id, savedQuery.name),
						)
						.sort((left, right) => left.name.localeCompare(right.name));
					matchedTotals.queries = queries.length;
					truncated ||= queries.length > MAX_QUERIES;
					const returnedQueries = queries.slice(0, MAX_QUERIES);
					detailsOmitted ||= !sourceId && returnedQueries.length > 0;
					sources.queries = returnedQueries.map((query) => {
						const parameters = extractHomeQueryParameters(query.sql);
						return {
							query_id: query.id,
							name: query.name,
							description: query.description,
							kind: query.kind,
							surface: query.surface,
							overlay_id: query.overlay_id,
							result_schema_available: false,
							...(sourceId
								? {
										parameters,
										param_schema: query.param_schema,
										viz_config: query.viz_config,
										default_limit: query.default_limit,
									}
								: {
										parameter_count: parameters.length,
										details_omitted: true,
									}),
						};
					});
				}
			} catch {
				unreadable.add(kind);
			}
		}),
	);

	const response = {
		status: unreadable.size || truncated ? "partial" : "ok",
		app_id: appId,
		scope,
		query: query || undefined,
		source_id: sourceId || undefined,
		object_type_query: objectTypeQuery || undefined,
		column_query: columnQuery || undefined,
		matched_totals: matchedTotals,
		column_records_returned: emittedColumnRecords,
		column_record_limit: MAX_TOTAL_COLUMN_RECORDS,
		detail_level: sourceId ? "selected_source" : "summaries",
		details_omitted: detailsOmitted,
		...(detailsOmitted
			? {
					detail_hint:
						"Call again with source_id for source details. Narrow object_type_query and column_query when object types or columns are omitted or truncated.",
				}
			: {}),
		complete: unreadable.size === 0 && !truncated,
		truncated,
		unreadable: [...unreadable].sort(),
		sources,
	};
	const boundedResponse = {
		...response,
		response_byte_limit: MAX_DATA_SOURCE_RESPONSE_BYTES,
	};
	if (
		new TextEncoder().encode(JSON.stringify(boundedResponse)).byteLength <=
		MAX_DATA_SOURCE_RESPONSE_BYTES
	) {
		return boundedResponse;
	}

	const text = (value: unknown) =>
		typeof value === "string" ? value.slice(0, 512) : "";
	const records = (value: unknown) =>
		Array.isArray(value)
			? (value as Array<Record<string, unknown>>).slice(
					0,
					MAX_PAYLOAD_FALLBACK_SOURCES,
				)
			: [];
	return {
		status: "partial",
		app_id: text(appId),
		scope,
		query: text(query) || undefined,
		source_id: text(sourceId) || undefined,
		object_type_query: text(objectTypeQuery) || undefined,
		column_query: text(columnQuery) || undefined,
		matched_totals: matchedTotals,
		complete: false,
		truncated: true,
		payload_limited: true,
		response_byte_limit: MAX_DATA_SOURCE_RESPONSE_BYTES,
		details_omitted: true,
		detail_hint:
			"The bounded response could not include these details. Retry with source_id, object_type_query, and column_query narrowed to the exact target.",
		unreadable: [...unreadable].sort(),
		sources: {
			tables: records(sources.tables).map((entry) => ({
				name: text(entry.name),
				columns_omitted: true,
			})),
			ontologies: records(sources.ontologies).map((entry) => ({
				ontology_id: text(entry.ontology_id),
				name: text(entry.name),
				object_types_omitted: true,
			})),
			queries: records(sources.queries).map((entry) => ({
				query_id: text(entry.query_id),
				name: text(entry.name),
				details_omitted: true,
			})),
		},
	};
}

function homeContractConditionMatches(
	condition: HomeWidgetConfigCondition | undefined,
	config: Record<string, unknown>,
	fields: Readonly<Record<string, HomeWidgetConfigFieldContract>>,
): boolean {
	if (!condition) return true;
	if (
		condition.all &&
		!condition.all.every((entry) =>
			homeContractConditionMatches(entry, config, fields),
		)
	)
		return false;
	if (!condition.field) return true;
	const actual = config[condition.field] ?? fields[condition.field]?.default;
	if (condition.equals !== undefined) return actual === condition.equals;
	if (condition.in) return condition.in.includes(actual as never);
	if (condition.not_equals !== undefined)
		return actual !== condition.not_equals;
	if (condition.not_in) return !condition.not_in.includes(actual as never);
	return true;
}

function referencedAppPaths(layout: IHomeLayout) {
	const references = new Map<string, string[]>();
	const add = (appId: string, path: string) => {
		if (!appId) return;
		const paths = references.get(appId) ?? [];
		paths.push(path);
		references.set(appId, paths);
	};
	layout.widgets.forEach((widget, index) => {
		const path = `$.widgets[${index}].config`;
		if (!Object.hasOwn(HOME_WIDGET_CONFIG_CONTRACTS, widget.type)) return;
		const contract =
			HOME_WIDGET_CONFIG_CONTRACTS[
				widget.type as keyof typeof HOME_WIDGET_CONFIG_CONTRACTS
			];
		for (const [fieldName, field] of Object.entries(contract.fields)) {
			const reference = field.reference;
			if (
				!reference ||
				!homeContractConditionMatches(
					reference.when,
					widget.config,
					contract.fields,
				)
			)
				continue;
			if (reference.kind === "profile_app_id") {
				add(stringArg(widget.config, fieldName), `${path}.${fieldName}`);
			} else if (
				reference.kind === "profile_app_ids" &&
				Array.isArray(widget.config[fieldName])
			) {
				widget.config[fieldName].forEach((appId, appIndex) => {
					if (typeof appId === "string")
						add(appId.trim(), `${path}.${fieldName}[${appIndex}]`);
				});
			}
		}
	});
	return references;
}

function dataFieldReferences(widget: IHomeWidget) {
	const config = normalizeHomeDataConfig(widget.config);
	const fields: Array<[string, string]> = [];
	const add = (field: string, suffix: string) => {
		if (field && field !== "value" && field !== "group") {
			fields.push([field, suffix]);
		}
	};
	if (config.dateRange !== "all") add(config.dateField, "dateField");
	if (config.mode === "records" && config.visualization !== "histogram") {
		add(config.groupBy, "groupBy");
		add(config.xField, "xField");
		add(config.yField, "yField");
		add(config.sortBy, "sortBy");
		config.fields.forEach((field, index) => add(field, `fields[${index}]`));
	} else {
		add(config.groupBy, "groupBy");
		add(config.seriesBy, "seriesBy");
		if (config.visualization === "boxplot") add(config.yField, "yField");
		else {
			config.measures.forEach((measure, index) => {
				if (measure.aggregation !== "count")
					add(measure.field, `measures[${index}].field`);
			});
		}
	}
	config.filters.forEach((filter, index) =>
		add(filter.field, `filters[${index}].field`),
	);
	return fields;
}

export interface HomeReferenceValidationOptions {
	profileAppIds: Set<string>;
}

/** Validate app, event, table, ontology, and saved-query identities used by a canonical layout. */
export async function validateHomeLayoutReferences(
	backend: Pick<
		IBackendState,
		"dbState" | "eventState" | "graphState" | "queryState"
	>,
	layout: IHomeLayout,
	options: HomeReferenceValidationOptions,
): Promise<HomeToolIssue[]> {
	const issues: HomeToolIssue[] = [];
	const appPaths = referencedAppPaths(layout);
	for (const [appId, paths] of appPaths) {
		if (options.profileAppIds.has(appId)) continue;
		for (const path of paths) {
			issues.push(
				issue(
					"error",
					"home_app_not_in_profile",
					path,
					`App '${appId}' is not available in the current profile.`,
				),
			);
		}
	}

	const eventsByApp = new Map<
		string,
		Promise<Awaited<ReturnType<IBackendState["eventState"]["getEvents"]>>>
	>();
	const tablesByScope = new Map<string, Promise<string[]>>();
	const ontologiesByScope = new Map<string, Promise<GraphOverlay[]>>();
	const queriesByScope = new Map<string, Promise<SavedQuery[]>>();
	const getEvents = (appId: string) => {
		let pending = eventsByApp.get(appId);
		if (!pending) {
			pending = backend.eventState.getEvents(appId);
			eventsByApp.set(appId, pending);
		}
		return pending;
	};

	for (const [index, widget] of layout.widgets.entries()) {
		const path = `$.widgets[${index}].config`;
		const appId = stringArg(widget.config, "appId");
		if (
			widget.type === "app-embed" &&
			appId &&
			options.profileAppIds.has(appId)
		) {
			try {
				const events = await getEvents(appId);
				const target = stringArg(widget.config, "target") || "landing";
				if (target === "event") {
					const eventId = stringArg(widget.config, "eventId");
					const event = events.find((candidate) => candidate.id === eventId);
					if (!event) {
						issues.push(
							issue(
								"error",
								"home_app_event_missing",
								`${path}.eventId`,
								`Event '${eventId}' is not available in app '${appId}'.`,
							),
						);
					} else {
						if (
							!event.default_page_id?.trim() &&
							!["simple_chat", "generic_form", "quick_action"].includes(
								event.event_type,
							)
						) {
							issues.push(
								issue(
									"error",
									"home_app_event_not_embeddable",
									`${path}.eventId`,
									`Event '${eventId}' does not expose a Home-compatible page, chat, form, or quick action.`,
								),
							);
						}
						if (!event.active) {
							issues.push(
								issue(
									"error",
									"home_app_event_inactive",
									`${path}.eventId`,
									`Event '${eventId}' is inactive.`,
								),
							);
						}
					}
				} else if (target === "route") {
					const route = parseHomeEmbedTarget(widget.config).routePath;
					const eventRoute = (value: string | null | undefined) => {
						const path = (value ?? "").split("?", 1)[0];
						return path ? `/${path.replace(/^\/+/, "")}` : "";
					};
					if (
						route !== "/" &&
						!events.some(
							(event) => event.active && eventRoute(event.route) === route,
						)
					) {
						issues.push(
							issue(
								"error",
								"home_app_route_missing",
								`${path}.route`,
								`Route '${route}' was not found in the app's active interface metadata.`,
							),
						);
					}
				}
			} catch {
				issues.push(
					issue(
						"warning",
						"home_app_interfaces_unreadable",
						path,
						`App '${appId}' interface metadata could not be checked.`,
					),
				);
			}
		}

		if (widget.type !== "data" || !appId || !options.profileAppIds.has(appId)) {
			continue;
		}
		const config = normalizeHomeDataConfig(widget.config);
		const personal = config.scope === "personal";
		const cacheKey = `${appId}:${config.scope}`;
		try {
			let knownColumns: Set<string> | undefined;
			if (config.sourceKind === "table") {
				let pending = tablesByScope.get(cacheKey);
				if (!pending) {
					pending = personal
						? backend.dbState.listTablesUser(appId)
						: backend.dbState.listTables(appId);
					tablesByScope.set(cacheKey, pending);
				}
				const tables = await pending;
				if (!tables.includes(config.table)) {
					issues.push(
						issue(
							"error",
							"home_table_missing",
							`${path}.table`,
							`Table '${config.table}' is not available in ${config.scope} data.`,
						),
					);
					continue;
				}
				const schema = await backend.dbState.getSchema(
					appId,
					config.table,
					personal,
				);
				knownColumns = new Set(
					homeDataColumns(schema).map((column) => column.name),
				);
			} else if (config.sourceKind === "ontology") {
				let pending = ontologiesByScope.get(cacheKey);
				if (!pending) {
					pending = backend.graphState.listOverlays(appId, personal);
					ontologiesByScope.set(cacheKey, pending);
				}
				const overlays = await pending;
				const overlay = overlays.find(
					(entry) => entry.id === config.ontologyId,
				);
				if (!overlay) {
					issues.push(
						issue(
							"error",
							"home_ontology_missing",
							`${path}.ontologyId`,
							`Ontology '${config.ontologyId}' is not available in ${config.scope} data.`,
						),
					);
					continue;
				}
				const objectType = overlay.nodes.find(
					(node) => node.label === config.objectType,
				);
				if (!objectType) {
					issues.push(
						issue(
							"error",
							"home_ontology_object_missing",
							`${path}.objectType`,
							`Object type '${config.objectType}' is not part of ontology '${config.ontologyId}'.`,
						),
					);
					continue;
				}
				knownColumns = new Set(
					homeOntologyColumns(overlay, config.objectType).map(
						(column) => column.name,
					),
				);
			} else {
				let pending = queriesByScope.get(cacheKey);
				if (!pending) {
					pending = backend.queryState.listSavedQueries(appId, personal);
					queriesByScope.set(cacheKey, pending);
				}
				const queries = await pending;
				const query = queries.find((entry) => entry.id === config.queryId);
				if (!query) {
					issues.push(
						issue(
							"error",
							"home_query_missing",
							`${path}.queryId`,
							`Saved query '${config.queryId}' is not available in ${config.scope} data.`,
						),
					);
					continue;
				}
				const schemaProperties = objectRecord(query.param_schema?.properties)
					? query.param_schema.properties
					: undefined;
				for (const parameter of extractHomeQueryParameters(query.sql)) {
					if (parameter.startsWith("__home_")) {
						issues.push(
							issue(
								"error",
								"home_query_parameter_reserved",
								`${path}.queryId`,
								`Saved query '${query.name}' uses reserved parameter '${parameter}'.`,
							),
						);
						continue;
					}
					if (!Object.hasOwn(config.queryParams, parameter)) {
						issues.push(
							issue(
								"error",
								"home_query_parameter_missing",
								`${path}.queryParams.${parameter}`,
								`Saved query '${query.name}' requires parameter '${parameter}'.`,
							),
						);
						continue;
					}
					const value = config.queryParams[parameter];
					if (value === "$viewer.id") continue;
					const parameterSchema = objectRecord(schemaProperties?.[parameter])
						? schemaProperties[parameter]
						: undefined;
					const expectedType = parameterSchema?.type;
					const matchesType =
						expectedType === "string"
							? typeof value === "string"
							: expectedType === "number"
								? typeof value === "number" && Number.isFinite(value)
								: expectedType === "integer"
									? typeof value === "number" &&
										Number.isFinite(value) &&
										Number.isInteger(value)
									: expectedType === "boolean"
										? typeof value === "boolean"
										: undefined;
					if (matchesType === false) {
						issues.push(
							issue(
								"error",
								"home_query_parameter_type_invalid",
								`${path}.queryParams.${parameter}`,
								`Parameter '${parameter}' must be a ${expectedType}.`,
							),
						);
					} else if (matchesType === undefined) {
						issues.push(
							issue(
								"warning",
								"home_query_parameter_schema_unavailable",
								`${path}.queryParams.${parameter}`,
								`Parameter '${parameter}' has no supported type metadata, so its fixed value could not be type-checked.`,
							),
						);
					}
				}
				if (dataFieldReferences(widget).length > 0) {
					issues.push(
						issue(
							"warning",
							"home_query_result_fields_unverified",
							path,
							"Saved-query result fields are not exposed by its metadata and could not be verified.",
						),
					);
				}
			}
			if (knownColumns?.size) {
				for (const [field, suffix] of dataFieldReferences(widget)) {
					if (knownColumns.has(field)) continue;
					issues.push(
						issue(
							"error",
							"home_data_field_missing",
							`${path}.${suffix}`,
							`Field '${field}' is not available in the selected source.`,
						),
					);
				}
			}
		} catch {
			issues.push(
				issue(
					"warning",
					"home_data_source_unreadable",
					path,
					"This data source could not be checked. Its identifiers were preserved.",
				),
			);
		}
	}
	return issues;
}

/** Merge asynchronous reference diagnostics into a local validation result. */
export function withHomeReferenceIssues(
	validation: HomeLayoutValidationResult,
	referenceIssues: HomeToolIssue[],
): HomeLayoutValidationResult {
	const issues = [...validation.issues, ...referenceIssues];
	const valid =
		Boolean(validation.layout) &&
		!issues.some((entry) => entry.severity === "error");
	return {
		...validation,
		status: valid ? "ok" : "validation_error",
		valid,
		issues,
	};
}

interface UnadvertisedConfigEntry {
	path: string;
	value: unknown;
}

function collectUnadvertisedConfig(
	value: Record<string, unknown>,
	contract: HomeWidgetObjectContract,
	logicalPath: string,
	actualPath: string,
	entries: Map<string, UnadvertisedConfigEntry>,
) {
	for (const [key, nested] of Object.entries(value)) {
		const field = contract.fields[key];
		if (!field) {
			entries.set(`${logicalPath}.${key}`, {
				path: `${actualPath}.${key}`,
				value: nested,
			});
			continue;
		}
		if (field.type !== "object_list" || !field.item || !Array.isArray(nested))
			continue;
		for (const [index, item] of nested.entries()) {
			if (!item || typeof item !== "object" || Array.isArray(item)) continue;
			const identityKey = field.item.unique_by;
			const identity =
				identityKey && Object.hasOwn(item, identityKey)
					? (item as Record<string, unknown>)[identityKey]
					: undefined;
			const logicalItem =
				identityKey && identity !== undefined && identity !== ""
					? `${logicalPath}.${key}[${identityKey}=${stableStringify(identity)}]`
					: `${logicalPath}.${key}[${index}]`;
			collectUnadvertisedConfig(
				item as Record<string, unknown>,
				field.item,
				logicalItem,
				`${actualPath}.${key}[${index}]`,
				entries,
			);
		}
	}
}

function unadvertisedKnownWidgetConfig(
	widget: IHomeWidget,
	index: number,
): Map<string, UnadvertisedConfigEntry> | undefined {
	if (!Object.hasOwn(HOME_WIDGET_CONFIG_CONTRACTS, widget.type))
		return undefined;
	const contract =
		HOME_WIDGET_CONFIG_CONTRACTS[
			widget.type as keyof typeof HOME_WIDGET_CONFIG_CONTRACTS
		];
	const entries = new Map<string, UnadvertisedConfigEntry>();
	collectUnadvertisedConfig(
		widget.config,
		contract,
		"config",
		`$.widgets[${index}].config`,
		entries,
	);
	return entries;
}

function unadvertisedConfigEqual(
	left: Map<string, UnadvertisedConfigEntry> | undefined,
	right: Map<string, UnadvertisedConfigEntry> | undefined,
) {
	if (!left || !right || left.size !== right.size) return false;
	for (const [path, entry] of left) {
		const other = right.get(path);
		if (!other || stableStringify(entry.value) !== stableStringify(other.value))
			return false;
	}
	return true;
}

/** Preserve config fields this client cannot interpret on otherwise known widgets. */
export function validateUnknownHomeWidgetConfigPreservation(
	candidate: IHomeLayout,
	current: IHomeLayout,
): HomeToolIssue[] {
	const issues: HomeToolIssue[] = [];
	const currentById = new Map(
		current.widgets.map((widget, index) => [widget.id, { widget, index }]),
	);
	const candidateById = new Map(
		candidate.widgets.map((widget, index) => [widget.id, { widget, index }]),
	);
	const checked = new Set<string>();

	for (const [currentIndex, currentWidget] of current.widgets.entries()) {
		const currentUnknown = unadvertisedKnownWidgetConfig(
			currentWidget,
			currentIndex,
		);
		if (!currentUnknown?.size) continue;
		checked.add(currentWidget.id);
		const candidateEntry = candidateById.get(currentWidget.id);
		if (!candidateEntry || candidateEntry.widget.type !== currentWidget.type) {
			issues.push(
				issue(
					"error",
					"unknown_widget_config_removed",
					"$.widgets",
					`Widget '${currentWidget.id}' contains unadvertised config that must remain unchanged because this client cannot validate it.`,
				),
			);
			continue;
		}
		const candidateUnknown = unadvertisedKnownWidgetConfig(
			candidateEntry.widget,
			candidateEntry.index,
		);
		if (unadvertisedConfigEqual(currentUnknown, candidateUnknown)) continue;
		const changedEntry = [...(candidateUnknown?.entries() ?? [])].find(
			([path, entry]) => {
				const previous = currentUnknown.get(path);
				return (
					!previous ||
					stableStringify(previous.value) !== stableStringify(entry.value)
				);
			},
		)?.[1];
		issues.push(
			issue(
				"error",
				"unknown_widget_config_changed",
				changedEntry?.path ?? `$.widgets[${candidateEntry.index}].config`,
				`Unadvertised config on widget '${currentWidget.id}' must remain unchanged because this client cannot validate it.`,
			),
		);
	}

	for (const [candidateIndex, candidateWidget] of candidate.widgets.entries()) {
		if (checked.has(candidateWidget.id)) continue;
		const candidateUnknown = unadvertisedKnownWidgetConfig(
			candidateWidget,
			candidateIndex,
		);
		if (!candidateUnknown?.size) continue;
		const currentEntry = currentById.get(candidateWidget.id);
		const currentUnknown = currentEntry
			? unadvertisedKnownWidgetConfig(currentEntry.widget, currentEntry.index)
			: undefined;
		if (
			currentEntry?.widget.type === candidateWidget.type &&
			unadvertisedConfigEqual(currentUnknown, candidateUnknown)
		)
			continue;
		const first = candidateUnknown.values().next().value;
		issues.push(
			issue(
				"error",
				"unknown_widget_config_introduced",
				first?.path ?? `$.widgets[${candidateIndex}].config`,
				`Widget '${candidateWidget.id}' introduces config fields that this client cannot validate.`,
			),
		);
	}
	return issues;
}

/**
 * Forward-compatible widgets may survive an edit, but this client cannot safely author their
 * payload. Match them by id so array reordering remains allowed while every widget value stays
 * unchanged.
 */
export function validateUnknownHomeWidgetPreservation(
	candidate: IHomeLayout,
	current: IHomeLayout,
): HomeToolIssue[] {
	const issues: HomeToolIssue[] = [];
	const currentUnknown = new Map(
		current.widgets
			.filter((widget) => !HOME_WIDGET_TYPES.has(widget.type))
			.map((widget) => [widget.id, widget]),
	);
	const candidateUnknown = new Map(
		candidate.widgets
			.filter((widget) => !HOME_WIDGET_TYPES.has(widget.type))
			.map((widget) => [widget.id, widget]),
	);
	for (const [id, widget] of candidateUnknown) {
		const existing = currentUnknown.get(id);
		if (existing && stableStringify(existing) === stableStringify(widget))
			continue;
		const index = candidate.widgets.findIndex((entry) => entry.id === id);
		issues.push(
			issue(
				"error",
				"unknown_widget_changed",
				`$.widgets[${index}]`,
				existing
					? `Unknown widget '${id}' must remain unchanged because this client cannot validate its config.`
					: `Unknown widget type '${widget.type}' cannot be introduced by FlowPilot.`,
			),
		);
	}
	for (const [id] of currentUnknown) {
		if (candidateUnknown.has(id)) continue;
		issues.push(
			issue(
				"error",
				"unknown_widget_removed",
				"$.widgets",
				`Unknown widget '${id}' must be preserved because this client cannot validate it.`,
			),
		);
	}
	return issues;
}
