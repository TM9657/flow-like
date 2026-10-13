/**
 * Platform-wide sensitivity labels: a classification level and a personal-data
 * flag that any resource (table columns, files, variables, …) can carry. Mirrors
 * `flow_like_types::data_classification` in Rust.
 */

/** Least to most sensitive. */
export const DATA_CLASSIFICATIONS = [
	"public",
	"internal",
	"confidential",
	"restricted",
] as const;

export type DataClassification = (typeof DATA_CLASSIFICATIONS)[number];

/** A resource's sensitivity label. Empty means nobody has labelled it. */
export interface IDataSensitivity {
	classification?: DataClassification;
	pii?: boolean;
}

/** Key/value metadata key holding a {@link DataClassification} name. */
export const CLASSIFICATION_METADATA_KEY = "flow-like:classification";
/** Key/value metadata key holding `"true"` when the resource contains personal data. */
export const PII_METADATA_KEY = "flow-like:pii";

const TRUE_TEXT = new Set(["true", "1", "yes"]);

/** The level named `value`, ignoring case and surrounding whitespace. */
export function parseDataClassification(
	value: unknown,
): DataClassification | undefined {
	if (typeof value !== "string") return undefined;
	const name = value.trim().toLowerCase();
	return DATA_CLASSIFICATIONS.find((level) => level === name);
}

const rank = (level: DataClassification | undefined) =>
	level ? DATA_CLASSIFICATIONS.indexOf(level) : -1;

/** Orders least to most sensitive; an unset level sorts below every level. */
export function compareDataClassification(
	a: DataClassification | undefined,
	b: DataClassification | undefined,
): number {
	return rank(a) - rank(b);
}

export function isEmptyDataSensitivity(
	sensitivity: IDataSensitivity | undefined,
): boolean {
	return !sensitivity?.classification && !sensitivity?.pii;
}

/** The label of data derived from all of `items`: the strictest level, PII when any holds it. */
export function combineDataSensitivity(
	...items: readonly (IDataSensitivity | undefined)[]
): IDataSensitivity {
	let classification: DataClassification | undefined;
	let pii = false;
	for (const item of items) {
		if (compareDataClassification(item?.classification, classification) > 0)
			classification = item?.classification;
		pii ||= item?.pii === true;
	}
	return { ...(classification ? { classification } : {}), pii };
}

function isTrueFlag(value: unknown): boolean {
	if (typeof value === "boolean") return value;
	return typeof value === "string" && TRUE_TEXT.has(value.trim().toLowerCase());
}

/** Reads the label from key/value metadata. An unknown classification reads as unset. */
export function sensitivityFromMetadata(
	metadata: Readonly<Record<string, unknown>> | undefined,
): IDataSensitivity {
	const classification = parseDataClassification(
		metadata?.[CLASSIFICATION_METADATA_KEY],
	);
	return {
		...(classification ? { classification } : {}),
		pii: isTrueFlag(metadata?.[PII_METADATA_KEY]),
	};
}
