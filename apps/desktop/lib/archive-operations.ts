import type { IApp, IAppVisibility } from "@flow-like/flow-like-ui";
import { humanFileSize } from "@flow-like/flow-like-ui/lib/utils";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import type { TFunction } from "i18next";

export const ARCHIVE_PROGRESS_EVENT = "archive:progress";

export type ArchivePhase =
	| "compacting"
	| "listing"
	| "packing"
	| "writing"
	| "reading"
	| "planning"
	| "restoring"
	| "cleaning"
	| "finalizing"
	| "done";

export type ArchiveOperationKind = "export" | "import";

export type ImportMode = "merge" | "replace";

export interface IArchiveProgress {
	phase: ArchivePhase;
	done_bytes: number;
	total_bytes: number;
	done_files: number;
	total_files: number;
}

export interface IArchiveProgressEvent {
	operation_id: string;
	kind: ArchiveOperationKind;
	progress: IArchiveProgress;
}

export interface IArchiveInfo {
	format_version: number;
	encrypted: boolean;
	app_id: string | null;
	created_at: number | null;
	file_count: number | null;
	total_bytes: number | null;
	exists_locally: boolean;
	local_visibility: IAppVisibility | null;
}

export interface IImportReport {
	app: IApp;
	mode: ImportMode;
	restored_files: number;
	skipped_files: number;
	deleted_files: number;
}

type Translate = TFunction<"common">;

const CANCELLED_MARKER = "cancelled";
const WRONG_PASSWORD_MARKER = "binding tag mismatch";

export function newOperationId(): string {
	return crypto.randomUUID();
}

export function listenArchiveProgress(
	operationId: string,
	onProgress: (progress: IArchiveProgress) => void,
): Promise<UnlistenFn> {
	return listen<IArchiveProgressEvent>(ARCHIVE_PROGRESS_EVENT, (event) => {
		if (event.payload.operation_id !== operationId) return;
		onProgress(event.payload.progress);
	});
}

export function cancelArchiveOperation(operationId: string): Promise<boolean> {
	return invoke<boolean>("cancel_archive_operation", { operationId });
}

export function describeArchiveError(error: unknown): string {
	if (typeof error === "string") return error;
	if (error instanceof Error) return error.message;
	if (error && typeof error === "object") {
		const record = error as Record<string, unknown>;
		const text = record.error ?? record.message;
		if (typeof text === "string") return text;
	}
	return String(error);
}

export function isArchiveCancelled(error: unknown): boolean {
	return describeArchiveError(error).toLowerCase().includes(CANCELLED_MARKER);
}

export function isWrongPassword(error: unknown): boolean {
	return describeArchiveError(error).includes(WRONG_PASSWORD_MARKER);
}

export function archivePhaseLabel(t: Translate, phase: ArchivePhase): string {
	switch (phase) {
		case "compacting":
			return t("compactingTables", "Compacting tables…");
		case "listing":
			return t("listingFiles", "Listing files…");
		case "packing":
			return t("packingFiles", "Packing files…");
		case "writing":
			return t("writingArchive", "Writing archive…");
		case "reading":
			return t("readingArchive", "Reading archive…");
		case "planning":
			return t("planningRestore", "Planning restore…");
		case "restoring":
			return t("restoringFiles", "Restoring files…");
		case "cleaning":
			return t("removingStaleFiles", "Removing stale files…");
		case "finalizing":
			return t("finalizing", "Finalizing…");
		case "done":
			return t("done", "Done");
	}
}

export function archiveProgressPercent(progress: IArchiveProgress): number {
	if (progress.phase === "done") return 100;
	if (progress.total_bytes > 0) {
		return Math.min(100, (progress.done_bytes / progress.total_bytes) * 100);
	}
	if (progress.total_files > 0) {
		return Math.min(100, (progress.done_files / progress.total_files) * 100);
	}
	return 0;
}

export function formatProgress(
	t: Translate,
	progress: IArchiveProgress,
): string {
	const parts: string[] = [];
	if (progress.total_bytes > 0) {
		parts.push(
			`${humanFileSize(progress.done_bytes)} / ${humanFileSize(progress.total_bytes)}`,
		);
	}
	if (progress.total_files > 0) {
		parts.push(
			t("doneTotalFiles", "{{done}} / {{total}} files", {
				done: progress.done_files,
				total: progress.total_files,
			}),
		);
	}
	return parts.join(" · ");
}
