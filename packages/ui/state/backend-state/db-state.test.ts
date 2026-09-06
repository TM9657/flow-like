import { describe, expect, test } from "bun:test";
import { IIndexType, indexTypeToString, parseIndexType } from "./db-state";

describe("database index compatibility", () => {
	test("keeps the original persisted numeric values", () => {
		const persistedTypes = ["FullText", "BTree", "Bitmap", "LabelList", "Auto"];
		for (const [value, name] of persistedTypes.entries()) {
			expect(indexTypeToString(value)).toBe(name);
			expect(parseIndexType(name)).toBe(value);
			expect(parseIndexType(value)).toBe(value);
		}
	});

	test.each([
		["full_text", IIndexType.FullText],
		["Full Text", IIndexType.FullText],
		["FTS", IIndexType.FullText],
		["INVERTED", IIndexType.FullText],
		["B-Tree", IIndexType.BTree],
		["label_list", IIndexType.LabelList],
		["VECTOR", IIndexType.Vector],
		["FM", IIndexType.Fm],
		["IVF_FLAT", IIndexType.IvfFlat],
		["IVF_PQ", IIndexType.IvfPq],
		["IVF_SQ", IIndexType.IvfSq],
		["IVF_RQ", IIndexType.IvfRq],
		["IVF_HNSW_FLAT", IIndexType.IvfHnswFlat],
		["IVF_HNSW_PQ", IIndexType.IvfHnswPq],
		[" IVF HNSW SQ ", IIndexType.IvfHnswSq],
		["NGRAM", IIndexType.NGram],
		["ZONE_MAP", IIndexType.ZoneMap],
		["BLOOM_FILTER", IIndexType.BloomFilter],
		["RTREE", IIndexType.RTree],
	] as const)("parses node and user input %s", (value, expected) => {
		expect(parseIndexType(value)).toBe(expected);
	});

	test("retains Auto for missing and unrecognized values", () => {
		for (const value of [undefined, null, "", "unknown", -1, 999]) {
			expect(parseIndexType(value)).toBe(IIndexType.Auto);
		}
		expect(indexTypeToString(999 as IIndexType)).toBe("Auto");
	});
});
