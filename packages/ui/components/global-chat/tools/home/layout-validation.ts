import { stableStringify } from "../../../../lib/stable-stringify";
import { HOME_ACCENTS } from "../../../home/home-appearance";
import {
	homeLayoutByteLength,
	minimumHomeWidgetRows,
} from "../../../home/home-layout";
import {
	homeLayoutFingerprint,
	parseHomeLayoutJson,
} from "../../../home/home-layout-json";
import type { IHomeWidget } from "../../../home/types";
import { HOME_VARIANTS, HOME_WIDGET_TYPES, issue } from "./shared";
import type { HomeLayoutValidationResult, HomeToolIssue } from "./types";
import { validateKnownHomeWidgetConfig } from "./widget-contracts/validation";

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
