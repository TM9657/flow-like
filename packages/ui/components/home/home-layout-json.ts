import {
	MAX_HOME_LAYOUT_BYTES,
	MAX_HOME_WIDGETS,
	homeLayoutByteLength,
	normalizeHomeLayout,
} from "./home-layout";
import type { IHomeLayout } from "./types";

export type HomeLayoutJsonResult =
	| { ok: true; layout: IHomeLayout }
	| { ok: false; error: string };

export type FormattedHomeLayoutJsonResult =
	| { ok: true; json: string }
	| { ok: false; error: string };

type JsonDocumentResult =
	| { ok: true; value: unknown }
	| { ok: false; error: string };

const textEncoder = new TextEncoder();

export function serializeHomeLayout(layout: IHomeLayout) {
	return JSON.stringify(layout, null, 2);
}

function parseJsonDocument(source: string): JsonDocumentResult {
	let value: unknown;
	try {
		value = JSON.parse(source);
	} catch (error) {
		return {
			ok: false,
			error: `Invalid JSON: ${
				error instanceof Error ? error.message : "Check the document syntax."
			}`,
		};
	}

	const pending = [value];
	while (pending.length) {
		const current = pending.pop();
		if (typeof current === "number" && !Number.isFinite(current)) {
			return {
				ok: false,
				error:
					"JSON numbers must be finite. Replace values such as 1e400 with a finite number.",
			};
		}
		if (Array.isArray(current)) pending.push(...current);
		else if (current && typeof current === "object") {
			pending.push(...Object.values(current));
		}
	}

	return { ok: true, value };
}

export function formatHomeLayoutJson(
	source: string,
): FormattedHomeLayoutJsonResult {
	const document = parseJsonDocument(source);
	return document.ok
		? { ok: true, json: JSON.stringify(document.value, null, 2) }
		: document;
}

function textLimitError(
	value: string | undefined,
	label: string,
	maximum: number,
	required = false,
) {
	if (value === undefined) return required ? `${label} is required.` : null;
	const bytes = textEncoder.encode(value).byteLength;
	if (required && bytes === 0)
		return `${label} must contain 1 to ${maximum} bytes.`;
	return bytes > maximum ? `${label} must not exceed ${maximum} bytes.` : null;
}

function persistenceError(layout: IHomeLayout) {
	const layoutError =
		textLimitError(layout.title, "Layout title", 256) ??
		textLimitError(layout.description, "Layout description", 2000);
	if (layoutError) return layoutError;

	for (const [index, widget] of layout.widgets.entries()) {
		const prefix = `Widget ${index + 1}`;
		const widgetError =
			textLimitError(widget.id, `${prefix} id`, 128, true) ??
			textLimitError(widget.type, `${prefix} type`, 80, true) ??
			textLimitError(widget.title, `${prefix} title`, 256) ??
			textLimitError(widget.description, `${prefix} description`, 2000) ??
			textLimitError(
				widget.appearance.variant,
				`${prefix} appearance variant`,
				80,
				true,
			) ??
			textLimitError(
				widget.appearance.accent,
				`${prefix} appearance accent`,
				128,
				true,
			);
		if (widgetError) return widgetError;
	}

	return null;
}

function jsonValuesEqual(left: unknown, right: unknown) {
	const pending: Array<[unknown, unknown]> = [[left, right]];
	while (pending.length) {
		const pair = pending.pop();
		if (!pair) break;
		const [currentLeft, currentRight] = pair;
		if (Object.is(currentLeft, currentRight)) continue;
		if (
			!currentLeft ||
			!currentRight ||
			typeof currentLeft !== "object" ||
			typeof currentRight !== "object"
		)
			return false;
		if (Array.isArray(currentLeft) || Array.isArray(currentRight)) {
			if (
				!Array.isArray(currentLeft) ||
				!Array.isArray(currentRight) ||
				currentLeft.length !== currentRight.length
			)
				return false;
			for (let index = 0; index < currentLeft.length; index++)
				pending.push([currentLeft[index], currentRight[index]]);
			continue;
		}
		const leftRecord = currentLeft as Record<string, unknown>;
		const rightRecord = currentRight as Record<string, unknown>;
		const leftKeys = Object.keys(leftRecord);
		const rightKeys = Object.keys(rightRecord);
		if (
			leftKeys.length !== rightKeys.length ||
			leftKeys.some((key) => !Object.hasOwn(rightRecord, key))
		)
			return false;
		for (const key of leftKeys)
			pending.push([leftRecord[key], rightRecord[key]]);
	}
	return true;
}

export function homeLayoutsEqual(left: IHomeLayout, right: IHomeLayout) {
	try {
		return jsonValuesEqual(
			JSON.parse(JSON.stringify(left)),
			JSON.parse(JSON.stringify(right)),
		);
	} catch {
		return false;
	}
}

export function parseHomeLayoutJson(source: string): HomeLayoutJsonResult {
	const document = parseJsonDocument(source);
	if (!document.ok) return document;

	const layout = normalizeHomeLayout(document.value);
	if (!layout) {
		return {
			ok: false,
			error: `Expected a version 1 home layout with a widgets array and no more than ${MAX_HOME_WIDGETS} widgets. Each widget needs a non-empty, unique id and type.`,
		};
	}

	if (homeLayoutByteLength(layout) > MAX_HOME_LAYOUT_BYTES) {
		return {
			ok: false,
			error: "This layout exceeds the 128 KiB save limit.",
		};
	}

	const validationError = persistenceError(layout);
	if (validationError) return { ok: false, error: validationError };

	return { ok: true, layout };
}
