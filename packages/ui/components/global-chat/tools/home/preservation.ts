import { stableStringify } from "../../../../lib/stable-stringify";
import type { IHomeLayout, IHomeWidget } from "../../../home/types";
import { HOME_WIDGET_TYPES, issue } from "./shared";
import type { HomeToolIssue } from "./types";
import { HOME_WIDGET_CONFIG_CONTRACTS } from "./widget-contracts/definitions";
import type { HomeWidgetObjectContract } from "./widget-contracts/types";

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
