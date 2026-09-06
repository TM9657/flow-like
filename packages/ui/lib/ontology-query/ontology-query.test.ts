import { describe, expect, test, vi } from "vitest";

import type {
	GraphOverlay,
	GraphSchema,
	IGraphState,
} from "../../state/backend-state/graph-state";
import {
	ONTOLOGY_QUERY_RECEIPT_SCHEMA,
	OntologyQueryController,
	type OntologyQueryProposal,
	type OntologyQueryRuntime,
	type OntologyQuerySchemaContext,
	type OntologyQueryTarget,
	buildOntologyQueryGeneratorPrompt,
	buildOntologyQuerySchemaContext,
	createGraphStateOntologyQueryRuntime,
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
			"WITH changed AS (DELETE FROM people RETURNING *) SELECT * FROM changed",
			"COPY people TO '/tmp/people.csv'",
		]) {
			expect(() => validateReadOnlyOntologyQuery("sql", query)).toThrow();
		}

		for (const query of [
			"MATCH (person:Person) DELETE person",
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
		} as GraphOverlay;
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
});

describe("ontology query specialist prompt", () => {
	test("contains only the question, language, bounded schema, and repair state", () => {
		const prompt = buildOntologyQueryGeneratorPrompt({
			prompt: "Show everyone",
			language: "auto",
			schema: schemaContext,
			attempt: 1,
		});
		const payload = JSON.parse(prompt.userPrompt);

		expect(payload).toEqual({
			question: "Show everyone",
			requestedLanguage: "auto",
			schema: schemaContext,
		});
		expect(prompt.userPrompt).not.toContain(target.appId);
		expect(prompt.userPrompt).not.toContain(target.overlayId);
		expect(prompt.systemPrompt).toContain("one read-only query");
		expect(prompt.systemPrompt).toContain("do not execute the query");
	});
});
