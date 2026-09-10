import { z } from "zod";
import type { IBackendState } from "../../state/backend-state";
import { IExecutionStage, ILogLevel } from "../schema/flow/board";
import type { CompiledAppSpec } from "./compiler";
import { tableColumnSchema } from "./contract";
import { appBuildFingerprint } from "./fingerprint";
import { schemaFields, tableSchemaIssues } from "./table-schema-readback";

const foundationSchema = z
	.object({
		app_id: z.string().min(1),
		boards: z
			.array(
				z
					.object({
						id: z.string().min(1),
						name: z.string().min(1),
						description: z.string().optional(),
					})
					.strict(),
			)
			.max(128),
		tables: z
			.array(
				z
					.object({
						name: z.string().min(1),
						columns: z.array(tableColumnSchema).min(1).max(256),
					})
					.strict(),
			)
			.max(128),
	})
	.strict();

export type AppBuildFoundation = z.infer<typeof foundationSchema>;

export interface AppBuildFoundationEvidence {
	readonly schema: "flowpilot.app-build-foundation/v1";
	readonly app_id: string;
	readonly contract_fingerprint: string;
	readonly boards: readonly { id: string; name: string; reused: boolean }[];
	readonly tables: readonly {
		name: string;
		created: boolean;
		fields: readonly Record<string, unknown>[];
	}[];
	readonly verified_at_ms: number;
}

export function appBuildFoundationForPlan(
	plan: CompiledAppSpec,
): AppBuildFoundation {
	return {
		app_id: plan.app_id,
		boards: plan.resources.flatMap((resource) =>
			resource.kind === "board"
				? [{ id: resource.physical_id, name: resource.config.name }]
				: [],
		),
		tables: plan.resources.flatMap((resource) =>
			resource.kind === "table"
				? [{ name: resource.config.name, columns: resource.config.columns }]
				: [],
		),
	};
}

/** Host-owned IDs identify existing board shells; setup never replaces their workflow content. */
export async function provisionAppBuildFoundation(
	backend: IBackendState,
	input: AppBuildFoundation,
	options: { assertActive(): void },
): Promise<AppBuildFoundationEvidence> {
	const contract = foundationSchema.parse(input);
	const unique = (values: readonly string[], label: string) => {
		if (new Set(values).size !== values.length)
			throw new Error(`Duplicate ${label} in app foundation.`);
	};
	unique(
		contract.boards.map((board) => board.id),
		"board IDs",
	);
	unique(
		contract.tables.map((table) => table.name),
		"table names",
	);
	for (const table of contract.tables)
		unique(
			table.columns.map((column) => column.name),
			"table columns",
		);
	const assertStaged = async () => {
		options.assertActive();
		const app = await backend.appState.getAppAuthoritative(contract.app_id);
		if (app.status !== "Inactive")
			throw new Error("App foundation requires an inactive staging app.");
		options.assertActive();
	};
	await assertStaged();
	const tables: AppBuildFoundationEvidence["tables"][number][] = [];
	const existingTables = contract.tables.length
		? await backend.dbState.listTablesAuthoritative(contract.app_id)
		: [];
	// Read every existing schema before creating anything. Drift is a repair decision.
	for (const table of contract.tables) {
		if (!existingTables.includes(table.name)) continue;
		const fields = schemaFields(
			await backend.dbState.getSchemaAuthoritative(contract.app_id, table.name),
		);
		const issues = tableSchemaIssues(fields, table.columns);
		if (issues.length) throw new Error(`${table.name}: ${issues.join(" ")}`);
	}
	for (const table of contract.tables) {
		await assertStaged();
		let created = false;
		if (!existingTables.includes(table.name)) {
			const result = await backend.dbState.createTable(
				contract.app_id,
				table.name,
				table.columns,
				true,
			);
			created = result.created;
		}
		const names = await backend.dbState.listTablesAuthoritative(
			contract.app_id,
		);
		if (!names.includes(table.name))
			throw new Error(`Table '${table.name}' was not persisted.`);
		const fields = schemaFields(
			await backend.dbState.getSchemaAuthoritative(contract.app_id, table.name),
		);
		const issues = tableSchemaIssues(fields, table.columns);
		if (issues.length) throw new Error(`${table.name}: ${issues.join(" ")}`);
		tables.push({ name: table.name, created, fields });
	}
	const boards: AppBuildFoundationEvidence["boards"][number][] = [];
	const existingBoards = contract.boards.length
		? await backend.boardState.getBoardSummariesAuthoritative(contract.app_id)
		: [];
	for (const board of contract.boards) {
		await assertStaged();
		const reused = existingBoards.some((item) => item.id === board.id);
		if (!reused) {
			await backend.boardState.upsertBoard(
				contract.app_id,
				board.id,
				board.name,
				board.description ?? "",
				ILogLevel.Info,
				IExecutionStage.Dev,
			);
		}
		const actual = await backend.boardState.getBoardAuthoritative(
			contract.app_id,
			board.id,
		);
		if (actual.id !== board.id)
			throw new Error(
				`Board '${board.id}' did not read back at its reserved ID.`,
			);
		// A canonical read also opens the native handle required by the compiler's apply path.
		await backend.boardState.getFlowScriptAuthoritative(
			contract.app_id,
			board.id,
			undefined,
			true,
		);
		boards.push({ id: actual.id, name: actual.name, reused });
	}
	await assertStaged();
	return {
		schema: "flowpilot.app-build-foundation/v1",
		app_id: contract.app_id,
		contract_fingerprint: appBuildFingerprint("app-foundation", contract),
		boards,
		tables,
		verified_at_ms: Date.now(),
	};
}
