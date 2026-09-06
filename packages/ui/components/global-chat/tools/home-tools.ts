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
	safeHomeHref,
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
const MAX_TABLES = 60;
const MAX_COLUMNS = 80;
const MAX_ONTOLOGIES = 60;
const MAX_OBJECT_TYPES = 80;
const MAX_QUERIES = 100;

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

function asJsonSource(value: unknown): string | undefined {
	try {
		return JSON.stringify(value);
	} catch {
		return undefined;
	}
}

function validateSafeLinks(
	value: unknown,
	path: string,
	issues: HomeToolIssue[],
	depth = 0,
) {
	if (depth > 12 || !value || typeof value !== "object") return;
	if (Array.isArray(value)) {
		value.forEach((item, index) =>
			validateSafeLinks(item, `${path}[${index}]`, issues, depth + 1),
		);
		return;
	}
	for (const [key, nested] of Object.entries(
		value as Record<string, unknown>,
	)) {
		const nestedPath = `${path}.${key}`;
		if (
			typeof nested === "string" &&
			nested.trim() &&
			(key === "href" || key.endsWith("Href") || key === "imageUrl") &&
			!safeHomeHref(nested)
		) {
			issues.push(
				issue(
					"error",
					"unsafe_home_url",
					nestedPath,
					"Use a relative path or an http, https, mailto, or tel URL.",
				),
			);
		}
		validateSafeLinks(nested, nestedPath, issues, depth + 1);
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
	if (widget.type === "app-embed") {
		const appId = stringArg(widget.config, "appId");
		const target = stringArg(widget.config, "target") || "landing";
		if (!appId) {
			issues.push(
				issue(
					"error",
					"app_embed_missing_app",
					`${path}.config.appId`,
					"Choose an app from the current profile.",
				),
			);
		}
		if (!["landing", "route", "event"].includes(target)) {
			issues.push(
				issue(
					"error",
					"app_embed_target_invalid",
					`${path}.config.target`,
					"Use landing, route, or event.",
				),
			);
		}
		if (target === "event" && !stringArg(widget.config, "eventId")) {
			issues.push(
				issue(
					"error",
					"app_embed_missing_event",
					`${path}.config.eventId`,
					"Choose an app interface event.",
				),
			);
		}
		if (
			target === "route" &&
			stringArg(widget.config, "route") &&
			!stringArg(widget.config, "route").startsWith("/")
		) {
			issues.push(
				issue(
					"error",
					"app_embed_route_invalid",
					`${path}.config.route`,
					"App routes must start with '/'.",
				),
			);
		}
	}
	if (widget.type === "data") {
		const sourceKind = widget.config.sourceKind;
		const scope = widget.config.scope;
		const visualization = widget.config.visualization;
		if (
			sourceKind !== undefined &&
			!["table", "ontology", "query"].includes(String(sourceKind))
		) {
			issues.push(
				issue(
					"error",
					"data_source_kind_invalid",
					`${path}.config.sourceKind`,
					"Use table, ontology, or query.",
				),
			);
		}
		if (
			scope !== undefined &&
			!["project", "personal"].includes(String(scope))
		) {
			issues.push(
				issue(
					"error",
					"data_scope_invalid",
					`${path}.config.scope`,
					"Use project or personal.",
				),
			);
		}
		if (
			visualization !== undefined &&
			!HOME_DATA_VISUALIZATIONS.some(([id]) => id === visualization)
		) {
			issues.push(
				issue(
					"error",
					"data_visualization_invalid",
					`${path}.config.visualization`,
					"Choose a visualization returned by the widget catalog.",
				),
			);
		}
		const config = normalizeHomeDataConfig(widget.config);
		if (!config.appId) {
			issues.push(
				issue(
					"error",
					"data_source_missing_app",
					`${path}.config.appId`,
					"Choose an app from the current profile.",
				),
			);
		}
		const identity =
			config.sourceKind === "table"
				? config.table
				: config.sourceKind === "ontology"
					? config.ontologyId
					: config.queryId;
		if (!identity) {
			issues.push(
				issue(
					"error",
					"data_source_missing",
					`${path}.config.${
						config.sourceKind === "table"
							? "table"
							: config.sourceKind === "ontology"
								? "ontologyId"
								: "queryId"
					}`,
					"Choose an available data source.",
				),
			);
		}
		if (config.sourceKind === "ontology" && !config.objectType) {
			issues.push(
				issue(
					"error",
					"ontology_object_type_missing",
					`${path}.config.objectType`,
					"Choose an object type from the ontology.",
				),
			);
		}
	}
	validateSafeLinks(widget.config, `${path}.config`, issues);
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
	return {
		status: "ok",
		total: matches.length,
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
			scopes: ["project", "personal"],
			modes: ["aggregate", "records"],
			aggregations: [...HOME_DATA_AGGREGATIONS],
			filter_operators: [...HOME_DATA_FILTER_OPERATORS],
			time_buckets: ["none", "day", "week", "month", "quarter", "year"],
			date_ranges: ["all", "7d", "30d", "90d", "year"],
			sort_directions: ["asc", "desc"],
			formats: ["number", "currency", "percent"],
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
	const {
		appId,
		scope,
		kinds,
		query,
		sourceId,
		objectTypeQuery,
		columnQuery,
	} = parsed;
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
					sources.tables = await Promise.all(
						names.slice(0, MAX_TABLES).map(async (name) => {
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
								const columnsTruncated = columns.length > MAX_COLUMNS;
								truncated ||= columnsTruncated;
								return {
									name,
									columns: columns.slice(0, MAX_COLUMNS),
									columns_matched_total: columns.length,
									columns_truncated: columnsTruncated,
								};
							} catch {
								unreadable.add("table");
								return { name, columns: [], schema_unreadable: true };
							}
						}),
					);
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
							const objectTypes = overlay.nodes.filter((node) =>
								matchesSourceQuery(objectTypeQuery, node.label, node.table),
							);
							const objectTypesTruncated =
								objectTypes.length > MAX_OBJECT_TYPES;
							truncated ||= objectTypesTruncated;
							return {
								ontology_id: overlay.id,
								name: overlay.name,
								description: overlay.description,
								object_types: objectTypes
									.slice(0, MAX_OBJECT_TYPES)
									.map((node) => {
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
										const columnsTruncated = columns.length > MAX_COLUMNS;
										truncated ||= columnsTruncated;
										return {
											name: node.label,
											table: node.table,
											id_column: node.id_column,
											display_column: node.display_column,
											columns: columns.slice(0, MAX_COLUMNS),
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
					sources.queries = queries.slice(0, MAX_QUERIES).map((query) => ({
						query_id: query.id,
						name: query.name,
						description: query.description,
						kind: query.kind,
						surface: query.surface,
						overlay_id: query.overlay_id,
						parameters: extractHomeQueryParameters(query.sql),
						param_schema: query.param_schema,
						viz_config: query.viz_config,
						default_limit: query.default_limit,
					}));
				}
			} catch {
				unreadable.add(kind);
			}
		}),
	);

	return {
		status: unreadable.size || truncated ? "partial" : "ok",
		app_id: appId,
		scope,
		query: query || undefined,
		source_id: sourceId || undefined,
		object_type_query: objectTypeQuery || undefined,
		column_query: columnQuery || undefined,
		matched_totals: matchedTotals,
		complete: unreadable.size === 0 && !truncated,
		truncated,
		unreadable: [...unreadable].sort(),
		sources,
	};
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
		add(stringArg(widget.config, "appId"), `${path}.appId`);
		if (Array.isArray(widget.config.appIds)) {
			widget.config.appIds.forEach((appId, appIndex) => {
				if (typeof appId === "string")
					add(appId.trim(), `${path}.appIds[${appIndex}]`);
			});
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
	add(config.groupBy, "groupBy");
	add(config.seriesBy, "seriesBy");
	add(config.dateField, "dateField");
	add(config.sortBy, "sortBy");
	add(config.xField, "xField");
	add(config.yField, "yField");
	config.fields.forEach((field, index) => add(field, `fields[${index}]`));
	config.measures.forEach((measure, index) => {
		if (measure.aggregation !== "count")
			add(measure.field, `measures[${index}].field`);
	});
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
					} else if (!event.active) {
						issues.push(
							issue(
								"warning",
								"home_app_event_inactive",
								`${path}.eventId`,
								`Event '${eventId}' is inactive.`,
							),
						);
					}
				} else if (target === "route") {
					const route = stringArg(widget.config, "route") || "/";
					if (route !== "/" && !events.some((event) => event.route === route)) {
						issues.push(
							issue(
								"warning",
								"home_app_route_unconfirmed",
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
				for (const parameter of extractHomeQueryParameters(query.sql)) {
					if (!Object.hasOwn(config.queryParams, parameter)) {
						issues.push(
							issue(
								"error",
								"home_query_parameter_missing",
								`${path}.queryParams.${parameter}`,
								`Saved query '${query.name}' requires parameter '${parameter}'.`,
							),
						);
					}
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
