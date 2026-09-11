"use client";

import { Progress } from "@flow-like/flow-like-ui";
import { useTranslation } from "@flow-like/locales";
import {
	type IArchiveProgress,
	archivePhaseLabel,
	archiveProgressPercent,
	formatProgress,
} from "../../../lib/archive-operations";

export function ArchiveProgressView({
	progress,
}: {
	progress: IArchiveProgress | null;
}) {
	const { t } = useTranslation("common");
	const label = progress
		? archivePhaseLabel(t, progress.phase)
		: t("preparing", "Preparing…");
	const detail = progress ? formatProgress(t, progress) : "";

	return (
		<div className="space-y-2" aria-live="polite">
			<div className="flex items-center justify-between gap-3 text-xs">
				<span className="font-medium">{label}</span>
				{detail && (
					<span className="text-muted-foreground tabular-nums">{detail}</span>
				)}
			</div>
			<Progress value={progress ? archiveProgressPercent(progress) : 0} />
		</div>
	);
}
