import { DEFAULT_HOME_DATA_CONFIG } from "../../../../home/home-data-query";
import {
	filterItem,
	measureItem,
	number,
	reference,
	strings,
	text,
} from "./fields";
import {
	AGGREGATE_ONLY_VISUALIZATIONS,
	DATA_VISUALIZATIONS,
	EITHER_MODE_VISUALIZATIONS,
	RECORD_ONLY_VISUALIZATIONS,
} from "./options";
import type { HomeWidgetConfigContract } from "./types";

export const HOME_DATA_WIDGET_CONFIG_CONTRACT = {
	description: "A table, ontology, or saved-query visualization.",
	additional_properties: false,
	fields: {
		sourceKind: text("Data source kind.", {
			default: DEFAULT_HOME_DATA_CONFIG.sourceKind,
			enum: ["table", "ontology", "query"],
		}),
		appId: text("App from the current profile that owns the source.", {
			reference: reference("profile_app_id", "list_apps"),
		}),
		scope: text("Project or current-user personal data.", {
			default: DEFAULT_HOME_DATA_CONFIG.scope,
			enum: ["project", "personal"],
		}),
		table: text("Exact table name.", {
			reference: reference("table_name", "list_home_data_sources", {
				app_field: "appId",
				scope_field: "scope",
				source_field: "sourceKind",
				when: { field: "sourceKind", equals: "table" },
			}),
		}),
		ontologyId: text("Exact ontology id.", {
			reference: reference("ontology_id", "list_home_data_sources", {
				app_field: "appId",
				scope_field: "scope",
				source_field: "sourceKind",
				when: { field: "sourceKind", equals: "ontology" },
			}),
		}),
		objectType: text("Exact ontology object-type label.", {
			reference: reference("ontology_object_type", "list_home_data_sources", {
				app_field: "appId",
				scope_field: "scope",
				source_field: "ontologyId",
				when: { field: "sourceKind", equals: "ontology" },
			}),
		}),
		queryId: text("Exact saved-query id.", {
			reference: reference("saved_query_id", "list_home_data_sources", {
				app_field: "appId",
				scope_field: "scope",
				source_field: "sourceKind",
				when: { field: "sourceKind", equals: "query" },
			}),
		}),
		queryParams: {
			type: "object",
			description: "Saved-query parameters. Values may use $viewer.id.",
			reference: reference("saved_query_parameters", "list_home_data_sources", {
				source_field: "queryId",
				when: { field: "sourceKind", equals: "query" },
			}),
		},
		visualization: text("Data presentation.", {
			default: DEFAULT_HOME_DATA_CONFIG.visualization,
			enum: DATA_VISUALIZATIONS,
		}),
		mode: text("Aggregate measures or source records.", {
			default: DEFAULT_HOME_DATA_CONFIG.mode,
			enum: ["aggregate", "records"],
		}),
		measures: {
			type: "object_list",
			description: "Up to six aggregate measures.",
			min_items: 1,
			max_items: 6,
			item: measureItem,
		},
		groupBy: text("Grouping or status field.", {
			reference: reference("data_field", "list_home_data_sources"),
		}),
		seriesBy: text("Secondary series field.", {
			reference: reference("data_field", "list_home_data_sources"),
		}),
		timeBucket: text("UTC date bucketing for groupBy.", {
			default: DEFAULT_HOME_DATA_CONFIG.timeBucket,
			enum: ["none", "day", "week", "month", "quarter", "year"],
		}),
		filters: {
			type: "object_list",
			description: "All-match source filters.",
			max_items: 12,
			item: filterItem,
		},
		dateField: text("Field used by a relative date range.", {
			reference: reference("data_field", "list_home_data_sources"),
		}),
		dateRange: text("Relative UTC time range.", {
			default: DEFAULT_HOME_DATA_CONFIG.dateRange,
			enum: ["all", "7d", "30d", "90d", "year"],
		}),
		sortBy: text("Source field, value, or group.", {
			default: DEFAULT_HOME_DATA_CONFIG.sortBy,
			reference: reference("data_field", "list_home_data_sources"),
		}),
		sortDirection: text("Result order.", {
			default: DEFAULT_HOME_DATA_CONFIG.sortDirection,
			enum: ["asc", "desc"],
		}),
		limit: number("Maximum returned rows.", {
			default: DEFAULT_HOME_DATA_CONFIG.limit,
			integer: true,
			minimum: 1,
			maximum: 500,
		}),
		refreshSeconds: number(
			"Refresh interval. Zero disables periodic refresh.",
			{
				default: DEFAULT_HOME_DATA_CONFIG.refreshSeconds,
				integer: true,
				ranges: [
					{ minimum: 0, maximum: 0 },
					{ minimum: 30, maximum: 3600 },
				],
			},
		),
		fields: strings("Visible record fields, in order.", {
			max_items: 20,
			unique_items: true,
			reference: reference("data_field", "list_home_data_sources"),
		}),
		xField: text("X, source-id, or record-date field.", {
			reference: reference("data_field", "list_home_data_sources"),
		}),
		yField: text("Y, target-id, or distribution field.", {
			reference: reference("data_field", "list_home_data_sources"),
		}),
		binWidth: number("Positive histogram bin width.", {
			default: DEFAULT_HOME_DATA_CONFIG.binWidth,
			exclusive_minimum: 0,
		}),
		format: text("Numeric display format.", {
			default: DEFAULT_HOME_DATA_CONFIG.format,
			enum: ["number", "currency", "percent"],
		}),
		currency: text("Uppercase ISO-style currency code.", {
			default: DEFAULT_HOME_DATA_CONFIG.currency,
			pattern: "^[A-Z]{3}$",
		}),
		decimals: number("Decimal places.", {
			default: DEFAULT_HOME_DATA_CONFIG.decimals,
			integer: true,
			minimum: 0,
			maximum: 6,
		}),
		target: number("Optional positive target for target visualizations.", {
			default: null,
			nullable: true,
		}),
		categoryOrder: strings("Optional explicit category order.", {
			max_items: 100,
		}),
		baseline: number("Waterfall starting value.", {
			default: DEFAULT_HOME_DATA_CONFIG.baseline,
		}),
	},
	requirements: [
		{
			code: "data_source_missing_app",
			message: "Choose an app from the current profile.",
			non_empty: ["appId"],
		},
		{
			code: "data_table_missing",
			message: "Choose an available table.",
			when: { field: "sourceKind", equals: "table" },
			non_empty: ["table"],
		},
		{
			code: "data_ontology_missing",
			message: "Choose an ontology and object type.",
			when: { field: "sourceKind", equals: "ontology" },
			non_empty: ["ontologyId", "objectType"],
		},
		{
			code: "data_query_missing",
			message: "Choose an available saved query.",
			when: { field: "sourceKind", equals: "query" },
			non_empty: ["queryId"],
		},
		{
			code: "data_target_missing",
			message:
				"Progress, gauge, and bullet views need a target greater than zero.",
			when: { field: "visualization", in: ["progress", "gauge", "bullet"] },
			positive: ["target"],
		},
		{
			code: "data_xy_fields_missing",
			message:
				"Scatter and graph views need both X/source and Y/target fields.",
			when: { field: "visualization", in: ["scatter", "graph"] },
			non_empty: ["xField", "yField"],
		},
		{
			code: "data_group_series_missing",
			message: "This visualization needs both group and series fields.",
			when: {
				field: "visualization",
				in: ["heatmap", "pivot", "sankey", "percentstacked"],
			},
			non_empty: ["groupBy", "seriesBy"],
		},
		{
			code: "data_calendar_fields_invalid",
			message: "Calendar needs a date group, daily bucketing, and no series.",
			when: { field: "visualization", equals: "calendar" },
			non_empty: ["groupBy"],
			empty: ["seriesBy"],
			equals: { timeBucket: "day" },
		},
		{
			code: "data_kanban_group_missing",
			message: "Kanban needs a status field.",
			when: { field: "visualization", equals: "kanban" },
			non_empty: ["groupBy"],
		},
		{
			code: "data_record_date_missing",
			message: "Timeline and record-calendar views need a date field.",
			when: {
				field: "visualization",
				in: ["timeline", "recordcalendar"],
			},
			non_empty: ["xField"],
		},
		{
			code: "data_boxplot_field_missing",
			message: "Box plot needs a numeric distribution field.",
			when: { field: "visualization", equals: "boxplot" },
			non_empty: ["yField"],
		},
		{
			code: "data_histogram_field_missing",
			message: "Histogram needs a numeric grouping field.",
			when: { field: "visualization", equals: "histogram" },
			non_empty: ["groupBy"],
		},
		{
			code: "data_date_field_missing",
			message: "A relative date range needs a date field.",
			when: { field: "dateRange", in: ["7d", "30d", "90d", "year"] },
			non_empty: ["dateField"],
		},
	],
	relations: [
		{
			kind: "mode_by_visualization",
			record_only: RECORD_ONLY_VISUALIZATIONS,
			aggregate_only: AGGREGATE_ONLY_VISUALIZATIONS,
			either: EITHER_MODE_VISUALIZATIONS,
		},
		{
			kind: "saved_query_parameter_namespace",
			reserved_prefix: "__home_",
		},
	],
} satisfies HomeWidgetConfigContract;
