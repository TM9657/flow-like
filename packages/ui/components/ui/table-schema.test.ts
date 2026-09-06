import { describe, expect, test } from "bun:test";
import { IIndexType } from "../../state/backend-state/db-state";
import { getIndexTypeOptions } from "./table-schema";

function optionsFor(columnType?: string): IIndexType[] {
	return getIndexTypeOptions(columnType).map((option) => option.type);
}

describe("index options for a column", () => {
	test.each(["date", "date32", "timestamp", "timestamp_ms", "int64", "number"])(
		"offers range and equality indexes on %s columns",
		(columnType) => {
			const options = optionsFor(columnType);
			expect(options).toContain(IIndexType.Auto);
			expect(options).toContain(IIndexType.BTree);
			expect(options).toContain(IIndexType.ZoneMap);
			expect(options).toContain(IIndexType.BloomFilter);
			expect(options).not.toContain(IIndexType.FullText);
			expect(options).not.toContain(IIndexType.IvfPq);
		},
	);

	test("offers text substring indexes on string columns", () => {
		const options = optionsFor("string");
		expect(options).toContain(IIndexType.FullText);
		expect(options).toContain(IIndexType.Fm);
		expect(options).toContain(IIndexType.NGram);
		expect(options).toContain(IIndexType.ZoneMap);
		expect(options).not.toContain(IIndexType.RTree);
	});

	test("lets users select every supported vector algorithm", () => {
		expect(optionsFor("vector")).toEqual([
			IIndexType.Auto,
			IIndexType.Vector,
			IIndexType.IvfFlat,
			IIndexType.IvfPq,
			IIndexType.IvfSq,
			IIndexType.IvfRq,
			IIndexType.IvfHnswFlat,
			IIndexType.IvfHnswPq,
			IIndexType.IvfHnswSq,
		]);
	});

	test("offers spatial indexes only for verified geometry columns", () => {
		expect(optionsFor("geometry")).toEqual([IIndexType.Auto, IIndexType.RTree]);
		expect(optionsFor("object")).not.toContain(IIndexType.RTree);
		expect(optionsFor("array")).not.toContain(IIndexType.RTree);
		expect(optionsFor("unsupported-geometry")).not.toContain(IIndexType.RTree);
		expect(optionsFor("array")).toContain(IIndexType.LabelList);
	});

	test("offers FM substring indexes on binary columns", () => {
		expect(optionsFor("binary")).toContain(IIndexType.BTree);
		expect(optionsFor("binary")).toContain(IIndexType.Bitmap);
		expect(optionsFor("binary")).toContain(IIndexType.Fm);
		expect(optionsFor("binary")).not.toContain(IIndexType.FullText);
	});

	test("keeps boolean options within the supported index types", () => {
		expect(optionsFor("boolean")).toContain(IIndexType.Bitmap);
		expect(optionsFor("boolean")).not.toContain(IIndexType.BloomFilter);
	});

	test("keeps general choices available when a column kind is unknown", () => {
		expect(optionsFor("unknown")).toEqual(optionsFor(undefined));
		expect(optionsFor("unknown")).not.toContain(IIndexType.RTree);
		expect(optionsFor("unknown")).toContain(IIndexType.IvfHnswSq);
	});

	test("does not offer vector training on a new empty table", () => {
		expect(getIndexTypeOptions(undefined, "scalar")).not.toContainEqual(
			expect.objectContaining({ type: IIndexType.IvfPq }),
		);
	});
});
