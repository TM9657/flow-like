import { APP_CATEGORY_ORDER } from "../../../lib/category-meta";
import {
	HOME_APP_RENDERINGS,
	HOME_MODEL_RENDERINGS,
	HOME_PACKAGE_RENDERINGS,
	safeHomeHref,
} from "../../home/home-content/config";
import {
	DEFAULT_HOME_DATA_CONFIG,
	HOME_DATA_AGGREGATIONS,
	HOME_DATA_FILTER_OPERATORS,
	HOME_DATA_VISUALIZATIONS,
	normalizeHomeDataConfig,
} from "../../home/home-data-query";

export const KNOWN_HOME_WIDGET_TYPES = [
	"ai-usage",
	"app-collection",
	"app-collection-feature",
	"app-embed",
	"app-ranking",
	"app-spotlight",
	"categories",
	"data",
	"executions-by-app",
	"flowpilot",
	"greeting",
	"information",
	"model-spotlight",
	"models",
	"needs-attention",
	"notifications",
	"packages",
	"quick-actions",
	"quick-links",
	"recent-runs",
	"run-activity",
	"run-stats",
	"schedules",
	"section-heading",
	"workspace-pulse",
] as const;

export type KnownHomeWidgetType = (typeof KNOWN_HOME_WIDGET_TYPES)[number];

export type HomeWidgetConfigReferenceKind =
	| "app_event_id"
	| "app_route"
	| "data_field"
	| "model_hub"
	| "model_id"
	| "ontology_id"
	| "ontology_object_type"
	| "profile_app_id"
	| "profile_app_ids"
	| "safe_home_url"
	| "saved_query_id"
	| "saved_query_parameters"
	| "table_name";

export interface HomeWidgetConfigReferenceContract {
	kind: HomeWidgetConfigReferenceKind;
	provenance:
		| "app_interface"
		| "current_profile"
		| "home_data_source"
		| "local_safety_check"
		| "manual_profile_model";
	validation:
		| "local"
		| "reference"
		| "reference_when_schema_available"
		| "unverified";
	discover_with?: string;
	inspect_with?: string;
	note?: string;
	app_field?: string;
	scope_field?: string;
	source_field?: string;
	when?: HomeWidgetConfigCondition;
}

type ContractScalar = string | number | boolean | null;

export interface HomeWidgetConfigCondition {
	field?: string;
	equals?: ContractScalar;
	in?: readonly ContractScalar[];
	not_equals?: ContractScalar;
	not_in?: readonly ContractScalar[];
	all?: readonly HomeWidgetConfigCondition[];
}

export interface HomeWidgetConfigRequirement {
	code: string;
	message: string;
	severity?: "error" | "warning";
	when?: HomeWidgetConfigCondition;
	required?: readonly string[];
	non_empty?: readonly string[];
	any_non_empty?: readonly string[];
	positive?: readonly string[];
	empty?: readonly string[];
	equals?: Readonly<Record<string, ContractScalar>>;
	numeric_strings?: readonly string[];
	allowed_values?: Readonly<Record<string, readonly ContractScalar[]>>;
}

export interface HomeWidgetObjectContract {
	additional_properties: boolean;
	fields: Readonly<Record<string, HomeWidgetConfigFieldContract>>;
	required?: readonly string[];
	requirements?: readonly HomeWidgetConfigRequirement[];
	unique_by?: string;
}

export interface HomeWidgetConfigFieldContract {
	type:
		| "boolean"
		| "number"
		| "object"
		| "object_list"
		| "string"
		| "string_list";
	description?: string;
	default?: unknown;
	enum?: readonly ContractScalar[];
	legacy_values?: readonly {
		value: ContractScalar;
		maps_to: ContractScalar;
	}[];
	minimum?: number;
	exclusive_minimum?: number;
	maximum?: number;
	integer?: boolean;
	nullable?: boolean;
	min_items?: number;
	max_items?: number;
	unique_items?: boolean;
	item_enum?: readonly string[];
	item?: HomeWidgetObjectContract;
	pattern?: string;
	string_format?: "date-time" | "image-url" | "route";
	reference?: HomeWidgetConfigReferenceContract;
	ranges?: readonly { minimum: number; maximum: number }[];
}

export interface HomeWidgetConfigContract extends HomeWidgetObjectContract {
	description: string;
	relations?: readonly Record<string, unknown>[];
}

export interface HomeWidgetConfigIssue {
	severity: "error" | "warning";
	code: string;
	path: string;
	message: string;
}

const DATA_VISUALIZATIONS = HOME_DATA_VISUALIZATIONS.map(([id]) => id);
const APP_RENDERINGS = HOME_APP_RENDERINGS.map(([id]) => id);
const MODEL_RENDERINGS = HOME_MODEL_RENDERINGS.map(([id]) => id);
const PACKAGE_RENDERINGS = HOME_PACKAGE_RENDERINGS.map(([id]) => id);
const APP_CATEGORIES = ["", ...APP_CATEGORY_ORDER];
const APP_SOURCES = [
	"library",
	"recent",
	"favorites",
	"manual",
	"new",
	"popular",
];
const DISCOVERY_APP_SOURCES = ["new", "popular", "library", "manual"];
const INFORMATION_MODES = [
	"markdown",
	"banner",
	"announcement",
	"faq",
	"checklist",
	"countdown",
	"story",
	"image",
	"quote",
	"feed",
	"steps",
	"resources",
	"facts",
];
const QUICK_ACTION_IDS = [
	"create",
	"import",
	"library",
	"packages",
	"explore",
	"learn",
];
const ACTIVITY_DAYS = [1, 7, 30];
const NOTIFICATION_TYPES = ["all", "WORKFLOW", "SYSTEM"];
const RUN_STAT_METRICS = [
	"overview",
	"executions",
	"ai",
	"embeddings",
	"errors",
	"duration",
];
const APP_EMBED_RESERVED_QUERY_KEYS = new Set([
	"id",
	"route",
	"eventId",
	"__proto__",
	"constructor",
	"prototype",
]);
const RECORD_ONLY_VISUALIZATIONS = [
	"kanban",
	"record",
	"graph",
	"scatter",
	"timeline",
	"recordcalendar",
	"comparison",
];
const EITHER_MODE_VISUALIZATIONS = ["table", "list", "cards"];
const AGGREGATE_ONLY_VISUALIZATIONS = DATA_VISUALIZATIONS.filter(
	(value) =>
		!RECORD_ONLY_VISUALIZATIONS.includes(value) &&
		!EITHER_MODE_VISUALIZATIONS.includes(value),
);

const reference = (
	kind: HomeWidgetConfigReferenceKind,
	discoverWith?: string,
	dependencies: Pick<
		HomeWidgetConfigReferenceContract,
		"app_field" | "scope_field" | "source_field" | "when"
	> = {},
): HomeWidgetConfigReferenceContract => {
	const details: Record<
		HomeWidgetConfigReferenceKind,
		Pick<
			HomeWidgetConfigReferenceContract,
			"note" | "provenance" | "validation"
		>
	> = {
		app_event_id: {
			provenance: "app_interface",
			validation: "reference",
			note: "Choose an active Event with a nonempty default_page_id or event_type simple_chat, generic_form, or quick_action.",
		},
		app_route: { provenance: "app_interface", validation: "reference" },
		data_field: {
			provenance: "home_data_source",
			validation: "reference_when_schema_available",
			note: "Saved-query result fields cannot be verified unless the source exposes a result schema.",
		},
		model_hub: {
			provenance: "manual_profile_model",
			validation: "unverified",
			note: "No Home model inventory tool currently resolves exact model references. Prefer source plus query unless the exact reference is already known.",
		},
		model_id: {
			provenance: "manual_profile_model",
			validation: "unverified",
			note: "No Home model inventory tool currently resolves exact model references. Prefer source plus query unless the exact reference is already known.",
		},
		ontology_id: {
			provenance: "home_data_source",
			validation: "reference",
		},
		ontology_object_type: {
			provenance: "home_data_source",
			validation: "reference",
		},
		profile_app_id: {
			provenance: "current_profile",
			validation: "reference",
		},
		profile_app_ids: {
			provenance: "current_profile",
			validation: "reference",
		},
		safe_home_url: {
			provenance: "local_safety_check",
			validation: "local",
		},
		saved_query_id: {
			provenance: "home_data_source",
			validation: "reference",
		},
		saved_query_parameters: {
			provenance: "home_data_source",
			validation: "reference",
		},
		table_name: {
			provenance: "home_data_source",
			validation: "reference",
		},
	};
	return {
		kind,
		...details[kind],
		...(discoverWith ? { discover_with: discoverWith } : {}),
		...dependencies,
	};
};

const text = (
	description: string,
	options: Partial<HomeWidgetConfigFieldContract> = {},
): HomeWidgetConfigFieldContract => ({
	type: "string",
	description,
	...options,
});
const number = (
	description: string,
	options: Partial<HomeWidgetConfigFieldContract> = {},
): HomeWidgetConfigFieldContract => ({
	type: "number",
	description,
	...options,
});
const boolean = (
	description: string,
	defaultValue?: boolean,
): HomeWidgetConfigFieldContract => ({
	type: "boolean",
	description,
	...(defaultValue === undefined ? {} : { default: defaultValue }),
});
const strings = (
	description: string,
	options: Partial<HomeWidgetConfigFieldContract> = {},
): HomeWidgetConfigFieldContract => ({
	type: "string_list",
	description,
	...options,
});
const count = (defaultValue: number, maximum = 50) =>
	number("Maximum items to show.", {
		default: defaultValue,
		integer: true,
		minimum: 1,
		maximum,
	});
const profileApp = (description: string, when?: HomeWidgetConfigCondition) =>
	text(description, {
		default: "",
		reference: reference("profile_app_id", "list_apps", { when }),
	});
const optionalProfileApp = profileApp(
	"Optional app filter from the current profile.",
);
const profileApps = (maximum?: number, when?: HomeWidgetConfigCondition) =>
	strings("Ordered app ids from the current profile.", {
		unique_items: true,
		...(maximum === undefined ? {} : { max_items: maximum }),
		reference: reference("profile_app_ids", "list_apps", { when }),
	});
const safeUrl = (description: string) =>
	text(description, {
		reference: reference("safe_home_url"),
	});
const imageUrl = (description: string) =>
	text(description, {
		string_format: "image-url",
		reference: reference("safe_home_url"),
	});

const informationItem: HomeWidgetObjectContract = {
	additional_properties: false,
	fields: {
		id: text("Optional stable item id."),
		title: text("Item heading."),
		body: text("Optional Markdown body."),
		href: safeUrl("Optional item destination."),
		label: text("Optional short label, date, or category."),
		checked: boolean("Checklist completion state.", false),
	},
	required: ["title"],
	unique_by: "id",
};

const linkItem: HomeWidgetObjectContract = {
	additional_properties: false,
	fields: {
		id: text("Optional stable link id."),
		title: text("Link title."),
		description: text("Optional link description."),
		href: safeUrl("Link destination."),
	},
	required: ["title", "href"],
	unique_by: "id",
};

const measureItem: HomeWidgetObjectContract = {
	additional_properties: false,
	fields: {
		id: text("Optional stable measure id."),
		aggregation: text("Aggregation to calculate.", {
			enum: HOME_DATA_AGGREGATIONS,
		}),
		field: text("Source field used by non-count aggregations.", {
			reference: reference("data_field", "list_home_data_sources", {
				source_field: "sourceKind",
			}),
		}),
		label: text("Optional display label."),
	},
	required: ["aggregation"],
	requirements: [
		{
			code: "data_measure_field_missing",
			message: "Choose a source field for every aggregation except count.",
			when: { field: "aggregation", not_equals: "count" },
			non_empty: ["field"],
		},
	],
	unique_by: "id",
};

const filterItem: HomeWidgetObjectContract = {
	additional_properties: false,
	fields: {
		id: text("Optional stable filter id."),
		field: text("Source field to filter.", {
			reference: reference("data_field", "list_home_data_sources", {
				source_field: "sourceKind",
			}),
		}),
		operator: text("Filter comparison.", { enum: HOME_DATA_FILTER_OPERATORS }),
		value: text("Comparison value. Empty and not_empty ignore this field."),
		valueType: text("How the comparison value is converted.", {
			enum: ["text", "number", "boolean", "viewer"],
		}),
	},
	required: ["field", "operator", "valueType"],
	requirements: [
		{
			code: "data_filter_number_invalid",
			message: "A numeric filter needs a finite numeric value.",
			when: {
				all: [
					{ field: "valueType", equals: "number" },
					{ field: "operator", not_in: ["empty", "not_empty"] },
				],
			},
			numeric_strings: ["value"],
		},
		{
			code: "data_filter_boolean_invalid",
			message: "A boolean filter value must be true or false.",
			when: {
				all: [
					{ field: "valueType", equals: "boolean" },
					{ field: "operator", not_in: ["empty", "not_empty"] },
				],
			},
			allowed_values: { value: ["true", "false"] },
		},
	],
	unique_by: "id",
};

const appDiscoveryFields = (
	defaultSource: string,
	limitMaximum: number,
): Record<string, HomeWidgetConfigFieldContract> => ({
	source: text("Which app inventory supplies the widget.", {
		default: defaultSource,
		enum: DISCOVERY_APP_SOURCES,
	}),
	appId: profileApp("Exact app used by a single-app manual selection.", {
		field: "source",
		equals: "manual",
	}),
	appIds: profileApps(limitMaximum, {
		all: [
			{ field: "source", equals: "manual" },
			{ field: "appId", equals: "" },
		],
	}),
	query: text("Case-insensitive app search text."),
	category: text("Exact app category filter.", { enum: APP_CATEGORIES }),
	tag: text("Exact app tag filter."),
	eyebrow: text("Short label above the widget heading."),
});

const emptyContract = (description: string): HomeWidgetConfigContract => ({
	description,
	additional_properties: false,
	fields: {},
});

export const HOME_WIDGET_CONFIG_CONTRACTS = {
	"ai-usage": emptyContract("Shows account AI and embedding usage totals."),
	"app-collection": {
		description: "A filtered or hand-picked app collection.",
		additional_properties: false,
		fields: {
			source: text("Which app inventory supplies the collection.", {
				default: "library",
				enum: APP_SOURCES,
			}),
			appIds: profileApps(undefined, {
				field: "source",
				equals: "manual",
			}),
			query: text("Case-insensitive app search text."),
			category: text("Exact app category filter.", { enum: APP_CATEGORIES }),
			tag: text("Exact app tag filter."),
			limit: count(8),
			rendering: text("Collection card presentation.", {
				default: "standard",
				enum: APP_RENDERINGS,
				legacy_values: [{ value: "spotlight", maps_to: "editorial" }],
			}),
			showUpdated: boolean("Show app update dates.", false),
			maxColumns: number("Maximum card columns. Zero uses automatic layout.", {
				default: 0,
				enum: [0, 1, 2, 3, 4, 6],
			}),
		},
		requirements: [
			{
				severity: "warning",
				code: "app_collection_manual_empty",
				message:
					"A manual app collection should contain at least one profile app.",
				when: { field: "source", equals: "manual" },
				non_empty: ["appIds"],
			},
		],
	},
	"app-collection-feature": {
		description: "An editorial feature for a selected app collection.",
		additional_properties: false,
		fields: {
			...appDiscoveryFields("new", 6),
			limit: count(3, 6),
			headline: text("Feature headline."),
			description: text("Feature supporting copy."),
			linkLabel: text("Optional destination label."),
			href: safeUrl("Optional destination override."),
		},
		requirements: [
			{
				severity: "warning",
				code: "app_feature_manual_empty",
				message:
					"A manual app feature should contain at least one profile app.",
				when: { field: "source", equals: "manual" },
				any_non_empty: ["appId", "appIds"],
			},
		],
	},
	"app-embed": {
		description: "An embedded app landing page, route, or Event interface.",
		additional_properties: false,
		fields: {
			target: text("Embedded app destination kind.", {
				default: "landing",
				enum: ["landing", "route", "event"],
			}),
			appId: text("App from the current profile.", {
				reference: reference("profile_app_id", "list_apps"),
			}),
			route: text("App route beginning with a single slash.", {
				default: "/",
				string_format: "route",
				reference: reference("app_route", "list_apps", {
					app_field: "appId",
					when: { field: "target", equals: "route" },
				}),
			}),
			eventId: text("Embeddable Event interface id.", {
				reference: {
					...reference("app_event_id", "list_apps", {
						app_field: "appId",
						when: { field: "target", equals: "event" },
					}),
					inspect_with: "describe_app_interface",
				},
			}),
			query: text("URL-encoded query parameters without routing keys."),
		},
		requirements: [
			{
				code: "app_embed_missing_app",
				message: "Choose an app from the current profile.",
				non_empty: ["appId"],
			},
			{
				code: "app_embed_missing_event",
				message: "Choose an embeddable Event interface.",
				when: { field: "target", equals: "event" },
				non_empty: ["eventId"],
			},
		],
	},
	"app-ranking": {
		description: "A ranked app list from a live or manual source.",
		additional_properties: false,
		fields: {
			...appDiscoveryFields("popular", 10),
			limit: count(6, 10),
			linkLabel: text("Optional destination label."),
			href: safeUrl("Optional destination override."),
		},
		requirements: [
			{
				severity: "warning",
				code: "app_ranking_manual_empty",
				message:
					"A manual app ranking should contain at least one profile app.",
				when: { field: "source", equals: "manual" },
				any_non_empty: ["appId", "appIds"],
			},
		],
	},
	"app-spotlight": {
		description: "A hero or compact feature for one app.",
		additional_properties: false,
		fields: {
			...appDiscoveryFields("new", 1),
			mode: text("Spotlight size.", {
				default: "hero",
				enum: ["hero", "compact"],
			}),
		},
		requirements: [
			{
				severity: "warning",
				code: "app_spotlight_manual_empty",
				message: "A manual app spotlight should identify one profile app.",
				when: { field: "source", equals: "manual" },
				any_non_empty: ["appId", "appIds"],
			},
		],
	},
	categories: {
		description: "App category chips or tiles.",
		additional_properties: false,
		fields: {
			rendering: text("Category presentation.", {
				default: "chips",
				enum: ["chips", "grid"],
			}),
			categories: strings("Categories to show. Empty means all categories.", {
				unique_items: true,
				item_enum: APP_CATEGORY_ORDER,
			}),
		},
	},
	data: {
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
				reference: reference(
					"saved_query_parameters",
					"list_home_data_sources",
					{
						source_field: "queryId",
						when: { field: "sourceKind", equals: "query" },
					},
				),
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
	},
	"executions-by-app": {
		description: "Recent execution counts grouped by app.",
		additional_properties: false,
		fields: {
			days: number("UTC time window.", { default: 7, enum: ACTIVITY_DAYS }),
			appId: optionalProfileApp,
			limit: count(5),
		},
	},
	flowpilot: {
		description: "A FlowPilot entry point.",
		additional_properties: false,
		fields: {
			mode: text("FlowPilot presentation.", {
				default: "bar",
				enum: ["orb", "bar", "card", "hero"],
			}),
			suggestions: boolean("Show prompt suggestions."),
			emphasis: boolean("Emphasize the prompt with the accent color.", false),
			placeholder: text("Prompt input placeholder."),
		},
	},
	greeting: {
		description: "A personalized greeting.",
		additional_properties: false,
		fields: {
			mode: text("Greeting presentation.", {
				default: "standard",
				enum: ["standard", "masthead"],
			}),
			name: text("Optional name override."),
			subtitle: text("Optional welcome message."),
		},
	},
	information: {
		description: "Markdown, structured guidance, media, or an announcement.",
		additional_properties: false,
		fields: {
			mode: text("Information presentation.", {
				default: "markdown",
				enum: INFORMATION_MODES,
			}),
			body: text("Markdown body."),
			items: {
				type: "object_list",
				description:
					"Items for FAQ, checklist, feed, steps, resources, and facts modes.",
				item: informationItem,
			},
			layout: text("Resource presentation.", {
				default: "cards",
				enum: ["cards", "list"],
			}),
			date: text("Milestone date or date-time.", {
				string_format: "date-time",
			}),
			eyebrow: text("Short label above the content."),
			imageUrl: imageUrl("HTTP(S) or relative image URL."),
			imageAlt: text("Accessible image description."),
			actionLabel: text("Call-to-action label."),
			actionHref: safeUrl("Call-to-action destination."),
			attribution: text("Quote attribution."),
		},
		requirements: [
			{
				severity: "warning",
				code: "information_countdown_empty",
				message: "A countdown should include a valid milestone date.",
				when: { field: "mode", equals: "countdown" },
				non_empty: ["date"],
			},
		],
	},
	"model-spotlight": {
		description: "A feature for one profile or catalog model.",
		additional_properties: false,
		fields: {
			source: text("Model inventory.", {
				default: "explore",
				enum: ["explore", "profile", "manual"],
			}),
			modelId: text("Exact model id for manual selection.", {
				reference: reference("model_id", undefined, {
					when: { field: "source", equals: "manual" },
				}),
			}),
			modelHub: text("Optional model hub paired with modelId.", {
				reference: reference("model_hub", undefined, {
					when: { field: "source", equals: "manual" },
				}),
			}),
			query: text("Case-insensitive model search text."),
			eyebrow: text("Short label above the widget heading."),
		},
		requirements: [
			{
				code: "model_spotlight_manual_empty",
				message: "A manual model spotlight needs an exact model id.",
				when: { field: "source", equals: "manual" },
				non_empty: ["modelId"],
			},
		],
	},
	models: {
		description: "A profile or catalog model collection.",
		additional_properties: false,
		fields: {
			source: text("Model inventory.", {
				default: "profile",
				enum: ["profile", "explore"],
			}),
			query: text("Case-insensitive model search text."),
			limit: count(6),
			rendering: text("Model card presentation.", {
				default: "standard",
				enum: MODEL_RENDERINGS,
			}),
		},
	},
	"needs-attention": {
		description:
			"Unread workflow notifications and Error or Fatal execution records.",
		additional_properties: false,
		fields: {
			notificationType: text("Notification filter.", {
				default: "WORKFLOW",
				enum: NOTIFICATION_TYPES,
			}),
			limit: count(8),
			days: number("Execution lookback in UTC.", {
				default: 7,
				enum: ACTIVITY_DAYS,
			}),
			appId: optionalProfileApp,
		},
	},
	notifications: {
		description: "Personal notifications.",
		additional_properties: false,
		fields: {
			notificationType: text("Notification filter.", {
				default: "all",
				enum: NOTIFICATION_TYPES,
			}),
			unread: boolean("Only show unread notifications.", false),
			limit: count(8),
		},
	},
	packages: {
		description: "A package-store collection.",
		additional_properties: false,
		fields: {
			query: text("Package search text."),
			category: text("Optional package category."),
			limit: count(6),
			sort: text("Package sort order.", {
				default: "downloads",
				enum: ["downloads", "created_at", "updated_at", "relevance"],
			}),
			rendering: text("Package card presentation.", {
				default: "standard",
				enum: PACKAGE_RENDERINGS,
				legacy_values: [
					{ value: "list", maps_to: "compact" },
					{ value: "grid", maps_to: "standard" },
				],
			}),
		},
	},
	"quick-actions": {
		description: "Built-in Home actions.",
		additional_properties: false,
		fields: {
			actions: strings("Ordered built-in action ids.", {
				unique_items: true,
				item_enum: QUICK_ACTION_IDS,
			}),
			layout: text("Action presentation.", {
				default: "cards",
				enum: ["cards", "toolbar"],
			}),
		},
	},
	"quick-links": {
		description: "Internal or external destination links.",
		additional_properties: false,
		fields: {
			rendering: text("Link presentation.", {
				default: "grid",
				enum: ["grid", "list"],
			}),
			links: {
				type: "object_list",
				description: "Ordered destination links.",
				item: linkItem,
			},
		},
	},
	"recent-runs": {
		description: "Recent execution records.",
		additional_properties: false,
		fields: { appId: optionalProfileApp, limit: count(8) },
	},
	"run-activity": {
		description: "Execution activity by UTC day.",
		additional_properties: false,
		fields: {
			days: number("UTC time window.", { default: 7, enum: ACTIVITY_DAYS }),
			appId: optionalProfileApp,
		},
	},
	"run-stats": {
		description: "Recorded execution or AI usage statistics.",
		additional_properties: false,
		fields: {
			metric: text("Statistic to display.", {
				default: "overview",
				enum: RUN_STAT_METRICS,
			}),
			appId: profileApp("Optional app filter from the current profile.", {
				field: "metric",
				in: ["errors", "duration"],
			}),
		},
	},
	schedules: {
		description: "Upcoming active app schedules.",
		additional_properties: false,
		fields: { appIds: profileApps(), limit: count(8) },
	},
	"section-heading": {
		description: "A section title with an optional destination.",
		additional_properties: false,
		fields: {
			linkLabel: text("Optional destination label."),
			href: safeUrl("Optional destination."),
		},
	},
	"workspace-pulse": {
		description: "Profile app and account activity summary.",
		additional_properties: false,
		fields: {
			mode: text("Workspace summary presentation.", {
				default: "card",
				enum: ["card", "strip", "attention"],
			}),
			days: number("UTC activity period.", { default: 7, enum: ACTIVITY_DAYS }),
			showAttention: boolean("Include Error or Fatal records.", true),
		},
	},
} satisfies Record<KnownHomeWidgetType, HomeWidgetConfigContract>;

function isRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function valueAt(
	value: Record<string, unknown>,
	contract: HomeWidgetObjectContract,
	key: string,
) {
	return value[key] ?? contract.fields[key]?.default;
}

function conditionMatches(
	condition: HomeWidgetConfigCondition | undefined,
	value: Record<string, unknown>,
	contract: HomeWidgetObjectContract,
) {
	if (!condition) return true;
	if (
		condition.all &&
		!condition.all.every((entry) => conditionMatches(entry, value, contract))
	)
		return false;
	if (!condition.field) return true;
	const actual = valueAt(value, contract, condition.field);
	if (condition.equals !== undefined) return actual === condition.equals;
	if (condition.in) return condition.in.includes(actual as ContractScalar);
	if (condition.not_equals !== undefined)
		return actual !== condition.not_equals;
	if (condition.not_in)
		return !condition.not_in.includes(actual as ContractScalar);
	return true;
}

function hasValue(value: unknown) {
	return typeof value === "string"
		? Boolean(value.trim())
		: Array.isArray(value)
			? value.length > 0
			: value !== undefined && value !== null;
}

function pushIssue(
	issues: HomeWidgetConfigIssue[],
	code: string,
	path: string,
	message: string,
	severity: "error" | "warning" = "error",
) {
	issues.push({ severity, code, path, message });
}

function validateRequirements(
	value: Record<string, unknown>,
	contract: HomeWidgetObjectContract,
	path: string,
	issues: HomeWidgetConfigIssue[],
) {
	for (const requirement of contract.requirements ?? []) {
		if (!conditionMatches(requirement.when, value, contract)) continue;
		const severity = requirement.severity ?? "error";
		for (const key of requirement.required ?? []) {
			if (value[key] !== undefined) continue;
			pushIssue(
				issues,
				requirement.code,
				`${path}.${key}`,
				requirement.message,
				severity,
			);
		}
		for (const key of requirement.non_empty ?? []) {
			if (hasValue(valueAt(value, contract, key))) continue;
			pushIssue(
				issues,
				requirement.code,
				`${path}.${key}`,
				requirement.message,
				severity,
			);
		}
		if (
			requirement.any_non_empty?.length &&
			!requirement.any_non_empty.some((key) =>
				hasValue(valueAt(value, contract, key)),
			)
		)
			pushIssue(issues, requirement.code, path, requirement.message, severity);
		for (const key of requirement.positive ?? []) {
			const actual = valueAt(value, contract, key);
			if (typeof actual === "number" && Number.isFinite(actual) && actual > 0)
				continue;
			pushIssue(
				issues,
				requirement.code,
				`${path}.${key}`,
				requirement.message,
				severity,
			);
		}
		for (const key of requirement.empty ?? []) {
			if (!hasValue(valueAt(value, contract, key))) continue;
			pushIssue(
				issues,
				requirement.code,
				`${path}.${key}`,
				requirement.message,
				severity,
			);
		}
		for (const [key, expected] of Object.entries(requirement.equals ?? {})) {
			if (valueAt(value, contract, key) === expected) continue;
			pushIssue(
				issues,
				requirement.code,
				`${path}.${key}`,
				requirement.message,
				severity,
			);
		}
		for (const key of requirement.numeric_strings ?? []) {
			const actual = valueAt(value, contract, key);
			if (
				typeof actual === "string" &&
				actual.trim() &&
				Number.isFinite(Number(actual))
			)
				continue;
			pushIssue(
				issues,
				requirement.code,
				`${path}.${key}`,
				requirement.message,
				severity,
			);
		}
		for (const [key, allowed] of Object.entries(
			requirement.allowed_values ?? {},
		)) {
			if (allowed.includes(valueAt(value, contract, key) as ContractScalar))
				continue;
			pushIssue(
				issues,
				requirement.code,
				`${path}.${key}`,
				requirement.message,
				severity,
			);
		}
	}
}

function validateField(
	value: unknown,
	contract: HomeWidgetConfigFieldContract,
	path: string,
	issues: HomeWidgetConfigIssue[],
) {
	if (value === null && contract.nullable) return;
	const typeMatches =
		(contract.type === "string" && typeof value === "string") ||
		(contract.type === "number" &&
			typeof value === "number" &&
			Number.isFinite(value)) ||
		(contract.type === "boolean" && typeof value === "boolean") ||
		(contract.type === "object" && isRecord(value)) ||
		(contract.type === "string_list" &&
			Array.isArray(value) &&
			value.every((entry) => typeof entry === "string")) ||
		(contract.type === "object_list" &&
			Array.isArray(value) &&
			value.every(isRecord));
	if (!typeMatches) {
		pushIssue(
			issues,
			"home_widget_config_type_invalid",
			path,
			`Expected ${contract.type}.`,
		);
		return;
	}
	if (contract.enum && !contract.enum.includes(value as ContractScalar)) {
		const legacy = contract.legacy_values?.find(
			(entry) => entry.value === value,
		);
		if (legacy)
			pushIssue(
				issues,
				"home_widget_config_legacy_value",
				path,
				`This legacy value renders as '${legacy.maps_to}'. Use '${legacy.maps_to}' for new layouts.`,
				"warning",
			);
		else
			pushIssue(
				issues,
				"home_widget_config_option_invalid",
				path,
				`Use one of: ${contract.enum.join(", ")}.`,
			);
	}
	if (typeof value === "string") {
		if (contract.pattern && !new RegExp(contract.pattern).test(value))
			pushIssue(
				issues,
				"home_widget_config_pattern_invalid",
				path,
				`Value must match ${contract.pattern}.`,
			);
		if (
			contract.string_format === "date-time" &&
			value &&
			!Number.isFinite(new Date(value).getTime())
		)
			pushIssue(
				issues,
				"home_widget_config_date_invalid",
				path,
				"Use a valid date or date-time.",
			);
		if (
			contract.string_format === "route" &&
			value &&
			(!value.startsWith("/") || value.startsWith("//"))
		)
			pushIssue(
				issues,
				"home_widget_config_route_invalid",
				path,
				"App routes must start with one '/'.",
			);
		if (
			contract.reference?.kind === "safe_home_url" &&
			value.trim() &&
			!safeHomeHref(value)
		)
			pushIssue(
				issues,
				"unsafe_home_url",
				path,
				"Use a relative path or an http, https, mailto, or tel URL.",
			);
		if (contract.string_format === "image-url" && value.trim()) {
			const href = safeHomeHref(value);
			if (
				!href ||
				(!href.startsWith("/") &&
					!href.startsWith("http://") &&
					!href.startsWith("https://"))
			)
				pushIssue(
					issues,
					"home_widget_config_image_url_invalid",
					path,
					"Use a relative path or an http or https image URL.",
				);
		}
	}
	if (typeof value === "number") {
		if (contract.integer && !Number.isInteger(value))
			pushIssue(
				issues,
				"home_widget_config_integer_required",
				path,
				"Use an integer.",
			);
		if (contract.minimum !== undefined && value < contract.minimum)
			pushIssue(
				issues,
				"home_widget_config_number_too_small",
				path,
				`Use ${contract.minimum} or greater.`,
			);
		if (
			contract.exclusive_minimum !== undefined &&
			value <= contract.exclusive_minimum
		)
			pushIssue(
				issues,
				"home_widget_config_number_too_small",
				path,
				`Use a value greater than ${contract.exclusive_minimum}.`,
			);
		if (contract.maximum !== undefined && value > contract.maximum)
			pushIssue(
				issues,
				"home_widget_config_number_too_large",
				path,
				`Use ${contract.maximum} or less.`,
			);
		if (
			contract.ranges &&
			!contract.ranges.some(
				(range) => value >= range.minimum && value <= range.maximum,
			)
		)
			pushIssue(
				issues,
				"home_widget_config_number_out_of_range",
				path,
				"Use a supported numeric range.",
			);
	}
	if (Array.isArray(value)) {
		if (contract.min_items !== undefined && value.length < contract.min_items)
			pushIssue(
				issues,
				"home_widget_config_list_too_short",
				path,
				`Use at least ${contract.min_items} items.`,
			);
		if (contract.max_items !== undefined && value.length > contract.max_items)
			pushIssue(
				issues,
				"home_widget_config_list_too_long",
				path,
				`Use at most ${contract.max_items} items.`,
			);
		if (contract.unique_items && new Set<unknown>(value).size !== value.length)
			pushIssue(
				issues,
				"home_widget_config_list_duplicate",
				path,
				"List values must be unique.",
			);
		if (contract.item_enum) {
			value.forEach((entry, index) => {
				if (typeof entry === "string" && !contract.item_enum?.includes(entry))
					pushIssue(
						issues,
						"home_widget_config_list_option_invalid",
						`${path}[${index}]`,
						`Use one of: ${contract.item_enum?.join(", ")}.`,
					);
			});
		}
		if (contract.type === "object_list" && contract.item) {
			const uniqueBy = contract.item.unique_by;
			if (uniqueBy) {
				const seen = new Set<unknown>();
				value.forEach((entry, index) => {
					if (!isRecord(entry)) return;
					const identity = entry[uniqueBy];
					if (identity === undefined || identity === "") return;
					if (seen.has(identity))
						pushIssue(
							issues,
							"home_widget_config_item_id_duplicate",
							`${path}[${index}].${uniqueBy}`,
							`Item ${uniqueBy} values must be unique.`,
						);
					seen.add(identity);
				});
			}
			value.forEach((entry, index) => {
				if (isRecord(entry))
					validateObject(
						entry,
						contract.item as HomeWidgetObjectContract,
						`${path}[${index}]`,
						issues,
					);
			});
		}
	}
}

function validateObject(
	value: Record<string, unknown>,
	contract: HomeWidgetObjectContract,
	path: string,
	issues: HomeWidgetConfigIssue[],
) {
	if (!contract.additional_properties) {
		for (const key of Object.keys(value)) {
			if (Object.hasOwn(contract.fields, key)) continue;
			pushIssue(
				issues,
				"home_widget_config_key_unknown",
				`${path}.${key}`,
				`Config key '${key}' is not advertised by this client. Preserve it when editing an existing widget.`,
				"warning",
			);
		}
	}
	for (const key of contract.required ?? []) {
		if (value[key] !== undefined) continue;
		pushIssue(
			issues,
			"home_widget_config_field_missing",
			`${path}.${key}`,
			`Config field '${key}' is required.`,
		);
	}
	for (const [key, field] of Object.entries(contract.fields)) {
		if (value[key] === undefined) continue;
		validateField(value[key], field, `${path}.${key}`, issues);
	}
	if (contract.unique_by) {
		// A list's item validator receives one item at a time. Uniqueness is checked by its parent.
	}
	validateRequirements(value, contract, path, issues);
}

/** Validate the config of one known Home widget without reading app or data resources. */
export function validateKnownHomeWidgetConfig(
	type: string,
	config: unknown,
	path = "$.config",
): HomeWidgetConfigIssue[] {
	if (!Object.hasOwn(HOME_WIDGET_CONFIG_CONTRACTS, type)) return [];
	if (!isRecord(config)) {
		return [
			{
				severity: "error",
				code: "home_widget_config_invalid",
				path,
				message: "Widget config must be an object.",
			},
		];
	}
	const contract = HOME_WIDGET_CONFIG_CONTRACTS[type as KnownHomeWidgetType];
	const issues: HomeWidgetConfigIssue[] = [];
	validateObject(config, contract, path, issues);

	if (type === "data") {
		const raw = config;
		const normalized = normalizeHomeDataConfig(raw);
		if (
			typeof raw.mode === "string" &&
			["aggregate", "records"].includes(raw.mode) &&
			normalized.mode !== raw.mode
		)
			pushIssue(
				issues,
				"data_mode_incompatible",
				`${path}.mode`,
				`Use mode '${normalized.mode}' with visualization '${normalized.visualization}'.`,
			);
		if (isRecord(raw.queryParams)) {
			for (const key of Object.keys(raw.queryParams)) {
				if (!key.startsWith("__home_")) continue;
				pushIssue(
					issues,
					"data_query_parameter_reserved",
					`${path}.queryParams.${key}`,
					"Saved-query parameter names cannot start with __home_.",
				);
			}
		}
	}

	if (type === "app-embed") {
		const querySources: Array<["query" | "route", string]> = [];
		if (typeof config.query === "string")
			querySources.push(["query", config.query.replace(/^\?/, "")]);
		if (typeof config.route === "string") {
			const queryStart = config.route.indexOf("?");
			if (queryStart >= 0)
				querySources.push(["route", config.route.slice(queryStart + 1)]);
		}
		for (const [field, query] of querySources) {
			for (const key of new URLSearchParams(query).keys()) {
				if (!APP_EMBED_RESERVED_QUERY_KEYS.has(key)) continue;
				pushIssue(
					issues,
					"app_embed_query_key_reserved",
					`${path}.${field}`,
					`Query parameter '${key}' is reserved and would be discarded.`,
				);
			}
		}
	}

	return issues;
}
