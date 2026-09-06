import { IAppStatus } from "../schema/app/app";
import { IVersionType } from "../schema/flow/version-type";
import type { CompiledAppResource, CompiledAppSpec } from "./compiler";
import {
	boardContentFingerprint,
	eventBoardResource,
	eventContentFingerprint,
	exactVersion,
	pageContentFingerprint,
	resourceByKey,
	widgetContentFingerprint,
} from "./promotion-readback";
import {
	requireActivePromotionApp,
	requireStagedApp,
} from "./staging-lifecycle";
import type {
	AppBuildLifecycleBackend,
	AppBuildPromotionResult,
	AssertAppBuildActive,
	ExactVersion,
	PromotionJournalEntry,
} from "./staging-types";

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

function pushJournal(
	journal: PromotionJournalEntry[],
	entry: Omit<PromotionJournalEntry, "recorded_at_ms">,
): void {
	journal.push({ ...entry, recorded_at_ms: Date.now() });
}

/**
 * Publish immutable snapshots and pin inactive Events before exposing the app. The app becomes
 * Active before Events are activated one at a time.
 *
 * The backend has no transaction spanning these resource families. Once app activation starts,
 * users may observe the app, and an Event may execute before a later step fails or compensation
 * finishes. The result records that possibility explicitly. A failure triggers best-effort app
 * and Event deactivation, while published immutable versions remain available for inspection.
 */
export async function promoteAppBuildResources(
	backend: AppBuildLifecycleBackend,
	plan: CompiledAppSpec,
	assertActive: AssertAppBuildActive,
): Promise<AppBuildPromotionResult> {
	const journal: PromotionJournalEntry[] = [];
	const boards: Record<string, { physical_id: string; version: ExactVersion }> =
		{};
	const pages: Record<
		string,
		{ physical_id: string; board_id: string; board_version: ExactVersion }
	> = {};
	const widgets: Record<
		string,
		{ physical_id: string; version: ExactVersion }
	> = {};
	const events: Record<
		string,
		{
			physical_id: string;
			event_version: ExactVersion;
			board_id: string;
			board_version: ExactVersion;
		}
	> = {};
	const attemptedEventIds = new Set<string>();
	const activatedEventIds = new Set<string>();
	let appActivationAttempted = false;

	try {
		await assertActive();
		await requireStagedApp(backend, plan.app_id);

		const boardBaselines = new Map<string, string>();
		const pageBaselines = new Map<string, string>();
		const widgetBaselines = new Map<string, string>();
		const eventBaselines = new Map<string, string>();
		const tableNames = new Set(
			await backend.dbState.listTablesAuthoritative(plan.app_id),
		);

		for (const resource of plan.resources) {
			switch (resource.kind) {
				case "board":
					boardBaselines.set(
						resource.key,
						await boardContentFingerprint(
							backend,
							plan.app_id,
							resource.physical_id,
						),
					);
					break;
				case "page": {
					const board = resourceByKey(plan, resource.config.board);
					if (board.kind !== "board") {
						throw new Error(
							`Page '${resource.key}' references a non-board resource.`,
						);
					}
					const page = await backend.pageState.getPageAuthoritative(
						plan.app_id,
						resource.physical_id,
						board.physical_id,
						undefined,
					);
					pageBaselines.set(resource.key, pageContentFingerprint(page));
					break;
				}
				case "widget": {
					const widget = await backend.widgetState.getWidgetAuthoritative(
						plan.app_id,
						resource.physical_id,
					);
					widgetBaselines.set(resource.key, widgetContentFingerprint(widget));
					break;
				}
				case "table":
					if (!tableNames.has(resource.physical_id)) {
						throw new Error(`Table '${resource.physical_id}' is missing.`);
					}
					break;
				case "event": {
					const event = await backend.eventState.getEventAuthoritative(
						plan.app_id,
						resource.physical_id,
					);
					if (event.active) {
						throw new Error(
							`Event '${resource.physical_id}' is active before promotion.`,
						);
					}
					const board = eventBoardResource(plan, resource);
					if (event.board_id !== board.physical_id) {
						throw new Error(
							`Event '${resource.physical_id}' targets the wrong board.`,
						);
					}
					eventBaselines.set(resource.key, eventContentFingerprint(event));
					break;
				}
			}
		}
		pushJournal(journal, {
			operation: "verify_promotion_inputs",
			status: "verified",
		});
		await assertActive();

		for (const resource of plan.resources.filter(
			(resource): resource is Extract<CompiledAppResource, { kind: "board" }> =>
				resource.kind === "board",
		)) {
			await assertActive();
			const version = exactVersion(
				await backend.boardState.createBoardVersion(
					plan.app_id,
					resource.physical_id,
					IVersionType.Patch,
				),
				`Board '${resource.physical_id}'`,
			);
			boards[resource.key] = { physical_id: resource.physical_id, version };
			const observed = await boardContentFingerprint(
				backend,
				plan.app_id,
				resource.physical_id,
				version,
			);
			if (observed !== boardBaselines.get(resource.key)) {
				throw new Error(
					`Board '${resource.physical_id}' changed while its immutable version was created.`,
				);
			}
			pushJournal(journal, {
				operation: "version_board",
				status: "applied",
				resource_key: resource.key,
				physical_id: resource.physical_id,
				version,
			});

			for (const pageResource of plan.resources.filter(
				(
					candidate,
				): candidate is Extract<CompiledAppResource, { kind: "page" }> =>
					candidate.kind === "page" && candidate.config.board === resource.key,
			)) {
				const page = await backend.pageState.getPageAuthoritative(
					plan.app_id,
					pageResource.physical_id,
					resource.physical_id,
					version,
				);
				if (
					pageContentFingerprint(page) !== pageBaselines.get(pageResource.key)
				) {
					throw new Error(
						`Page '${pageResource.physical_id}' changed while board version ${version.join(".")} was created.`,
					);
				}
				pages[pageResource.key] = {
					physical_id: pageResource.physical_id,
					board_id: resource.physical_id,
					board_version: version,
				};
				pushJournal(journal, {
					operation: "verify_versioned_page",
					status: "verified",
					resource_key: pageResource.key,
					physical_id: pageResource.physical_id,
					version,
				});
			}
		}

		for (const resource of plan.resources.filter(
			(
				resource,
			): resource is Extract<CompiledAppResource, { kind: "widget" }> =>
				resource.kind === "widget",
		)) {
			await assertActive();
			const version = exactVersion(
				await backend.widgetState.createWidgetVersion(
					plan.app_id,
					resource.physical_id,
					"Patch",
				),
				`Widget '${resource.physical_id}'`,
			);
			widgets[resource.key] = { physical_id: resource.physical_id, version };
			const versioned = await backend.widgetState.getWidgetAuthoritative(
				plan.app_id,
				resource.physical_id,
				version,
			);
			if (
				widgetContentFingerprint(versioned) !==
				widgetBaselines.get(resource.key)
			) {
				throw new Error(
					`Widget '${resource.physical_id}' changed while its immutable version was created.`,
				);
			}
			pushJournal(journal, {
				operation: "version_widget",
				status: "applied",
				resource_key: resource.key,
				physical_id: resource.physical_id,
				version,
			});
		}

		await assertActive();
		await requireStagedApp(backend, plan.app_id);
		for (const resource of plan.resources.filter(
			(resource): resource is Extract<CompiledAppResource, { kind: "event" }> =>
				resource.kind === "event",
		)) {
			await assertActive();
			await requireStagedApp(backend, plan.app_id);
			const current = await backend.eventState.getEventAuthoritative(
				plan.app_id,
				resource.physical_id,
			);
			if (
				current.active ||
				eventContentFingerprint(current) !== eventBaselines.get(resource.key)
			) {
				throw new Error(
					`Event '${resource.physical_id}' changed before it could be pinned.`,
				);
			}
			const board = eventBoardResource(plan, resource);
			const boardVersion = boards[board.key]?.version;
			if (!boardVersion) {
				throw new Error(
					`Event '${resource.physical_id}' has no published board version.`,
				);
			}
			const saved = await backend.eventState.upsertEvent(
				plan.app_id,
				{ ...current, active: false, board_version: boardVersion },
				IVersionType.Patch,
			);
			const pinnedVersion = exactVersion(
				saved.event_version,
				`Event '${resource.physical_id}'`,
			);
			const observed = await backend.eventState.getEventAuthoritative(
				plan.app_id,
				resource.physical_id,
			);
			await requireStagedApp(backend, plan.app_id);
			if (
				saved.active ||
				observed.active ||
				saved.board_id !== board.physical_id ||
				observed.board_id !== board.physical_id ||
				JSON.stringify(saved.board_version) !== JSON.stringify(boardVersion) ||
				JSON.stringify(observed.board_version) !==
					JSON.stringify(boardVersion) ||
				eventContentFingerprint(saved) !== eventBaselines.get(resource.key) ||
				eventContentFingerprint(observed) !== eventBaselines.get(resource.key)
			) {
				throw new Error(
					`Event '${resource.physical_id}' pin readback differed.`,
				);
			}
			pushJournal(journal, {
				operation: "pin_event",
				status: "applied",
				resource_key: resource.key,
				physical_id: resource.physical_id,
				version: pinnedVersion,
			});
		}

		await assertActive();
		const staged = await requireStagedApp(backend, plan.app_id);
		appActivationAttempted = true;
		await backend.appState.updateAppAuthoritative({
			...staged,
			status: IAppStatus.Active,
		});
		const active = await backend.appState.getAppAuthoritative(plan.app_id);
		if (active.status !== IAppStatus.Active) {
			throw new Error(
				`App '${plan.app_id}' did not remain Active after promotion.`,
			);
		}
		pushJournal(journal, {
			operation: "activate_app",
			status: "applied",
			physical_id: plan.app_id,
		});

		for (const resource of plan.resources.filter(
			(resource): resource is Extract<CompiledAppResource, { kind: "event" }> =>
				resource.kind === "event",
		)) {
			await assertActive();
			await requireActivePromotionApp(backend, plan.app_id);
			const current = await backend.eventState.getEventAuthoritative(
				plan.app_id,
				resource.physical_id,
			);
			if (
				current.active ||
				eventContentFingerprint(current) !== eventBaselines.get(resource.key)
			) {
				throw new Error(
					`Event '${resource.physical_id}' changed before it could be activated.`,
				);
			}
			const board = eventBoardResource(plan, resource);
			const boardVersion = boards[board.key]?.version;
			if (!boardVersion) throw new Error("Pinned board version disappeared.");
			if (
				JSON.stringify(current.board_version) !== JSON.stringify(boardVersion)
			) {
				throw new Error(
					`Event '${resource.physical_id}' lost its pinned board version.`,
				);
			}
			attemptedEventIds.add(resource.physical_id);
			const saved = await backend.eventState.upsertEvent(
				plan.app_id,
				{ ...current, active: true, board_version: boardVersion },
				IVersionType.Patch,
			);
			activatedEventIds.add(resource.physical_id);
			const eventVersion = exactVersion(
				saved.event_version,
				`Event '${resource.physical_id}'`,
			);
			const observed = await backend.eventState.getEventAuthoritative(
				plan.app_id,
				resource.physical_id,
			);
			if (
				!saved.active ||
				!observed.active ||
				saved.board_id !== board.physical_id ||
				observed.board_id !== board.physical_id ||
				JSON.stringify(saved.board_version) !== JSON.stringify(boardVersion) ||
				JSON.stringify(observed.board_version) !==
					JSON.stringify(boardVersion) ||
				eventContentFingerprint(saved) !== eventBaselines.get(resource.key) ||
				eventContentFingerprint(observed) !== eventBaselines.get(resource.key)
			) {
				throw new Error(
					`Event '${resource.physical_id}' activation readback differed.`,
				);
			}
			events[resource.key] = {
				physical_id: resource.physical_id,
				event_version: eventVersion,
				board_id: board.physical_id,
				board_version: boardVersion,
			};
			pushJournal(journal, {
				operation: "activate_event",
				status: "applied",
				resource_key: resource.key,
				physical_id: resource.physical_id,
				version: eventVersion,
			});
		}
		await requireActivePromotionApp(backend, plan.app_id);

		return {
			status: "complete",
			app_id: plan.app_id,
			version_receipt: { boards, pages, widgets, events },
			journal,
			attempted_event_ids: [...attemptedEventIds],
			activated_event_ids: [...activatedEventIds],
			external_effects_may_have_occurred:
				appActivationAttempted || attemptedEventIds.size > 0,
			rollback: {
				attempted: false,
				app_deactivated: false,
				disabled_event_ids: [],
				failed_event_ids: [],
			},
		};
	} catch (error) {
		pushJournal(journal, {
			operation: "promotion",
			status: "failed",
			message: errorMessage(error),
		});

		let appDeactivated = false;
		if (appActivationAttempted) {
			try {
				const app = await backend.appState.getAppAuthoritative(plan.app_id);
				if (app.status === IAppStatus.Active) {
					await backend.appState.updateAppAuthoritative({
						...app,
						status: IAppStatus.Inactive,
					});
				}
				const verified = await backend.appState.getAppAuthoritative(
					plan.app_id,
				);
				appDeactivated = verified.status === IAppStatus.Inactive;
				pushJournal(journal, {
					operation: "deactivate_app",
					status: appDeactivated ? "rolled_back" : "rollback_failed",
					physical_id: plan.app_id,
				});
			} catch (rollbackError) {
				pushJournal(journal, {
					operation: "deactivate_app",
					status: "rollback_failed",
					physical_id: plan.app_id,
					message: errorMessage(rollbackError),
				});
			}
		}

		const disabledEventIds: string[] = [];
		const failedEventIds: string[] = [];
		const plannedEvents = plan.resources.filter(
			(resource): resource is Extract<CompiledAppResource, { kind: "event" }> =>
				resource.kind === "event",
		);
		for (const resource of [...plannedEvents].reverse()) {
			try {
				const current = await backend.eventState.getEventAuthoritative(
					plan.app_id,
					resource.physical_id,
				);
				const wasActive = current.active;
				if (current.active) {
					activatedEventIds.add(resource.physical_id);
					await backend.eventState.upsertEvent(plan.app_id, {
						...current,
						active: false,
					});
				}
				const verified = await backend.eventState.getEventAuthoritative(
					plan.app_id,
					resource.physical_id,
				);
				if (verified.active) {
					throw new Error("Event remained active after rollback.");
				}
				if (wasActive) {
					disabledEventIds.push(resource.physical_id);
					pushJournal(journal, {
						operation: "disable_event",
						status: "rolled_back",
						resource_key: resource.key,
						physical_id: resource.physical_id,
					});
				}
			} catch (rollbackError) {
				failedEventIds.push(resource.physical_id);
				pushJournal(journal, {
					operation: "disable_event",
					status: "rollback_failed",
					resource_key: resource.key,
					physical_id: resource.physical_id,
					message: errorMessage(rollbackError),
				});
			}
		}

		return {
			status: "partial",
			app_id: plan.app_id,
			version_receipt: { boards, pages, widgets, events },
			journal,
			attempted_event_ids: [...attemptedEventIds],
			activated_event_ids: [...activatedEventIds],
			external_effects_may_have_occurred:
				appActivationAttempted || attemptedEventIds.size > 0,
			rollback: {
				attempted: appActivationAttempted || attemptedEventIds.size > 0,
				app_deactivated: appDeactivated,
				disabled_event_ids: disabledEventIds,
				failed_event_ids: failedEventIds,
			},
			error: errorMessage(error),
		};
	}
}
