import { HOME_WIDGET_PRESETS } from "../../../home/catalog";
import type { HomeDataSourceKind, HomeToolIssue } from "./types";

export const HOME_WIDGET_TYPES = new Set(
	HOME_WIDGET_PRESETS.map((preset) => preset.type),
);

export const HOME_VARIANTS = new Set(["card", "borderless", "tinted", "solid"]);

export const DATA_SOURCE_KINDS: HomeDataSourceKind[] = [
	"table",
	"ontology",
	"query",
];

export function stringArg(args: Record<string, unknown>, key: string) {
	return typeof args[key] === "string" ? args[key].trim() : "";
}

export function issue(
	severity: HomeToolIssue["severity"],
	code: string,
	path: string,
	message: string,
): HomeToolIssue {
	return { severity, code, path, message };
}

export function objectRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
