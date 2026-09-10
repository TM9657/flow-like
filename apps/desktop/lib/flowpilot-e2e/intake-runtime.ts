import { resolveEventActions } from "@flow-like/flow-like-ui/components/a2ui/event-handlers";
import {
	type LivePageRunRecord,
	findLivePage,
	subscribeLivePageRuns,
	waitForLivePage,
} from "@flow-like/flow-like-ui/components/a2ui/live-page-registry";
import {
	type AppScenarioHostAdapter,
	type AppScenarioRunResult,
	runAppBehaviorScenario,
} from "@flow-like/flow-like-ui/lib/app-build/scenarios";
import {
	interactWithAppPage,
	parseInteractActions,
} from "@flow-like/flow-like-ui/lib/interact-app-page";
import type { IBackendState } from "@flow-like/flow-like-ui/state/backend-state";
import type { IPageBootstrap } from "@flow-like/flow-like-ui/state/backend-state/page-state";
import { invoke } from "@tauri-apps/api/core";
import { readIntakeRenderedQueue } from "./intake-rendered-state";
import {
	INTAKE_RUNTIME_CASES,
	INTAKE_RUNTIME_POLICY,
	type IntakeRuntimeAttestation,
	type NativeIntakeRuntimeOutcome,
	intakeRuntimeScenario,
	requireIntakeRuntimeAttestation,
	validateIntakeNativeOutcomes,
} from "./intake-runtime-contract";

export interface IntakeRuntimeAcceptanceOptions {
	backend: IBackendState;
	appId: string;
	eventId: string;
	pageId: string;
	mountPage: (bootstrap: IPageBootstrap) => Promise<void>;
	describeMount?: () => Record<string, unknown>;
	unmountPage: () => Promise<void>;
	signal?: AbortSignal;
}

export interface IntakeRuntimeAcceptanceResult {
	scenarios: AppScenarioRunResult[];
	attestation?: IntakeRuntimeAttestation;
	error?: string;
	startedAtMs: number;
	completedAtMs: number;
}

export async function assertIntakeRuntimeAvailable(): Promise<void> {
	const status = await invoke<{ available: boolean; policy: string }>(
		"intake_e2e_runtime_status",
	);
	if (!status.available || status.policy !== INTAKE_RUNTIME_POLICY)
		throw new Error("The isolated native intake runtime is unavailable.");
}

function message(error: unknown): string {
	if (error instanceof Error) return error.message;
	if (typeof error === "string") return error;
	return JSON.stringify(error);
}

export async function runIntakeRuntimeAcceptance(
	options: IntakeRuntimeAcceptanceOptions,
): Promise<IntakeRuntimeAcceptanceResult> {
	const { backend, appId, eventId, pageId, signal } = options;
	const result: IntakeRuntimeAcceptanceResult = {
		scenarios: [],
		startedAtMs: Date.now(),
		completedAtMs: 0,
	};
	let restore: (() => Promise<unknown>) | undefined;
	let mounted = false;
	const assertActive = () => signal?.throwIfAborted();
	try {
		assertActive();
		const attest = () =>
			invoke<IntakeRuntimeAttestation>("attest_intake_e2e_runtime", {
				appId,
				eventId,
				pageId,
			});
		result.attestation = await attest();
		requireIntakeRuntimeAttestation(result.attestation, options);
		const original = await backend.eventState.getEvent(appId, eventId);
		if (!original.active) {
			await backend.eventState.upsertEvent(appId, {
				...original,
				active: true,
			});
			restore = () => backend.eventState.upsertEvent(appId, original);
		}
		assertActive();
		mounted = true;
		const bootstrap = await backend.pageState.getPageBootstrap(
			appId,
			undefined,
			eventId,
		);
		if (
			bootstrap.event.id !== eventId ||
			bootstrap.page?.id !== pageId ||
			!bootstrap.executionRevision
		)
			throw new Error(
				"Native Page bootstrap does not match the intake fixture.",
			);
		await options.mountPage(bootstrap);
		const handle = await waitForLivePage(appId, { eventId, pageId }, 20_000);
		if (!handle || !handle.getContainer?.()?.isConnected)
			throw new Error(
				`The intake Page did not mount a connected runtime surface: ${JSON.stringify(
					{
						pageId,
						eventId,
						componentCount: bootstrap.page?.components?.length ?? 0,
						registered: Boolean(handle),
						registeredPageId: findLivePage(appId)?.pageId,
						registeredEventId: findLivePage(appId)?.eventId,
						...options.describeMount?.(),
					},
				)}`,
			);
		const adapter: AppScenarioHostAdapter = {
			async attest(request) {
				assertActive();
				request.signal.throwIfAborted();
				if (
					request.appId !== appId ||
					request.tools.some((tool) => tool !== "interact_app_page")
				)
					throw new Error(
						"Intake runtime accepts only this Page's interaction scenario.",
					);
				const native = await attest();
				requireIntakeRuntimeAttestation(native, options);
				result.attestation = native;
				return {
					source: "host",
					mode: "isolated_runtime",
					strategy: "ephemeral_app",
					isolationId: native.isolationId,
					attestationId: `${native.isolationId}:${native.boardHash}:${native.attestedAtMs}`,
					appId,
					tools: request.tools,
					attestedAtMs: native.attestedAtMs,
					expiresAtMs: Math.min(request.deadlineAtMs, native.expiresAtMs),
				};
			},
			async invoke(request) {
				assertActive();
				request.signal.throwIfAborted();
				const native = result.attestation;
				if (!native) throw new Error("Native attestation is absent.");
				requireIntakeRuntimeAttestation(native, options);
				if (
					request.tool !== "interact_app_page" ||
					request.appId !== appId ||
					request.arguments.event_id !== eventId ||
					request.isolationId !== native.isolationId
				)
					throw new Error("Runtime invocation escaped its attested Page.");
				const live = findLivePage(appId, { eventId, pageId });
				if (live !== handle || !live.getContainer?.()?.isConnected)
					throw new Error("The attested Page surface changed.");
				const button = live.getSurface()?.components.submit_ticket?.component;
				if (!button)
					throw new Error("Required submit_ticket component is absent.");
				const bound = resolveEventActions(
					button.eventHandlers,
					"click",
					button.actions,
				);
				if (
					bound.actions.length !== 1 ||
					bound.actions.some(
						(action) =>
							!action.pageAction ||
							action.pageAction.capabilityJwt ||
							!native.actionIds.includes(action.pageAction.actionId),
					)
				)
					throw new Error(
						"Submit must resolve to exactly one native compiled Page action.",
					);
				const actions = parseInteractActions(request.arguments.actions);
				if (
					actions.length !== 2 ||
					actions[0]?.action !== "set_value" ||
					actions[0].component_id !== "summary_input" ||
					typeof actions[0].value !== "string" ||
					actions[1]?.action !== "trigger" ||
					actions[1].component_id !== "submit_ticket" ||
					actions[1].event !== "click"
				)
					throw new Error("Unexpected intake scenario actions.");
				const summary = actions[0].value;
				const before = await backend.dbState.countItems(
					appId,
					"intake_tickets",
					false,
				);
				const startedAtMs = Date.now();
				const records: LivePageRunRecord[] = [];
				const unsubscribe = subscribeLivePageRuns(pageId, (record) =>
					records.push(record),
				);
				let value: Record<string, unknown>;
				try {
					value = await interactWithAppPage(backend, {
						appId,
						eventId,
						pageId,
						actions,
						captureScreenshots: false,
						deadlineAtMs: request.deadlineAtMs,
					});
				} finally {
					unsubscribe();
				}
				const runs = (Array.isArray(value.runs) ? value.runs : []) as {
					run_id?: string;
				}[];
				const runIds = runs
					.map((run) => run.run_id)
					.filter(
						(id): id is string => typeof id === "string" && id.length > 0,
					);
				if (
					runIds.length !== runs.length ||
					records.some(
						(record) => record.runId && !runIds.includes(record.runId),
					)
				)
					throw new Error("Page run observation is incomplete.");
				const outcomes = await invoke<NativeIntakeRuntimeOutcome[]>(
					"read_intake_e2e_outcomes",
					{ appId, runIds },
				);
				const runtimeOutcomes = validateIntakeNativeOutcomes(
					outcomes,
					runIds,
					options,
					startedAtMs,
				);
				const after = await backend.dbState.countItems(
					appId,
					"intake_tickets",
					false,
				);
				if (after > 250)
					throw new Error(
						"Isolated intake row count exceeded the bounded inspection limit.",
					);
				const rows = await backend.dbState.listItems(
					appId,
					"intake_tickets",
					0,
					250,
					false,
				);
				const matches = rows.filter((row) => row.summary === summary);
				const applied = (
					Array.isArray(value.applied_actions) ? value.applied_actions : []
				) as {
					ok?: boolean;
				}[];
				const state = {
					rowCountDelta: after - before,
					matchingRows: matches.length,
					row: matches[0] ?? null,
					renderedQueue: readIntakeRenderedQueue(live),
					actionsCompleted:
						applied.length === 2 &&
						applied.every((action) => action.ok === true),
					nativeOutcomes: outcomes,
					pageRunRecords: records,
				};
				return {
					invocationId: request.invocationId,
					observedAtMs: Date.now(),
					value,
					state,
					runtimeOutcomes,
				};
			},
		};
		for (const fixture of INTAKE_RUNTIME_CASES) {
			assertActive();
			result.scenarios.push(
				await runAppBehaviorScenario(
					intakeRuntimeScenario(fixture, crypto.randomUUID()),
					{
						appId,
						resources: {
							intake_page: { kind: "page", id: pageId, appId, eventId, pageId },
						},
						adapter,
						signal,
						maxDurationMs: 90_000,
					},
				),
			);
		}
	} catch (error) {
		result.error = message(error);
	} finally {
		try {
			if (mounted) await options.unmountPage();
		} catch (error) {
			result.error = [result.error, `Page cleanup failed: ${message(error)}`]
				.filter(Boolean)
				.join("; ");
		}
		try {
			await restore?.();
		} catch (error) {
			result.error = [
				result.error,
				`Event restoration failed: ${message(error)}`,
			]
				.filter(Boolean)
				.join("; ");
		}
		result.completedAtMs = Date.now();
	}
	return result;
}
