import type { IBackendState } from "../../state/backend-state";
import type { IApp } from "../schema/app/app";

export type AppBuildLifecycleBackend = Pick<
	IBackendState,
	| "appState"
	| "boardState"
	| "dbState"
	| "eventState"
	| "pageState"
	| "widgetState"
>;

export type ExactVersion = [number, number, number];

export interface AppBuildInventory {
	readonly event_ids: readonly string[];
	readonly page_ids: readonly string[];
	readonly widget_ids: readonly string[];
	readonly table_names: readonly string[];
	readonly board_ids: readonly string[];
	readonly nonempty_board_ids: readonly string[];
}

export interface AppBuildStagingResult {
	readonly status: "staged" | "already_staged";
	readonly app: IApp;
	readonly inventory: AppBuildInventory;
}

export type AppBuildStagingErrorCode =
	| "APP_BUILD_PUBLIC_APP"
	| "APP_BUILD_ARCHIVED_APP"
	| "APP_BUILD_NONEMPTY_APP"
	| "APP_BUILD_STAGING_CONFLICT"
	| "APP_BUILD_INVENTORY_FAILED";

export class AppBuildStagingError extends Error {
	readonly code: AppBuildStagingErrorCode;
	readonly inventory?: AppBuildInventory;

	constructor(
		code: AppBuildStagingErrorCode,
		message: string,
		inventory?: AppBuildInventory,
	) {
		super(message);
		this.name = "AppBuildStagingError";
		this.code = code;
		this.inventory = inventory;
	}
}

export type PromotionJournalStatus =
	| "verified"
	| "applied"
	| "failed"
	| "rolled_back"
	| "rollback_failed";

export interface PromotionJournalEntry {
	readonly operation: string;
	readonly status: PromotionJournalStatus;
	readonly recorded_at_ms: number;
	readonly resource_key?: string;
	readonly physical_id?: string;
	readonly version?: ExactVersion;
	readonly message?: string;
}

export interface AppBuildVersionReceipt {
	readonly boards: Readonly<
		Record<
			string,
			{ readonly physical_id: string; readonly version: ExactVersion }
		>
	>;
	readonly pages: Readonly<
		Record<
			string,
			{
				readonly physical_id: string;
				readonly board_id: string;
				readonly board_version: ExactVersion;
			}
		>
	>;
	readonly widgets: Readonly<
		Record<
			string,
			{ readonly physical_id: string; readonly version: ExactVersion }
		>
	>;
	readonly events: Readonly<
		Record<
			string,
			{
				readonly physical_id: string;
				readonly event_version: ExactVersion;
				readonly board_id: string;
				readonly board_version: ExactVersion;
			}
		>
	>;
}

export interface AppBuildPromotionResult {
	readonly status: "complete" | "partial";
	readonly app_id: string;
	readonly version_receipt: AppBuildVersionReceipt;
	readonly journal: readonly PromotionJournalEntry[];
	readonly attempted_event_ids: readonly string[];
	readonly activated_event_ids: readonly string[];
	readonly external_effects_may_have_occurred: boolean;
	readonly rollback: {
		readonly attempted: boolean;
		readonly app_deactivated: boolean;
		readonly disabled_event_ids: readonly string[];
		readonly failed_event_ids: readonly string[];
	};
	readonly error?: string;
}

/** Called between effect phases so the engine can reject a stale build or abort signal. */
export type AssertAppBuildActive = () => void | Promise<void>;
