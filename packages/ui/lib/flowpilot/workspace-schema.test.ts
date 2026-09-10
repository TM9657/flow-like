import { describe, expect, it } from "vitest";
import { projectWorkspaceSchema } from "./workspace-schema";

describe("workspace schema projection", () => {
	it("keeps the input contract while removing instance defaults and examples", () => {
		const input = {
			$schema: "https://json-schema.org/draft/2020-12/schema",
			title: "Invoice request",
			description: "Find invoices for a customer",
			type: "object",
			properties: {
				customer_id: {
					type: "string",
					description: "Customer ID",
					default: "PRIVATE_CUSTOMER",
				},
				requested_at: {
					type: "string",
					format: "date-time",
					examples: ["PRIVATE_DATE"],
				},
				lines: {
					type: "array",
					items: {
						type: "object",
						properties: { amount: { type: "number" } },
						required: ["amount"],
					},
				},
			},
			required: ["customer_id"],
			additionalProperties: false,
			default: { customer_id: "PRIVATE_DEFAULT" },
		};
		const result = projectWorkspaceSchema(JSON.stringify(input));
		expect(result.truncated).toBe(false);
		expect(result.schema).toMatchObject({
			type: "object",
			properties: {
				customer_id: { type: "string", description: "Customer ID" },
				requested_at: { type: "string", format: "date-time" },
				lines: { items: { required: ["amount"] } },
			},
			required: ["customer_id"],
			additionalProperties: false,
		});
		expect(JSON.stringify(result)).not.toContain("PRIVATE_");
		expect(input.properties.customer_id.default).toBe("PRIVATE_CUSTOMER");
	});

	it("retains only reachable local definitions and terminates recursive references", () => {
		const result = projectWorkspaceSchema({
			type: "object",
			properties: { root: { $ref: "#/$defs/Node" } },
			$defs: {
				Node: {
					type: "object",
					properties: {
						name: { type: "string" },
						child: { $ref: "#/$defs/Node" },
					},
				},
				Unused: { type: "string", description: "UNREACHABLE_DEFINITION" },
			},
		});
		expect(result.truncated).toBe(false);
		expect(result.schema).toEqual({
			type: "object",
			properties: { root: { $ref: "#/$defs/ref_0" } },
			$defs: {
				ref_0: {
					type: "object",
					properties: {
						name: { type: "string" },
						child: { $ref: "#/$defs/ref_0" },
					},
				},
			},
		});
		expect(JSON.stringify(result)).not.toContain("UNREACHABLE_DEFINITION");
	});

	it("resolves escaped JSON Pointers and boolean schemas without fetching references", () => {
		const result = projectWorkspaceSchema({
			allOf: [{ $ref: "#/definitions/a~1b~0c" }, { $ref: "#/definitions/no" }],
			definitions: { "a/b~c": { type: "string" }, no: false },
		});
		expect(result).toEqual({
			schema: {
				allOf: [{ $ref: "#/$defs/ref_0" }, { $ref: "#/$defs/ref_1" }],
				$defs: { ref_0: { type: "string" }, ref_1: false },
			},
			truncated: false,
		});
		expect(projectWorkspaceSchema(false)).toEqual({
			schema: false,
			truncated: false,
		});
		expect(projectWorkspaceSchema(true)).toEqual({
			schema: true,
			truncated: false,
		});
	});

	it("marks omitted constraints and references incomplete without copying their values", () => {
		const result = projectWorkspaceSchema({
			type: "string",
			enum: ["PRIVATE_ENUM"],
			const: "PRIVATE_CONST",
			pattern: "PRIVATE_PATTERN",
			minLength: 12,
			$ref: "https://private.example/schema?token=PRIVATE_REF",
		});
		expect(result).toEqual({ schema: { type: "string" }, truncated: true });
		expect(JSON.stringify(result)).not.toContain("PRIVATE_");
		for (const reference of [
			"#missing",
			"#/missing",
			"#/definitions/a~2b",
			"#/%bad",
		]) {
			expect(projectWorkspaceSchema({ $ref: reference }).truncated).toBe(true);
		}
	});

	it("supports alternatives, tuple items and nullable types", () => {
		const schema = {
			type: ["object", "null"],
			properties: {
				value: { anyOf: [{ type: "string" }, { type: "integer" }] },
				choice: { oneOf: [{ type: "number" }, false] },
				pair: {
					type: "array",
					items: [{ type: "string" }, { type: "boolean" }],
				},
			},
			additionalProperties: { type: "string" },
		};
		expect(projectWorkspaceSchema(schema)).toEqual({
			schema,
			truncated: false,
		});
	});

	it("bounds nodes, depth and UTF-8 input size with explicit incomplete results", () => {
		const wide = projectWorkspaceSchema({
			type: "object",
			properties: Object.fromEntries(
				Array.from({ length: 200 }, (_, index) => [
					`field_${index}`,
					{ type: "string" },
				]),
			),
		});
		expect(wide.truncated).toBe(true);
		expect(
			Object.keys((wide.schema as { properties: object }).properties),
		).toHaveLength(127);
		let nested: unknown = { type: "string" };
		for (let index = 0; index < 12; index++)
			nested = { type: "array", items: nested };
		expect(projectWorkspaceSchema(nested).truncated).toBe(true);
		expect(
			projectWorkspaceSchema({ description: "🧩".repeat(17_000) }),
		).toEqual({ schema: null, truncated: true });
	});

	it("rejects malformed or cyclic inputs and never follows inherited pointer properties", () => {
		for (const value of ["{", '"plain string"', "[]", "null", 17]) {
			expect(projectWorkspaceSchema(value).truncated).toBe(true);
		}
		const cyclic: Record<string, unknown> = {};
		cyclic.properties = cyclic;
		expect(projectWorkspaceSchema(cyclic)).toEqual({
			schema: null,
			truncated: true,
		});
		expect(projectWorkspaceSchema({ $ref: "#/constructor" }).truncated).toBe(
			true,
		);
		expect(projectWorkspaceSchema(undefined)).toEqual({
			schema: null,
			truncated: false,
		});
	});

	it("preserves property names safely and marks malformed schema fields incomplete", () => {
		const result = projectWorkspaceSchema(
			'{"type":"object","properties":{"__proto__":{"type":"string"}}}',
		);
		expect(JSON.stringify(result.schema)).toContain(
			'"__proto__":{"type":"string"}',
		);
		expect(result.truncated).toBe(false);
		expect(
			projectWorkspaceSchema({
				type: ["string", {}],
				required: ["id", 99],
				description: { secret: "HIDDEN" },
				properties: [],
			}).truncated,
		).toBe(true);
	});

	it("does not use references to recover excluded instance values", () => {
		for (const [reference, extra] of [
			["#/default", { default: { title: "PRIVATE_VALUE" } }],
			["#/examples/0", { examples: [{ title: "PRIVATE_VALUE" }] }],
			[
				"#/properties/id/default",
				{ properties: { id: { default: { title: "PRIVATE_VALUE" } } } },
			],
		] as const) {
			const result = projectWorkspaceSchema({ $ref: reference, ...extra });
			expect(result.truncated).toBe(true);
			expect(JSON.stringify(result)).not.toContain("PRIVATE_VALUE");
		}
		const allowed = projectWorkspaceSchema({
			$ref: "#/properties/default",
			properties: { default: { type: "string" } },
		});
		expect(allowed.truncated).toBe(false);
	});
});
