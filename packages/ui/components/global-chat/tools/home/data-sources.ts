import type { IBackendState } from "../../../../state/backend-state";
import {
	extractHomeQueryParameters,
	homeDataColumns,
	homeOntologyColumns,
} from "../../../home/home-data-query";
import { DATA_SOURCE_KINDS, stringArg } from "./shared";
import type { HomeDataScope, HomeDataSourceKind } from "./types";

const MAX_TABLES = 60;

const MAX_COLUMNS = 80;

const MAX_ONTOLOGIES = 60;

const MAX_OBJECT_TYPES = 80;

const MAX_QUERIES = 100;

const MAX_TOTAL_COLUMN_RECORDS = 240;

const MAX_DATA_SOURCE_RESPONSE_BYTES = 128 * 1024;

const MAX_PAYLOAD_FALLBACK_SOURCES = 20;

function matchesSourceQuery(
	query: string,
	...values: Array<string | undefined>
) {
	if (!query) return true;
	return values.some((value) => value?.toLowerCase().includes(query));
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
