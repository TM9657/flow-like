import {
	CLASSIFICATION_METADATA_KEY,
	DATA_CLASSIFICATIONS,
	type IDataSensitivity,
	PII_METADATA_KEY,
	isEmptyDataSensitivity,
	parseDataClassification,
	sensitivityFromMetadata,
} from "./data-classification";
import { isRecord } from "./response-shape";

/**
 * What a table column means and who may read it, stored as Lance field metadata.
 * Mirrors `flow_like_storage::contracts::column_metadata` in Rust.
 */
export interface IColumnMetadata extends IDataSensitivity {
	/** Human-readable name shown in place of the column name. */
	display_name?: string;
	description?: string;
	/**
	 * Role attributes allowed to read the column. Empty: everyone who may read
	 * the table. Otherwise only admins and roles holding one of them see it.
	 */
	read_attributes?: string[];
}

export const COLUMN_DISPLAY_NAME_KEY = "flow-like:display-name";
export const COLUMN_DESCRIPTION_KEY = "flow-like:description";
export const COLUMN_READ_ATTRIBUTES_KEY = "flow-like:read-attributes";
export const COLUMN_METADATA_KEYS = [
	COLUMN_DISPLAY_NAME_KEY,
	COLUMN_DESCRIPTION_KEY,
	CLASSIFICATION_METADATA_KEY,
	PII_METADATA_KEY,
	COLUMN_READ_ATTRIBUTES_KEY,
] as const;

export const MAX_COLUMN_DISPLAY_NAME_CHARS = 128;
export const MAX_COLUMN_DESCRIPTION_CHARS = 4096;
export const MAX_COLUMN_READ_ATTRIBUTES = 64;
export const MAX_COLUMN_ATTRIBUTE_CHARS = 128;

const COLUMN_METADATA_FIELDS = [
	"display_name",
	"description",
	"classification",
	"pii",
	"read_attributes",
] as const;

function text(value: unknown): string | undefined {
	if (typeof value !== "string") return undefined;
	return value.trim() || undefined;
}

/** Trimmed, non-blank attributes without duplicates, in first-seen order. */
export function normalizeAttributes(values: readonly unknown[]): string[] {
	const attributes = new Set<string>();
	for (const value of values) {
		const attribute = text(value);
		if (attribute) attributes.add(attribute);
	}
	return [...attributes];
}

function jsonStringArray(raw: string): string[] | undefined {
	try {
		const parsed: unknown = JSON.parse(raw);
		return Array.isArray(parsed) &&
			parsed.every((item) => typeof item === "string")
			? parsed
			: undefined;
	} catch {
		return undefined;
	}
}

function trimDelimiters(
	value: string,
	leading: string,
	trailing = leading,
): string {
	let start = 0;
	let end = value.length;
	while (start < end && value[start] === leading) start++;
	while (end > start && value[end - 1] === trailing) end--;
	return value.slice(start, end);
}

/** `flow-like:read-attributes` holds a JSON string array; comma-separated text is read too. */
export function parseReadAttributes(value: unknown): string[] {
	if (Array.isArray(value)) return normalizeAttributes(value);
	if (typeof value !== "string") return [];
	const raw = value.trim();
	const listed =
		jsonStringArray(raw) ??
		trimDelimiters(raw, "[", "]")
			.split(",")
			.map((attribute) => trimDelimiters(attribute.trim(), '"'));
	return normalizeAttributes(listed);
}

/** The column metadata a field's Arrow metadata carries; unset parts are left out. */
export function columnMetadataFromField(
	metadata: Readonly<Record<string, unknown>> | undefined,
): IColumnMetadata {
	const displayName = text(metadata?.[COLUMN_DISPLAY_NAME_KEY]);
	const description = text(metadata?.[COLUMN_DESCRIPTION_KEY]);
	const { classification, pii } = sensitivityFromMetadata(metadata);
	const readAttributes = parseReadAttributes(
		metadata?.[COLUMN_READ_ATTRIBUTES_KEY],
	);
	return {
		...(displayName ? { display_name: displayName } : {}),
		...(description ? { description } : {}),
		...(classification ? { classification } : {}),
		...(pii ? { pii } : {}),
		...(readAttributes.length > 0 ? { read_attributes: readAttributes } : {}),
	};
}

/**
 * The complete metadata a write stores: values trimmed, blanks dropped,
 * attributes de-duplicated. Writes replace every part, so `pii` and
 * `read_attributes` are always spelled out.
 */
export function normalizeColumnMetadata(
	metadata: IColumnMetadata | undefined,
): IColumnMetadata {
	const displayName = text(metadata?.display_name);
	const description = text(metadata?.description);
	const classification = parseDataClassification(metadata?.classification);
	return {
		...(displayName ? { display_name: displayName } : {}),
		...(description ? { description } : {}),
		...(classification ? { classification } : {}),
		pii: metadata?.pii === true,
		read_attributes: normalizeAttributes(metadata?.read_attributes ?? []),
	};
}

export function sameColumnMetadata(
	a: IColumnMetadata | undefined,
	b: IColumnMetadata | undefined,
): boolean {
	return (
		JSON.stringify(normalizeColumnMetadata(a)) ===
		JSON.stringify(normalizeColumnMetadata(b))
	);
}

/** Readable only by admins and roles holding one of its read attributes. */
export function isColumnReadRestricted(
	metadata: IColumnMetadata | undefined,
): boolean {
	return normalizeAttributes(metadata?.read_attributes ?? []).length > 0;
}

export function isEmptyColumnMetadata(
	metadata: IColumnMetadata | undefined,
): boolean {
	return (
		!text(metadata?.display_name) &&
		!text(metadata?.description) &&
		isEmptyDataSensitivity(metadata) &&
		!isColumnReadRestricted(metadata)
	);
}

function invalidField(field: string, expected: string, value: unknown) {
	return new Error(
		`column_metadata.${field} must be ${expected}; got ${JSON.stringify(value)}.`,
	);
}

function optionalString(record: Record<string, unknown>, field: string) {
	const value = record[field];
	if (value == null) return undefined;
	if (typeof value === "string") return value;
	throw invalidField(field, "a string", value);
}

function optionalClassification(record: Record<string, unknown>) {
	const value = optionalString(record, "classification");
	const classification = parseDataClassification(value);
	if (text(value) && !classification)
		throw invalidField(
			"classification",
			`one of ${DATA_CLASSIFICATIONS.join(", ")}`,
			value,
		);
	return classification;
}

function optionalFlag(record: Record<string, unknown>, field: string) {
	const value = record[field];
	if (value == null) return false;
	if (typeof value === "boolean") return value;
	throw invalidField(field, "true or false", value);
}

function optionalStrings(record: Record<string, unknown>, field: string) {
	const value = record[field];
	if (value == null) return [];
	if (
		Array.isArray(value) &&
		value.every((item): item is string => typeof item === "string")
	)
		return value;
	throw invalidField(field, "an array of strings", value);
}

/**
 * Validates column metadata that crossed a trust boundary (a tool call, an
 * import) and returns it normalized. Unknown fields are refused so a misspelt
 * one is not silently cleared by the replacing write.
 */
export function parseColumnMetadataInput(value: unknown): IColumnMetadata {
	const fields = COLUMN_METADATA_FIELDS.join(", ");
	if (!isRecord(value))
		throw new Error(`column_metadata must be an object with ${fields}.`);
	const unknown = Object.keys(value).filter(
		(field) => !(COLUMN_METADATA_FIELDS as readonly string[]).includes(field),
	);
	if (unknown.length > 0)
		throw new Error(
			`column_metadata has unknown field ${unknown.map((field) => `'${field}'`).join(", ")}; use ${fields}.`,
		);
	return normalizeColumnMetadata({
		display_name: optionalString(value, "display_name"),
		description: optionalString(value, "description"),
		classification: optionalClassification(value),
		pii: optionalFlag(value, "pii"),
		read_attributes: optionalStrings(value, "read_attributes"),
	});
}
