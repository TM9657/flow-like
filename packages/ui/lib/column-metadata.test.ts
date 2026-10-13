import { describe, expect, test } from "bun:test";

import {
	COLUMN_DESCRIPTION_KEY,
	COLUMN_DISPLAY_NAME_KEY,
	COLUMN_READ_ATTRIBUTES_KEY,
	columnMetadataFromField,
	isColumnReadRestricted,
	isEmptyColumnMetadata,
	normalizeColumnMetadata,
	parseColumnMetadataInput,
	parseReadAttributes,
	sameColumnMetadata,
} from "./column-metadata";
import {
	CLASSIFICATION_METADATA_KEY,
	PII_METADATA_KEY,
} from "./data-classification";

describe("columnMetadataFromField", () => {
	test("reads every key of a labelled column", () => {
		expect(
			columnMetadataFromField({
				[COLUMN_DISPLAY_NAME_KEY]: " Annual salary ",
				[COLUMN_DESCRIPTION_KEY]: "Gross, EUR",
				[CLASSIFICATION_METADATA_KEY]: "confidential",
				[PII_METADATA_KEY]: "true",
				[COLUMN_READ_ATTRIBUTES_KEY]: '["hr","finance"]',
				"lance-schema:unenforced-primary-key:position": "1",
			}),
		).toEqual({
			display_name: "Annual salary",
			description: "Gross, EUR",
			classification: "confidential",
			pii: true,
			read_attributes: ["hr", "finance"],
		});
	});

	test("leaves out what is unset, blank or unknown", () => {
		expect(
			columnMetadataFromField({
				[COLUMN_DISPLAY_NAME_KEY]: "  ",
				[CLASSIFICATION_METADATA_KEY]: "secret",
				[PII_METADATA_KEY]: "false",
				[COLUMN_READ_ATTRIBUTES_KEY]: "[]",
			}),
		).toEqual({});
		expect(columnMetadataFromField(undefined)).toEqual({});
	});
});

describe("parseReadAttributes", () => {
	test("accepts a JSON array, comma text and quoted comma text", () => {
		expect(parseReadAttributes('["hr", " finance ", "hr", ""]')).toEqual([
			"hr",
			"finance",
		]);
		expect(parseReadAttributes("hr, finance,,eu")).toEqual([
			"hr",
			"finance",
			"eu",
		]);
		expect(parseReadAttributes('["hr", finance]')).toEqual(["hr", "finance"]);
		expect(parseReadAttributes(["hr", 3, " eu "])).toEqual(["hr", "eu"]);
	});

	test("anything else holds no attributes", () => {
		for (const value of [undefined, null, 7, "", "   ", "[]"]) {
			expect(parseReadAttributes(value)).toEqual([]);
		}
	});
});

describe("normalizeColumnMetadata", () => {
	test("spells out every part a replacing write stores", () => {
		expect(
			normalizeColumnMetadata({
				display_name: "  ",
				description: " Gross ",
				read_attributes: [" hr ", "hr", ""],
			}),
		).toEqual({ description: "Gross", pii: false, read_attributes: ["hr"] });
		expect(normalizeColumnMetadata(undefined)).toEqual({
			pii: false,
			read_attributes: [],
		});
	});

	test("compares metadata by what a write would store", () => {
		expect(
			sameColumnMetadata(
				{ display_name: "Salary ", read_attributes: ["hr", "hr"] },
				{ display_name: "Salary", pii: false, read_attributes: ["hr"] },
			),
		).toBe(true);
		expect(
			sameColumnMetadata({ classification: "internal" }, { pii: false }),
		).toBe(false);
	});
});

describe("column restriction and emptiness", () => {
	test("a column with attributes is read-restricted", () => {
		expect(isColumnReadRestricted({ read_attributes: ["hr"] })).toBe(true);
		expect(isColumnReadRestricted({ read_attributes: [" "] })).toBe(false);
		expect(isColumnReadRestricted(undefined)).toBe(false);
	});

	test("empty metadata carries no label", () => {
		expect(isEmptyColumnMetadata({ pii: false, read_attributes: [] })).toBe(
			true,
		);
		expect(isEmptyColumnMetadata({ description: "x" })).toBe(false);
		expect(isEmptyColumnMetadata({ pii: true })).toBe(false);
	});
});

describe("parseColumnMetadataInput", () => {
	test("normalizes valid input", () => {
		expect(
			parseColumnMetadataInput({
				display_name: "Salary",
				classification: "Restricted",
				pii: true,
				read_attributes: ["hr", " hr "],
			}),
		).toEqual({
			display_name: "Salary",
			classification: "restricted",
			pii: true,
			read_attributes: ["hr"],
		});
		expect(parseColumnMetadataInput({ classification: "", pii: null })).toEqual(
			{ pii: false, read_attributes: [] },
		);
	});

	test("refuses values of the wrong shape with the field named", () => {
		const cases: [unknown, RegExp][] = [
			["salary", /must be an object/],
			[{ classification: "secret" }, /classification must be one of public/],
			[{ pii: "yes" }, /pii must be true or false/],
			[{ read_attributes: "hr" }, /read_attributes must be an array/],
			[{ read_attributes: ["hr", 3] }, /read_attributes must be an array/],
			[{ display_name: 3 }, /display_name must be a string/],
			[{ name: "Salary" }, /unknown field 'name'/],
		];
		for (const [input, message] of cases) {
			expect(() => parseColumnMetadataInput(input)).toThrow(message);
		}
	});
});
