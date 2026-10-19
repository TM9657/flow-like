"use client";

import AppWindow from "lucide-react/dist/esm/icons/app-window.js";
import ChevronDown from "lucide-react/dist/esm/icons/chevron-down.js";
import Database from "lucide-react/dist/esm/icons/database.js";
import ExternalLink from "lucide-react/dist/esm/icons/external-link.js";
import FileText from "lucide-react/dist/esm/icons/file-text.js";
import Globe from "lucide-react/dist/esm/icons/globe.js";
import Search from "lucide-react/dist/esm/icons/search.js";
import Workflow from "lucide-react/dist/esm/icons/workflow.js";
import Wrench from "lucide-react/dist/esm/icons/wrench.js";
import { type ReactNode, useId, useState } from "react";
import { cn } from "../../../lib/utils";
import { getToolActivityLabel } from "../../../state/global-chat/tool-step-activity";
import type { IPlanStep, IToolActivityDetail } from "./chat-db";
import { ReasoningViewer } from "./reasoning-viewer";

type ActivitySource = NonNullable<IToolActivityDetail["sources"]>[number];

const ACTIVITY_ICONS = {
	web: Globe,
	search: Search,
	apps: AppWindow,
	files: FileText,
	database: Database,
	workflow: Workflow,
	tool: Wrench,
};

export function stepActivityTitle(step: IPlanStep): string {
	const isToolLabel =
		step.title === `Using ${step.toolName}` ||
		(step.detail?.kind === "tool_activity" &&
			/^Using [\w./:-]+$/.test(step.title)) ||
		step.title === step.toolName ||
		step.title === "Step" ||
		step.title === "Working";
	if (step.title && !isToolLabel) return step.title;
	if (step.detail?.kind === "tool_activity") return step.detail.label;
	if (step.toolName) {
		return getToolActivityLabel(step.toolName);
	}
	return step.title;
}

/** Use only a public hostname for the icon service, never the request path or query. */
function faviconUrl(hostname: string): string | undefined {
	const host = hostname.toLowerCase();
	if (
		!host.includes(".") ||
		/^[\d.]+$/.test(host) ||
		host.includes(":") ||
		/(?:^|\.)(?:localhost|local|internal|test|invalid|example|lan|home|onion)$/.test(
			host,
		)
	) {
		return undefined;
	}
	return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(host)}&sz=32`;
}

function WebsiteIcon({ source }: { source: ActivitySource }) {
	const url = faviconUrl(source.hostname);
	const [failedUrl, setFailedUrl] = useState<string>();
	return url && failedUrl !== url ? (
		<img
			src={url}
			alt=""
			width={16}
			height={16}
			loading="lazy"
			referrerPolicy="no-referrer"
			onError={() => setFailedUrl(url)}
			className="size-4 shrink-0 rounded-sm object-contain"
		/>
	) : (
		<Globe
			className="size-4 shrink-0 text-muted-foreground"
			aria-hidden="true"
		/>
	);
}

function SourceChip({ source }: { source: ActivitySource }) {
	const className =
		"group/source flex w-fit max-w-full items-center gap-1.5 rounded-md border border-border/60 bg-background/50 px-2 py-1 text-[11px] text-muted-foreground";
	const content = (
		<>
			<WebsiteIcon source={source} />
			<span className="min-w-0 truncate">
				<span className="font-medium text-foreground/80">
					{source.hostname}
				</span>
				{source.path && source.path !== "/" && (
					<span className="ml-1.5">{source.path}</span>
				)}
			</span>
		</>
	);
	if (source.linkable === false) {
		return (
			<span className={className} title={source.url}>
				{content}
			</span>
		);
	}
	return (
		<a
			href={source.url}
			target="_blank"
			rel="noopener noreferrer"
			title={source.url}
			className={cn(
				className,
				"transition-colors hover:bg-muted/50 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40",
			)}
		>
			{content}
			<ExternalLink
				className="size-3 shrink-0 opacity-50 group-hover/source:opacity-100"
				aria-hidden="true"
			/>
		</a>
	);
}

/** Keep the sites visible even after the completed activity group folds away. */
export function ActivitySources({ steps }: { steps: IPlanStep[] }) {
	const sources = new Map<string, ActivitySource>();
	for (const step of steps) {
		if (step.detail?.kind !== "tool_activity") continue;
		for (const source of step.detail.sources ?? []) {
			if (!sources.has(source.hostname)) sources.set(source.hostname, source);
		}
	}
	if (!sources.size) return null;
	const shown = [...sources.values()].slice(0, 3);
	return (
		<span className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-[11px] font-normal text-muted-foreground">
			{shown.map((source) => (
				<span
					key={source.hostname}
					className="inline-flex min-w-0 max-w-full items-center gap-1.5"
				>
					<WebsiteIcon source={source} />
					<span className="truncate">{source.hostname}</span>
				</span>
			))}
			{sources.size > shown.length && (
				<span>+{sources.size - shown.length}</span>
			)}
		</span>
	);
}

export function ToolActivityRow({
	step,
	detail,
	isActive,
	isExpanded,
	onToggle,
	statusIcon,
	duration,
}: {
	step: IPlanStep;
	detail: IToolActivityDetail;
	isActive: boolean;
	isExpanded: boolean;
	onToggle: (stepId: string) => void;
	statusIcon: ReactNode;
	duration: string | null;
}) {
	const Icon = ACTIVITY_ICONS[detail.category] ?? Wrench;
	const reasoningId = useId();
	const hasReasoning = Boolean(step.reasoning?.trim());
	const context = detail.query ?? detail.target;
	const description = step.description?.trim();
	const title = stepActivityTitle(step);
	const showDescription =
		description &&
		description !== context &&
		description !== title &&
		description !== detail.label &&
		description !== detail.outcome;
	return (
		<div
			className={cn(
				"min-w-0 rounded-lg px-2 py-2 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300",
				isActive && "bg-primary/5",
			)}
			data-tool-activity={detail.category}
		>
			<div className="flex min-w-0 items-start gap-2.5">
				<span
					className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-md border border-border/60 bg-muted/30"
					aria-hidden="true"
				>
					<Icon className="size-3.5 text-muted-foreground" />
				</span>
				<div className="min-w-0 flex-1 space-y-1.5">
					<div className="flex min-w-0 items-center gap-2 text-xs">
						<span
							className={cn(
								"min-w-0 flex-1 font-medium",
								step.status === "failed"
									? "text-muted-foreground"
									: "text-foreground/90",
							)}
						>
							{title}
						</span>
						{duration && (
							<span className="shrink-0 text-[10px] tabular-nums text-muted-foreground/70">
								{duration}
							</span>
						)}
						<span className="shrink-0">{statusIcon}</span>
					</div>
					{context && (
						<p
							className="line-clamp-2 break-words text-xs leading-relaxed text-muted-foreground [overflow-wrap:anywhere]"
							title={context}
						>
							{context}
						</p>
					)}
					{detail.sources?.map((source) => (
						<SourceChip key={source.url} source={source} />
					))}
					{showDescription && (
						<p
							className="line-clamp-2 break-words text-[11px] leading-relaxed text-muted-foreground [overflow-wrap:anywhere]"
							title={description}
						>
							{description}
						</p>
					)}
					{(detail.method || detail.outcome) && (
						<div className="flex flex-wrap items-center gap-2 text-[10px] text-muted-foreground/80">
							{detail.method && (
								<span className="rounded border border-border/60 px-1 font-mono">
									{detail.method}
								</span>
							)}
							{detail.outcome && (
								<span className="min-w-0 break-words [overflow-wrap:anywhere]">
									{detail.outcome}
								</span>
							)}
						</div>
					)}
					{hasReasoning && (
						<button
							type="button"
							onClick={() => onToggle(step.id)}
							aria-expanded={isExpanded}
							aria-controls={reasoningId}
							className="flex items-center gap-1 rounded text-[11px] text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
						>
							<ChevronDown
								className={cn(
									"size-3 transition-transform",
									isExpanded && "rotate-180",
								)}
								aria-hidden="true"
							/>
							Details
						</button>
					)}
					{isExpanded && hasReasoning && (
						<div id={reasoningId}>
							<ReasoningViewer
								reasoning={step.reasoning ?? ""}
								defaultExpanded
								compact
							/>
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
