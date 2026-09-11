"use client";

import { useTranslation } from "@flow-like/locales";
import { memo, useMemo } from "react";
import {
	type IQualityMark,
	describeQualityFinding,
} from "../../../lib/board-quality";
import { cn } from "../../../lib/utils";
import { useBoardQualityStore } from "../../../state/board-quality-state";
import { QualitySeverityIcon } from "../flow-quality";

const RING: Record<string, string> = {
	error: "bg-destructive/15 ring-destructive/50",
	warning: "bg-amber-500/15 ring-amber-500/50",
	info: "bg-sky-500/15 ring-sky-500/50",
};

const QualityBadgeContent = memo(function QualityBadgeContent({
	mark,
	className,
}: Readonly<{ mark: IQualityMark; className?: string }>) {
	const { t } = useTranslation("flow");
	const title = useMemo(() => {
		const lines = mark.findings.map((finding) =>
			describeQualityFinding(finding, t),
		);
		if (mark.nested > 0) {
			lines.push(
				t("qualityNestedFindings", {
					defaultValue_one: "{{count}} finding inside this layer",
					defaultValue_other: "{{count}} findings inside this layer",
					count: mark.nested,
				}),
			);
		}
		return lines.join("\n");
	}, [mark, t]);
	const total = mark.counts.error + mark.counts.warning + mark.counts.info;
	return (
		<div
			title={title}
			className={cn(
				"absolute z-10 flex h-3.5 items-center gap-0.5 rounded-full bg-background px-1 ring-1",
				RING[mark.severity],
				className,
			)}
		>
			<QualitySeverityIcon severity={mark.severity} className="size-2.5" />
			{total > 1 && (
				<span className="text-[8px] font-semibold leading-none tabular-nums">
					{total}
				</span>
			)}
		</div>
	);
});

/**
 * The inline lint marker. Every node mounts one, but it subscribes to its own
 * mark only — the store keeps a mark's identity while its findings are
 * unchanged — so a board-wide re-lint re-renders exactly the badges whose
 * findings moved, none of the nodes, and nothing at all while the canvas
 * markers are switched off. The translation hook lives in the child so the
 * common no-finding case costs a single store selector.
 */
export const FlowNodeQualityBadge = memo(function FlowNodeQualityBadge({
	boardId,
	targetId,
	className,
}: Readonly<{
	boardId: string;
	targetId: string;
	className?: string;
}>) {
	const mark = useBoardQualityStore((state) =>
		state.inline ? state.marks[boardId]?.[targetId] : undefined,
	);
	if (!mark) return null;
	return <QualityBadgeContent mark={mark} className={className} />;
});
