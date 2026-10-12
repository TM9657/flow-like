import { describe, expect, test } from "bun:test";

import {
	CLASSIFICATION_METADATA_KEY,
	DATA_CLASSIFICATIONS,
	PII_METADATA_KEY,
	combineDataSensitivity,
	compareDataClassification,
	isEmptyDataSensitivity,
	parseDataClassification,
	sensitivityFromMetadata,
} from "./data-classification";

describe("parseDataClassification", () => {
	test("accepts every level regardless of case and whitespace", () => {
		expect(parseDataClassification(" Confidential ")).toBe("confidential");
		expect(parseDataClassification("RESTRICTED")).toBe("restricted");
		for (const level of DATA_CLASSIFICATIONS) {
			expect(parseDataClassification(level)).toBe(level);
		}
	});

	test("reads anything else as unset", () => {
		for (const value of ["secret", "", 3, null, undefined, ["public"]]) {
			expect(parseDataClassification(value)).toBeUndefined();
		}
	});
});

describe("compareDataClassification", () => {
	test("orders least to most sensitive with unset lowest", () => {
		const levels = [
			"restricted",
			"internal",
			"public",
			"confidential",
		] as const;
		expect(levels.toSorted(compareDataClassification)).toEqual([
			"public",
			"internal",
			"confidential",
			"restricted",
		]);
		expect(compareDataClassification(undefined, "public")).toBeLessThan(0);
		expect(compareDataClassification("public", undefined)).toBeGreaterThan(0);
		expect(compareDataClassification("internal", "internal")).toBe(0);
	});
});

describe("combineDataSensitivity", () => {
	test("keeps the stricter level and PII when any part holds it", () => {
		expect(
			combineDataSensitivity(
				{ classification: "internal" },
				{ classification: "confidential", pii: false },
				undefined,
				{ pii: true },
			),
		).toEqual({ classification: "confidential", pii: true });
	});

	test("combining nothing is an empty label", () => {
		const empty = combineDataSensitivity();
		expect(empty).toEqual({ pii: false });
		expect(isEmptyDataSensitivity(empty)).toBe(true);
		expect(isEmptyDataSensitivity({ pii: true })).toBe(false);
		expect(isEmptyDataSensitivity({ classification: "public" })).toBe(false);
	});
});

describe("sensitivityFromMetadata", () => {
	test("reads the two metadata keys", () => {
		expect(
			sensitivityFromMetadata({
				[CLASSIFICATION_METADATA_KEY]: "Restricted",
				[PII_METADATA_KEY]: "true",
				other: "kept elsewhere",
			}),
		).toEqual({ classification: "restricted", pii: true });
	});

	test("accepts the PII spellings Rust writes and reads", () => {
		for (const value of ["true", "TRUE", " 1 ", "yes", true]) {
			expect(sensitivityFromMetadata({ [PII_METADATA_KEY]: value }).pii).toBe(
				true,
			);
		}
		for (const value of ["false", "no", "", 1, undefined]) {
			expect(sensitivityFromMetadata({ [PII_METADATA_KEY]: value }).pii).toBe(
				false,
			);
		}
	});

	test("an unknown level or missing metadata reads as unlabelled", () => {
		expect(
			sensitivityFromMetadata({ [CLASSIFICATION_METADATA_KEY]: "secret" }),
		).toEqual({ pii: false });
		expect(sensitivityFromMetadata(undefined)).toEqual({ pii: false });
	});
});
