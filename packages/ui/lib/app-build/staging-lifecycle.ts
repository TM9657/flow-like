import type { IBackendState } from "../../state/backend-state";
import { type IApp, IAppStatus, IAppVisibility } from "../schema/app/app";
import {
	type AppBuildInventory,
	type AppBuildLifecycleBackend,
	AppBuildStagingError,
	type AppBuildStagingResult,
	type AssertAppBuildActive,
} from "./staging-types";

type InventorySources = [
	Awaited<ReturnType<IBackendState["eventState"]["getEventsAuthoritative"]>>,
	Awaited<ReturnType<IBackendState["pageState"]["getPages"]>>,
	Awaited<ReturnType<IBackendState["widgetState"]["getWidgets"]>>,
	Awaited<ReturnType<IBackendState["dbState"]["listTables"]>>,
	Awaited<
		ReturnType<IBackendState["boardState"]["getBoardSummariesAuthoritative"]>
	>,
];

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

function sortedUnique(values: readonly string[]): string[] {
	return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}

function boardHasContent(
	board: Awaited<ReturnType<IBackendState["boardState"]["getBoard"]>>,
): boolean {
	return (
		Object.keys(board.nodes ?? {}).length > 0 ||
		Object.keys(board.variables ?? {}).length > 0 ||
		Object.keys(board.layers ?? {}).length > 0 ||
		Object.keys(board.comments ?? {}).length > 0 ||
		Object.keys(board.refs ?? {}).length > 0 ||
		(board.page_ids?.length ?? 0) > 0
	);
}

function inventoryError(appId: string, error: unknown): AppBuildStagingError {
	return new AppBuildStagingError(
		"APP_BUILD_INVENTORY_FAILED",
		`Could not verify that app '${appId}' is empty: ${errorMessage(error)}`,
	);
}

async function inspectAppInventory(
	backend: AppBuildLifecycleBackend,
	app: IApp,
	assertActive: AssertAppBuildActive,
): Promise<AppBuildInventory> {
	await assertActive();
	let inventorySources: InventorySources;
	try {
		inventorySources = await Promise.all([
			backend.eventState.getEventsAuthoritative(app.id),
			backend.pageState.getPagesAuthoritative(app.id),
			backend.widgetState.getWidgetsAuthoritative(app.id),
			backend.dbState.listTablesAuthoritative(app.id),
			backend.boardState.getBoardSummariesAuthoritative(app.id),
		]);
	} catch (error) {
		throw inventoryError(app.id, error);
	}
	await assertActive();
	const [events, pages, widgets, tables, boardSummaries] = inventorySources;
	const boardIds = sortedUnique([
		...(app.boards ?? []),
		...boardSummaries.map((board) => board.id),
	]);
	const boards: Array<
		Awaited<ReturnType<IBackendState["boardState"]["getBoardAuthoritative"]>>
	> = [];
	for (const boardId of boardIds) {
		await assertActive();
		try {
			boards.push(
				await backend.boardState.getBoardAuthoritative(app.id, boardId),
			);
		} catch (error) {
			throw inventoryError(app.id, error);
		}
	}
	await assertActive();

	return {
		event_ids: sortedUnique([
			...(app.events ?? []),
			...events.map((event) => event.id),
		]),
		page_ids: sortedUnique([
			...(app.page_ids ?? []),
			...pages.map((page) => page.pageId),
		]),
		widget_ids: sortedUnique([
			...(app.widget_ids ?? []),
			...widgets.map(([, widgetId]) => widgetId),
		]),
		table_names: sortedUnique(tables),
		board_ids: boardIds,
		nonempty_board_ids: boards
			.filter(boardHasContent)
			.map((board) => board.id)
			.sort((left, right) => left.localeCompare(right)),
	};
}

function inventoryIsEmpty(inventory: AppBuildInventory): boolean {
	return (
		inventory.event_ids.length === 0 &&
		inventory.page_ids.length === 0 &&
		inventory.widget_ids.length === 0 &&
		inventory.table_names.length === 0 &&
		inventory.nonempty_board_ids.length === 0
	);
}

export function assertPrivateBuildVisibility(app: IApp): void {
	if (
		app.visibility === IAppVisibility.Public ||
		app.visibility === IAppVisibility.PublicRequestAccess
	) {
		throw new AppBuildStagingError(
			"APP_BUILD_PUBLIC_APP",
			`App '${app.id}' has public visibility and cannot be used as a FlowPilot staging target.`,
		);
	}
}

function assertEmpty(inventory: AppBuildInventory, appId: string): void {
	if (!inventoryIsEmpty(inventory)) {
		throw new AppBuildStagingError(
			"APP_BUILD_NONEMPTY_APP",
			`FlowPilot can only stage a new empty app; '${appId}' already contains resources.`,
			inventory,
		);
	}
}

/** Move a new empty app to Inactive before FlowPilot writes any resources. */
export async function beginAppBuildStaging(
	backend: AppBuildLifecycleBackend,
	appId: string,
	assertActive: AssertAppBuildActive = () => undefined,
): Promise<AppBuildStagingResult> {
	await assertActive();
	const app = await backend.appState.getAppAuthoritative(appId);
	assertPrivateBuildVisibility(app);
	if (app.status === IAppStatus.Archived) {
		throw new AppBuildStagingError(
			"APP_BUILD_ARCHIVED_APP",
			`Archived app '${appId}' cannot be used as a FlowPilot staging target.`,
		);
	}

	const inventory = await inspectAppInventory(backend, app, assertActive);
	assertEmpty(inventory, appId);
	if (app.status === IAppStatus.Inactive) {
		await assertActive();
		const verified = await backend.appState.getAppAuthoritative(appId);
		assertPrivateBuildVisibility(verified);
		if (verified.status !== IAppStatus.Inactive) {
			throw new AppBuildStagingError(
				"APP_BUILD_STAGING_CONFLICT",
				`App '${appId}' changed status while FlowPilot verified its staging state.`,
			);
		}
		const verifiedInventory = await inspectAppInventory(
			backend,
			verified,
			assertActive,
		);
		assertEmpty(verifiedInventory, appId);
		return {
			status: "already_staged",
			app: verified,
			inventory: verifiedInventory,
		};
	}
	if (app.status !== IAppStatus.Active) {
		throw new AppBuildStagingError(
			"APP_BUILD_NONEMPTY_APP",
			`App '${appId}' is not eligible for a new FlowPilot staging transition.`,
			inventory,
		);
	}

	await assertActive();
	const transitionApp = await backend.appState.getAppAuthoritative(appId);
	assertPrivateBuildVisibility(transitionApp);
	if (transitionApp.status !== IAppStatus.Active) {
		throw new AppBuildStagingError(
			"APP_BUILD_STAGING_CONFLICT",
			`App '${appId}' changed status before FlowPilot could stage it.`,
		);
	}
	const transitionInventory = await inspectAppInventory(
		backend,
		transitionApp,
		assertActive,
	);
	if (!inventoryIsEmpty(transitionInventory)) {
		throw new AppBuildStagingError(
			"APP_BUILD_STAGING_CONFLICT",
			`App '${appId}' changed before FlowPilot could stage it.`,
			transitionInventory,
		);
	}
	await assertActive();
	await backend.appState.updateAppAuthoritative({
		...transitionApp,
		status: IAppStatus.Inactive,
	});
	await assertActive();
	const staged = await backend.appState.getAppAuthoritative(appId);
	assertPrivateBuildVisibility(staged);
	if (staged.status !== IAppStatus.Inactive) {
		throw new AppBuildStagingError(
			"APP_BUILD_STAGING_CONFLICT",
			`App '${appId}' did not remain Inactive after the staging transition.`,
		);
	}
	const verifiedInventory = await inspectAppInventory(
		backend,
		staged,
		assertActive,
	);
	if (!inventoryIsEmpty(verifiedInventory)) {
		throw new AppBuildStagingError(
			"APP_BUILD_STAGING_CONFLICT",
			`App '${appId}' changed while FlowPilot was staging it. It remains Inactive for review.`,
			verifiedInventory,
		);
	}
	return { status: "staged", app: staged, inventory: verifiedInventory };
}

export async function requireStagedApp(
	backend: AppBuildLifecycleBackend,
	appId: string,
): Promise<IApp> {
	const app = await backend.appState.getAppAuthoritative(appId);
	assertPrivateBuildVisibility(app);
	if (app.status !== IAppStatus.Inactive) {
		throw new Error(`App '${appId}' must be Inactive throughout promotion.`);
	}
	return app;
}

export async function requireActivePromotionApp(
	backend: AppBuildLifecycleBackend,
	appId: string,
): Promise<IApp> {
	const app = await backend.appState.getAppAuthoritative(appId);
	assertPrivateBuildVisibility(app);
	if (app.status !== IAppStatus.Active) {
		throw new Error(`App '${appId}' is no longer Active during promotion.`);
	}
	return app;
}
