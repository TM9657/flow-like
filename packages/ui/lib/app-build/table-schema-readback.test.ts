import { describe, expect, test } from "vitest";
import { schemaFields, tableSchemaIssues } from "./table-schema-readback";

describe("persisted table schema matching", () => {
	test("matches native scalar and UTC millisecond timestamp fields", () => {
		const fields = schemaFields({
			fields: [
				{ name: "title", data_type: "Utf8", nullable: false },
				{
					name: "created_at",
					data_type: { Timestamp: ["Millisecond", "UTC"] },
				},
			],
		});
		expect(
			tableSchemaIssues(fields, [
				{ name: "title", type: "string", nullable: false },
				{ name: "created_at", type: "timestamp:ms:UTC" },
			]),
		).toEqual([]);
	});
	test("requires exact vector width and float32 element type", () => {
		const column = {
			name: "embedding",
			type: "vector" as const,
			vector_size: 1536,
		};
		const field = (type: string, size: number) => [
			{
				name: "embedding",
				data_type: {
					FixedSizeList: [{ name: "item_1536", data_type: type }, size],
				},
			},
		];
		expect(tableSchemaIssues(field("Float32", 1536), [column])).toEqual([]);
		expect(tableSchemaIssues(field("Float32", 768), [column])).toHaveLength(1);
		expect(tableSchemaIssues(field("Utf8", 1536), [column])).toHaveLength(1);
	});
	test("nested type text cannot impersonate a native timestamp", () => {
		expect(
			tableSchemaIssues(
				[
					{
						name: "created_at",
						data_type: { Struct: [{ name: "Millisecond_UTC" }] },
					},
				],
				[{ name: "created_at", type: "timestamp:ms:UTC" }],
			),
		).toHaveLength(1);
	});
	test("missing schemas and unverified nullability fail explicitly", () => {
		expect(
			tableSchemaIssues(schemaFields(null), [{ name: "id", type: "string" }]),
		).toEqual(["Missing table column 'id'."]);
		expect(
			tableSchemaIssues(
				[{ name: "id", type: "Utf8" }],
				[{ name: "id", type: "string", nullable: false }],
			),
		).toEqual(["Column 'id' has different nullability."]);
	});
});
