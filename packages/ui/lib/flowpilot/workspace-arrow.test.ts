import { describe, expect, it } from "vitest";
import { projectWorkspaceTableFields } from "./workspace-arrow";

function field(name: string, data_type: unknown) {
	return {
		name,
		data_type,
		nullable: false,
		dict_id: 0,
		dict_is_ordered: false,
		metadata: { secret: "DO_NOT_INDEX_METADATA" },
		default: "DO_NOT_INDEX_DEFAULT",
	};
}

describe("workspace Arrow schema projection", () => {
	it("preserves the nested serde Struct shape while removing every Field metadata map", () => {
		// Shape follows arrow-schema's serde_struct_type test, including metadata on child fields.
		const input = [
			field("person", {
				Struct: [
					field("first_name", "Utf8"),
					field("address", {
						Struct: [field("street", "Utf8"), field("zip", "UInt16")],
					}),
				],
			}),
		];
		const result = projectWorkspaceTableFields(input);
		expect(result).toEqual({
			fields: [
				{
					name: "person",
					nullable: false,
					data_type: {
						Struct: [
							{ name: "first_name", nullable: false, data_type: "Utf8" },
							{
								name: "address",
								nullable: false,
								data_type: {
									Struct: [
										{ name: "street", nullable: false, data_type: "Utf8" },
										{ name: "zip", nullable: false, data_type: "UInt16" },
									],
								},
							},
						],
					},
				},
			],
			truncated: false,
		});
		expect(JSON.stringify(result)).not.toContain("DO_NOT_INDEX");
		expect(JSON.stringify(input)).toContain("DO_NOT_INDEX_METADATA");
	});

	it.each(["List", "LargeList", "ListView", "LargeListView"])(
		"preserves %s geometry child structure without extension metadata",
		(variant) => {
			const result = projectWorkspaceTableFields([
				field("vertices", {
					[variant]: field("point", {
						Struct: [field("x", "Float64"), field("y", "Float64")],
					}),
				}),
			]);
			expect(result.truncated).toBe(false);
			expect(result.fields).toEqual([
				{
					name: "vertices",
					nullable: false,
					data_type: {
						[variant]: {
							name: "point",
							nullable: false,
							data_type: {
								Struct: [
									{ name: "x", nullable: false, data_type: "Float64" },
									{ name: "y", nullable: false, data_type: "Float64" },
								],
							},
						},
					},
				},
			]);
			expect(JSON.stringify(result)).not.toContain("metadata");
		},
	);

	it("retains vector width, dictionary values, map ordering and timestamp timezone", () => {
		const types = [
			{ FixedSizeList: [field("item", "Float32"), 1536] },
			{ Dictionary: ["Int32", { List: field("value", "Utf8") }] },
			{
				Map: [
					field("entries", {
						Struct: [field("key", "Utf8"), field("value", "Int64")],
					}),
					true,
				],
			},
			{ Timestamp: ["Millisecond", "UTC"] },
			{ Timestamp: ["Nanosecond", null] },
			{ Decimal128: [38, -2] },
			{ Duration: "Microsecond" },
			{ FixedSizeBinary: 16 },
		];
		const result = projectWorkspaceTableFields(
			types.map((type, index) => field(`column_${index}`, type)),
		);
		expect(result.truncated).toBe(false);
		const clean = JSON.parse(
			JSON.stringify(types, (key, value) =>
				["metadata", "default", "dict_id", "dict_is_ordered"].includes(key)
					? undefined
					: value,
			),
		);
		expect(
			result.fields.map((entry) => (entry as { data_type: unknown }).data_type),
		).toEqual(clean);
	});

	it("projects Union and RunEndEncoded child Fields recursively", () => {
		const result = projectWorkspaceTableFields([
			field("choice", {
				Union: [
					[
						[0, field("text", "Utf8")],
						[1, field("number", "Int64")],
					],
					"Dense",
				],
			}),
			field("runs", {
				RunEndEncoded: [field("run_ends", "Int32"), field("values", "Utf8")],
			}),
		]);
		expect(result.truncated).toBe(false);
		expect(JSON.stringify(result)).not.toContain("DO_NOT_INDEX");
		expect(result.fields).toEqual([
			{
				name: "choice",
				nullable: false,
				data_type: {
					Union: [
						[
							[0, { name: "text", nullable: false, data_type: "Utf8" }],
							[1, { name: "number", nullable: false, data_type: "Int64" }],
						],
						"Dense",
					],
				},
			},
			{
				name: "runs",
				nullable: false,
				data_type: {
					RunEndEncoded: [
						{ name: "run_ends", nullable: false, data_type: "Int32" },
						{ name: "values", nullable: false, data_type: "Utf8" },
					],
				},
			},
		]);
	});

	it("supports scalar strings and the type alias without inspecting non-contract properties", () => {
		const input = {
			name: "legacy",
			type: "timestamp:ms:UTC",
			nullable: true,
			get metadata() {
				throw new Error("Metadata must not be read.");
			},
			get default() {
				throw new Error("Defaults must not be read.");
			},
		};
		expect(projectWorkspaceTableFields([input])).toEqual({
			fields: [{ name: "legacy", type: "timestamp:ms:UTC", nullable: true }],
			truncated: false,
		});
	});

	it.each([
		{ UnknownVariant: { token: "DO_NOT_INDEX" } },
		{ Struct: [], List: field("ambiguous", "Utf8") },
		{ Timestamp: ["Millisecond", { token: "DO_NOT_INDEX" }] },
		{ List: "DO_NOT_INDEX" },
		{ FixedSizeList: [field("item", "Float32"), "DO_NOT_INDEX"] },
		{ Map: [field("entry", "Utf8"), { token: "DO_NOT_INDEX" }] },
		{ Dictionary: ["Int32", { unknown: "DO_NOT_INDEX" }] },
	])(
		"omits unsupported or malformed type parameters and marks their coverage incomplete",
		(data_type) => {
			const result = projectWorkspaceTableFields([field("column", data_type)]);
			expect(result).toEqual({
				fields: [{ name: "column", nullable: false }],
				truncated: true,
			});
			expect(JSON.stringify(result)).not.toContain("DO_NOT_INDEX");
		},
	);

	it("bounds wide schemas, nested fields and aggregate work, including cycles", () => {
		const wide = projectWorkspaceTableFields(
			Array.from({ length: 300 }, (_, index) => field(`f${index}`, "Utf8")),
		);
		expect(wide.fields).toHaveLength(256);
		expect(wide.truncated).toBe(true);
		let nested: unknown = "Utf8";
		for (let depth = 0; depth < 40; depth++)
			nested = { List: field("item", nested) };
		expect(projectWorkspaceTableFields([field("deep", nested)]).truncated).toBe(
			true,
		);
		const sharedChildren = Array.from({ length: 256 }, (_, index) =>
			field(`child${index}`, "Float64"),
		);
		expect(
			projectWorkspaceTableFields(
				Array.from({ length: 256 }, (_, index) =>
					field(`parent${index}`, { Struct: sharedChildren }),
				),
			).truncated,
		).toBe(true);
		const cycle: { name: string; data_type?: unknown } = { name: "cycle" };
		cycle.data_type = { List: cycle };
		expect(projectWorkspaceTableFields([cycle])).toEqual({
			fields: [{ name: "cycle" }],
			truncated: true,
		});
	});

	it("marks oversized names and types incomplete instead of inventing shortened identifiers", () => {
		expect(
			projectWorkspaceTableFields([
				{ name: "x".repeat(257), data_type: "y".repeat(129) },
			]),
		).toEqual({ fields: [], truncated: true });
		expect(projectWorkspaceTableFields([null, 3, "bad"])).toEqual({
			fields: [],
			truncated: true,
		});
	});
});
