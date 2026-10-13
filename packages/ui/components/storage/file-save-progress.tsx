"use client";

import { Loader2Icon } from "lucide-react";
import {
	BulkUploadPartialFailureError,
	type IBulkUploadProgress,
} from "../../lib/bulk-upload";
import { Progress } from "../ui/progress";

export interface FileSaveStatus {
	stage: string;
	progress: number | null;
}

export function storageSaveProgress(
	percent: number,
	detail?: IBulkUploadProgress,
): FileSaveStatus {
	if (detail?.phase === "preparing") {
		return { stage: "Preparing storage", progress: null };
	}
	if (detail?.phase === "done" || percent >= 100) {
		return { stage: "Finishing save", progress: null };
	}
	return {
		stage: "Saving file",
		progress: Number.isFinite(percent) ? Math.max(0, percent) : null,
	};
}

export function fileSaveError(cause: unknown): string {
	if (cause instanceof BulkUploadPartialFailureError) {
		const detail = cause.result.failed[0]?.error;
		if (detail?.trim()) return detail;
	}
	if (cause instanceof Error && cause.message.trim()) return cause.message;
	if (typeof cause === "string" && cause.trim()) return cause;
	return "The file could not be saved.";
}

export function FileSaveProgress({
	stage,
	progress = null,
}: {
	stage: string;
	progress?: number | null;
}) {
	const percent =
		progress !== null && Number.isFinite(progress)
			? Math.min(100, Math.max(0, Math.floor(progress)))
			: null;
	return (
		<div className="shrink-0 space-y-2 border-b px-3 py-2">
			<div className="flex items-center gap-2 text-sm">
				<Loader2Icon
					className="size-4 shrink-0 animate-spin motion-reduce:animate-none"
					aria-hidden="true"
				/>
				<output className="flex-1">{stage}...</output>
				{percent !== null && (
					<span className="text-xs tabular-nums text-muted-foreground">
						{percent}%
					</span>
				)}
			</div>
			{percent === null ? (
				<div className="h-1.5 animate-pulse rounded-full bg-primary/20 motion-reduce:animate-none" />
			) : (
				<Progress value={percent} aria-label={stage} className="h-1.5" />
			)}
		</div>
	);
}
