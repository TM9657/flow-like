import { describe, expect, test, vi } from "vitest";

import type {
	GraphOverlay,
	GraphSchema,
	IGraphState,
} from "../../state/backend-state/graph-state";
import {
	ONTOLOGY_QUERY_API_PROMPT_LIMIT,
	ONTOLOGY_QUERY_GENERATOR_PROMPT_BUDGET,
	ONTOLOGY_QUERY_MAX_SCHEMA_CONTEXT_LENGTH,
	ONTOLOGY_QUERY_RECEIPT_SCHEMA,
	OntologyQueryController,
	type OntologyQueryProposal,
	type OntologyQueryRuntime,
	type OntologyQuerySchemaContext,
	type OntologyQueryTarget,
	buildOntologyQueryGeneratorPrompt,
	buildOntologyQuerySchemaContext,
	createGraphStateOntologyQueryRuntime,
	normalizeReadOnlyOntologyQuery,
	parseOntologyQueryProposal,
	validateReadOnlyOntologyQuery,
} from "./ontology-query";

const target: OntologyQueryTarget = {
	appId: "app-1",
	overlayId: "overlay-1",
	userScoped: true,
	surfaceInstanceId: "surface-1",
};

const schemaContext: OntologyQuerySchemaContext = {
	ontologyName: "CRM",
	nodes: [
		{
			label: "Person",
			table: "people",
			idColumn: "id",
			displayColumn: "name",
			properties: [
				{ name: "id", dataType: "Utf8", nullable: false },
				{ name: "name", dataType: "Utf8", nullable: false },
			],
		},
	],
	edges: [],
	truncated: false,
};

function proposal(
	overrides: Partial<OntologyQueryProposal> = {},
): OntologyQueryProposal {
	return {
		language: "cypher",
		query: "MATCH (person:Person) RETURN person",
		params: {},
		presentation: "graph",
		...overrides,
	};
}

function runtime(
	execute: OntologyQueryRuntime["execute"] = async () => ({ rows: [] }),
): OntologyQueryRuntime {
	return {
		loadSchema: vi.fn(async () => schemaContext),
		execute: vi.fn(execute),
	};
}

function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (error: unknown) => void;
	const promise = new Promise<T>((resolvePromise, rejectPromise) => {
		resolve = resolvePromise;
		reject = rejectPromise;
	});
	return { promise, resolve, reject };
}

describe("ontology query proposal validation", () => {
	test("normalizes a valid manual statement before backend execution", () => {
		expect(
			normalizeReadOnlyOntologyQuery(
				"cypher",
				"  MATCH (person:Person) RETURN person;  ",
			),
		).toBe("MATCH (person:Person) RETURN person");
	});

	test("accepts one strict read-only proposal and removes its trailing semicolon", () => {
		const parsed = parseOntologyQueryProposal(
			'```json\n{"language":"sql","query":"SELECT name FROM people WHERE id = $person_id;","params":{"person_id":"p-1"},"presentation":"table"}\n```',
			"sql",
		);

		expect(parsed).toEqual({
			language: "sql",
			query: "SELECT name FROM people WHERE id = $person_id",
			params: { person_id: "p-1" },
			presentation: "table",
		});
	});

	test("rejects target overrides and any other model-authored fields", () => {
		expect(() =>
			parseOntologyQueryProposal({
				...proposal(),
				target: { appId: "other-app", overlayId: "other-overlay" },
			}),
		).toThrow("unsupported fields: target");
		expect(() =>
			parseOntologyQueryProposal({
				...proposal(),
				userScoped: false,
			}),
		).toThrow("unsupported fields: userScoped");
	});

	test("rejects mutations, procedures, multiple statements, and unbounded paths", () => {
		for (const query of [
			"SELECT * FROM people; DROP TABLE people",
			"SELECT * FROM people;;",
			"WITH changed AS (DELETE FROM people RETURNING *) SELECT * FROM changed",
			"COPY people TO '/tmp/people.csv'",
		]) {
			expect(() => validateReadOnlyOntologyQuery("sql", query)).toThrow();
		}

		for (const query of [
			"MATCH (person:Person) DELETE person",
			"MATCH (person)--(other) DELETE other",
			"MATCH (person:Person) CALL db.labels() RETURN person",
			"MATCH (a)-[:KNOWS*]->(b) RETURN a, b",
			"MATCH (a)-[:KNOWS*1..5]->(b) RETURN a, b",
		]) {
			expect(() => validateReadOnlyOntologyQuery("cypher", query)).toThrow();
		}
	});

	test("does not treat quoted keywords or comments as executable clauses", () => {
		expect(() =>
			validateReadOnlyOntologyQuery(
				"sql",
				`SELECT "delete", 'DROP TABLE people' AS note FROM people -- UPDATE ignored`,
			),
		).not.toThrow();
		expect(() =>
			validateReadOnlyOntologyQuery(
				"cypher",
				"MATCH (person:`Create`) RETURN person.`delete` /* SET ignored */",
			),
		).not.toThrow();
	});

	test("requires the selected language and JSON-safe bound parameters", () => {
		expect(() => parseOntologyQueryProposal(proposal(), "sql")).toThrow(
			"must produce a sql query",
		);
		expect(() =>
			parseOntologyQueryProposal({
				...proposal(),
				params: { "not-valid!": "value" },
			}),
		).toThrow("parameter name");
		expect(() =>
			parseOntologyQueryProposal({
				...proposal(),
				params: { value: Number.POSITIVE_INFINITY },
			}),
		).toThrow("non-finite");
	});
});

describe("ontology query schema and graph runtime", () => {
	test("builds bounded schema-only model context", () => {
		const overlay = {
			name: "CRM",
			nodes: [
				{
					label: "Person",
					table: "people",
					id_column: "id",
					display_column: "name",
				},
			],
			edges: [
				{
					label: "KNOWS",
					table: "knows",
					src_label: "Person",
					dst_label: "Person",
					src_column: "source",
					dst_column: "target",
				},
			],
		} as unknown as GraphOverlay;
		const schema = {
			node_labels: [
				{
					label: "Person",
					table: "people",
					properties: [{ name: "id", data_type: "Utf8", nullable: false }],
				},
			],
			edge_labels: [
				{
					label: "KNOWS",
					table: "knows",
					properties: [],
				},
			],
		} as GraphSchema;

		expect(buildOntologyQuerySchemaContext(overlay, schema)).toEqual({
			ontologyName: "CRM",
			nodes: [
				{
					label: "Person",
					table: "people",
					idColumn: "id",
					displayColumn: "name",
					properties: [{ name: "id", dataType: "Utf8", nullable: false }],
				},
			],
			edges: [
				{
					label: "KNOWS",
					table: "knows",
					sourceLabel: "Person",
					targetLabel: "Person",
					sourceColumn: "source",
					targetColumn: "target",
					properties: [],
				},
			],
			truncated: false,
		});
		expect(
			JSON.stringify(buildOntologyQuerySchemaContext(overlay, schema)),
		).not.toContain("sample");
	});

	test("caps the serialized schema context", () => {
		const longName = "x".repeat(128);
		const properties = Array.from({ length: 64 }, (_, index) => ({
			name: `${longName}${index}`,
			data_type: `${longName}${index}`,
			nullable: false,
		}));
		const overlay = {
			name: "Large ontology",
			nodes: Array.from({ length: 128 }, (_, index) => ({
				label: `Node${index}${longName}`,
				table: `table${index}${longName}`,
				id_column: `id${index}${longName}`,
			})),
			edges: [],
		} as unknown as GraphOverlay;
		const schema = {
			node_labels: overlay.nodes.map((node) => ({
				label: node.label,
				table: node.table,
				properties,
			})),
			edge_labels: [],
		} as GraphSchema;

		const context = buildOntologyQuerySchemaContext(overlay, schema);

		expect(JSON.stringify(context).length).toBeLessThanOrEqual(
			ONTOLOGY_QUERY_MAX_SCHEMA_CONTEXT_LENGTH,
		);
		expect(context.truncated).toBe(true);
	});

	test("forwards the immutable scope, bound params, and truncation probe", async () => {
		const getOverlay = vi.fn(async () => ({
			name: "CRM",
			nodes: [],
			edges: [],
		}));
		const getSchema = vi.fn(async () => ({
			node_labels: [],
			edge_labels: [],
		}));
		const cypherWithMetadata = vi.fn(async () => ({
			rows: [{ person: "p-1" }],
			property_metadata: { person: { type: "node" } },
		}));
		const graphState = {
			getOverlay,
			getSchema,
			cypherWithMetadata,
		} as unknown as IGraphState;
		const adapter = createGraphStateOntologyQueryRuntime(graphState);

		await adapter.loadSchema(target);
		const result = await adapter.execute(
			target,
			proposal({ params: { name: "Avery" } }),
			50,
		);

		expect(getOverlay).toHaveBeenCalledWith("app-1", "overlay-1", true);
		expect(getSchema).toHaveBeenCalledWith("app-1", "overlay-1", true);
		expect(cypherWithMetadata).toHaveBeenCalledWith(
			"app-1",
			"overlay-1",
			{
				query: "MATCH (person:Person) RETURN person",
				params: { name: "Avery" },
				limit: 51,
			},
			true,
		);
		expect(result.propertyMetadata).toEqual({ person: { type: "node" } });
	});
});

describe("ontology query controller", () => {
	test("returns a structured receipt and never publishes the truncation probe", async () => {
		const statuses: string[] = [];
		let now = 100;
		const controller = new OntologyQueryController({
			target,
			generator: vi.fn(async () => proposal({ presentation: "table" })),
			runtime: runtime(async () => ({
				rows: [{ name: "A" }, { name: "B" }, { name: "probe" }],
				propertyMetadata: { name: { logical_type: "string" } },
			})),
			idFactory: () => "request-1",
			now: () => {
				now += 5;
				return now;
			},
		});

		const result = await controller.run({
			prompt: "Who is in the ontology?",
			limit: 2,
			onStatus: (event) => statuses.push(event.phase),
		});

		expect(result.status).toBe("success");
		if (result.status !== "success") throw new Error("expected success");
		expect(result.receipt).toMatchObject({
			schema: ONTOLOGY_QUERY_RECEIPT_SCHEMA,
			requestId: "request-1",
			target,
			rows: [{ name: "A" }, { name: "B" }],
			rowCount: 2,
			truncated: true,
			effectiveLimit: 2,
			durationMs: 5,
			attempts: 1,
			status: "success",
		});
		expect(result.receipt.columns).toEqual([
			{ name: "name", metadata: { logical_type: "string" } },
		]);
		expect(statuses).toEqual([
			"loading-schema",
			"generating",
			"validating",
			"running",
		]);
	});

	test("uses one repair turn for malformed output", async () => {
		const generator = vi
			.fn()
			.mockResolvedValueOnce("I would use MATCH")
			.mockResolvedValueOnce(proposal());
		const controller = new OntologyQueryController({
			target,
			generator,
			runtime: runtime(),
			idFactory: () => "repair-1",
		});

		const result = await controller.run({ prompt: "Show everyone" });

		expect(result.status).toBe("success");
		if (result.status !== "success") throw new Error("expected success");
		expect(result.receipt.attempts).toBe(2);
		expect(generator.mock.calls[1]?.[0].repair).toMatchObject({
			message: "FlowPilot returned invalid JSON for the query proposal.",
			previousOutput: "I would use MATCH",
		});
	});

	test("repairs a schema execution error without returning rows to the model", async () => {
		const first = proposal({ query: "MATCH (x:Missing) RETURN x" });
		const second = proposal();
		const generator = vi
			.fn()
			.mockResolvedValueOnce(first)
			.mockResolvedValueOnce(second);
		const execute = vi
			.fn()
			.mockRejectedValueOnce(new Error("Unknown label Missing"))
			.mockResolvedValueOnce({ rows: [] });
		const controller = new OntologyQueryController({
			target,
			generator,
			runtime: runtime(execute),
			idFactory: () => "repair-2",
		});

		const result = await controller.run({ prompt: "Show everyone" });

		expect(result.status).toBe("success");
		expect(generator.mock.calls[1]?.[0].repair).toEqual({
			message: "Unknown label Missing",
			previousOutput: first,
			previousProposal: first,
		});
		expect(JSON.stringify(generator.mock.calls[1]?.[0])).not.toContain("rows");
	});

	test("treats zero rows as a successful result", async () => {
		const controller = new OntologyQueryController({
			target,
			generator: async () => proposal(),
			runtime: runtime(),
			idFactory: () => "empty-1",
		});

		const result = await controller.run({ prompt: "Find nobody" });

		expect(result.status).toBe("success");
		if (result.status !== "success") throw new Error("expected success");
		expect(result.receipt.rows).toEqual([]);
		expect(result.receipt.rowCount).toBe(0);
	});

	test("suppresses an older result after a newer request starts", async () => {
		const firstGeneration = deferred<OntologyQueryProposal>();
		let call = 0;
		const controller = new OntologyQueryController({
			target,
			generator: async () => {
				call += 1;
				return call === 1 ? firstGeneration.promise : proposal();
			},
			runtime: runtime(),
			idFactory: (() => {
				let id = 0;
				return () => `stale-${++id}`;
			})(),
		});
		const first = controller.run({ prompt: "First" });
		await vi.waitFor(() => expect(call).toBe(1));
		const second = controller.run({ prompt: "Second" });
		firstGeneration.resolve(proposal());

		expect(await first).toEqual({ status: "stale", requestId: "stale-1" });
		expect((await second).status).toBe("success");
	});

	test("suppresses a cancelled execution even when runtime cannot abort it", async () => {
		const execution = deferred<{ rows: unknown[] }>();
		const execute = vi.fn(async () => execution.promise);
		const controller = new OntologyQueryController({
			target,
			generator: async () => proposal(),
			runtime: runtime(execute),
			idFactory: () => "cancel-1",
		});
		const running = controller.run({ prompt: "Long query" });
		await vi.waitFor(() => expect(execute).toHaveBeenCalledOnce());
		controller.cancel();
		execution.resolve({ rows: [{ shouldNotPublish: true }] });

		expect(await running).toEqual({
			status: "cancelled",
			requestId: "cancel-1",
		});
	});

	test("returns stage-specific errors for non-repairable execution failures", async () => {
		const controller = new OntologyQueryController({
			target,
			generator: async () => proposal(),
			runtime: runtime(async () => {
				throw new Error("Permission denied");
			}),
			idFactory: () => "error-1",
		});

		const result = await controller.run({ prompt: "Show everyone" });

		expect(result.status).toBe("error");
		if (result.status !== "error") throw new Error("expected error");
		expect(result.receipt).toMatchObject({
			language: "cypher",
			query: "MATCH (person:Person) RETURN person",
			status: "error",
			error: { stage: "execution", message: "Permission denied" },
		});
	});

	test("returns transport setup failures as visible generation errors", async () => {
		const controller = new OntologyQueryController({
			target,
			generator: async () => {
				throw new Error(
					"A modelId is required for the selected FlowPilot agent backend.",
				);
			},
			runtime: runtime(),
			idFactory: () => "generation-error-1",
		});

		const result = await controller.run({ prompt: "Show everyone" });

		expect(result.status).toBe("error");
		if (result.status !== "error") throw new Error("expected error");
		expect(result.receipt.error).toEqual({
			stage: "generation",
			message:
				"A modelId is required for the selected FlowPilot agent backend.",
		});
	});
});

describe("ontology query specialist prompt", () => {
	test("contains only the bounded planner inputs and parameter syntax", () => {
		const prompt = buildOntologyQueryGeneratorPrompt({
			requestId: "prompt-1",
			prompt: "Show everyone",
			language: "auto",
			schema: schemaContext,
			attempt: 1,
		});
		const payload = JSON.parse(prompt.userPrompt);

		expect(payload).toEqual({
			question: "Show everyone",
			requestedLanguage: "auto",
			parameterSyntax:
				"Use $name placeholders for bound values in both Cypher and SQL. Parameter keys in params omit the leading $.",
			schema: schemaContext,
		});
		expect(prompt.userPrompt).not.toContain(target.appId);
		expect(prompt.userPrompt).not.toContain(target.overlayId);
		expect(prompt.systemPrompt).toContain("one read-only query");
		expect(prompt.systemPrompt).toContain("do not execute the query");
		expect(prompt.systemPrompt).toContain("$name placeholders");
	});

	test("compacts repair context and a cloned schema below the API boundary", () => {
		const longField = "\\".repeat(128);
		const largeSchema: OntologyQuerySchemaContext = {
			ontologyName: "Large ontology",
			nodes: Array.from({ length: 40 }, (_, index) => ({
				label: `Node${index}${longField}`,
				table: `table${index}${longField}`,
				idColumn: `id${index}${longField}`,
				properties: [
					{
						name: `property${index}${longField}`,
						dataType: `type${index}${longField}`,
						nullable: false,
					},
				],
			})),
			edges: [],
			truncated: false,
		};
		const originalSchema = JSON.stringify(largeSchema);

		const prompt = buildOntologyQueryGeneratorPrompt({
			requestId: "prompt-boundary",
			prompt: "\0".repeat(8_000),
			language: "sql",
			schema: largeSchema,
			attempt: 2,
			repair: {
				message: "\0".repeat(1_000),
				previousOutput: "\\".repeat(20_000),
			},
		});
		const payload = JSON.parse(prompt.userPrompt);

		expect(prompt.userPrompt.length).toBeLessThanOrEqual(
			ONTOLOGY_QUERY_GENERATOR_PROMPT_BUDGET,
		);
		expect(prompt.userPrompt.length).toBeLessThan(
			ONTOLOGY_QUERY_API_PROMPT_LIMIT,
		);
		expect(JSON.stringify(largeSchema)).toBe(originalSchema);
		expect(payload.schema.truncated).toBe(true);
		expect(JSON.stringify(payload.schema).length).toBeLessThan(
			originalSchema.length,
		);
		expect(payload.repair.message).toHaveLength(1_000);
		expect(payload.repair.previousProposal).toContain("[truncated]");
	});
});
