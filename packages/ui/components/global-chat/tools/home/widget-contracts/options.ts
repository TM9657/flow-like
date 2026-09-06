import { APP_CATEGORY_ORDER } from "../../../../../lib/category-meta";
import {
	HOME_APP_RENDERINGS,
	HOME_MODEL_RENDERINGS,
	HOME_PACKAGE_RENDERINGS,
} from "../../../../home/home-content/config";
import { HOME_DATA_VISUALIZATIONS } from "../../../../home/home-data-query";

export const DATA_VISUALIZATIONS = HOME_DATA_VISUALIZATIONS.map(([id]) => id);

export const APP_RENDERINGS = HOME_APP_RENDERINGS.map(([id]) => id);

export const MODEL_RENDERINGS = HOME_MODEL_RENDERINGS.map(([id]) => id);

export const PACKAGE_RENDERINGS = HOME_PACKAGE_RENDERINGS.map(([id]) => id);

export const APP_CATEGORIES = ["", ...APP_CATEGORY_ORDER];

export const APP_SOURCES = [
	"library",
	"recent",
	"favorites",
	"manual",
	"new",
	"popular",
];

export const DISCOVERY_APP_SOURCES = ["new", "popular", "library", "manual"];

export const INFORMATION_MODES = [
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

export const QUICK_ACTION_IDS = [
	"create",
	"import",
	"library",
	"packages",
	"explore",
	"learn",
];

export const ACTIVITY_DAYS = [1, 7, 30];

export const NOTIFICATION_TYPES = ["all", "WORKFLOW", "SYSTEM"];

export const RUN_STAT_METRICS = [
	"overview",
	"executions",
	"ai",
	"embeddings",
	"errors",
	"duration",
];

export const APP_EMBED_RESERVED_QUERY_KEYS = new Set([
	"id",
	"route",
	"eventId",
	"__proto__",
	"constructor",
	"prototype",
]);

export const RECORD_ONLY_VISUALIZATIONS = [
	"kanban",
	"record",
	"graph",
	"scatter",
	"timeline",
	"recordcalendar",
	"comparison",
];

export const EITHER_MODE_VISUALIZATIONS = ["table", "list", "cards"];

export const AGGREGATE_ONLY_VISUALIZATIONS = DATA_VISUALIZATIONS.filter(
	(value) =>
		!RECORD_ONLY_VISUALIZATIONS.includes(value) &&
		!EITHER_MODE_VISUALIZATIONS.includes(value),
);
