"use client";

import { useTranslation } from "@flow-like/locales";
import { ActivityIcon, SendIcon, SparklesIcon } from "lucide-react";
import { cn } from "../../../lib/utils";
import { ScopedCustomCss } from "../../scoped-custom-css";
import { Badge } from "../../ui/badge";
import { Button } from "../../ui/button";
import { Card } from "../../ui/card";
import { Input } from "../../ui/input";
import { Skeleton } from "../../ui/skeleton";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "../../ui/table";

export type AppearancePreviewView = "dashboard" | "chat";

export const PREVIEW_SCOPE = '[data-appearance-preview="1"]';

/** Runs per hour, midnight to 23:00, so the chart has a shape worth looking at. */
const SERIES = [38, 26, 19, 24, 57, 92, 118, 134, 148, 127, 96, 61];

/**
 * The surfaces the app stylesheet lands on, rendered from the same components a real app
 * page uses — the sheet is scoped to this element exactly as the runtime scopes it to the
 * app root, so every `[data-slot]` rule it writes hits here too.
 */
export function AppearancePreview({
	view,
	mode,
	sheet,
}: Readonly<{
	view: AppearancePreviewView;
	mode: "light" | "dark";
	sheet: string;
}>) {
	const { t } = useTranslation("flow");

	return (
		<div
			data-appearance-preview="1"
			className={cn(
				"relative isolate overflow-hidden rounded-xl border bg-background text-foreground",
				mode === "dark" && "dark",
			)}
		>
			<ScopedCustomCss
				css={sheet}
				scopeSelector={PREVIEW_SCOPE}
				options={{ scopeRoot: true }}
			/>

			<header className="flex items-center gap-4 border-b px-4 py-3">
				<span className="flex items-center gap-2 font-semibold text-sm">
					<span className="size-5 rounded-md bg-primary" />
					{t("appearancePreviewAppName", "Operations")}
				</span>
				<nav className="hidden gap-3 text-muted-foreground text-xs sm:flex">
					<span>{t("appearancePreviewNavOverview", "Overview")}</span>
					<span>{t("appearancePreviewNavRuns", "Runs")}</span>
					<span>{t("appearancePreviewNavSources", "Sources")}</span>
				</nav>
				<span className="flex-1" />
				<Button size="sm" className="h-7 text-xs">
					{t("appearancePreviewNewRun", "New run")}
				</Button>
			</header>

			{view === "dashboard" ? <DashboardSurface /> : <ChatSurface />}
		</div>
	);
}

function DashboardSurface() {
	const { t } = useTranslation("flow");
	const peak = Math.max(...SERIES);

	const tiles = [
		{
			label: t("appearancePreviewRuns", "Runs"),
			value: "1 284",
			delta: t("appearancePreviewRunsDelta", "+12.4 % vs. yesterday"),
			tone: "up" as const,
		},
		{
			label: t("appearancePreviewSuccess", "Success rate"),
			value: "98.2 %",
			delta: t("appearancePreviewSuccessDelta", "+0.6 pts"),
			tone: "up" as const,
		},
		{
			label: t("appearancePreviewLatency", "p95 latency"),
			value: "1.8 s",
			delta: t("appearancePreviewLatencyDelta", "+240 ms"),
			tone: "down" as const,
		},
		{
			label: t("appearancePreviewSpend", "Spend"),
			value: "41,20 €",
			delta: t("appearancePreviewSpendDelta", "of 120 € budget"),
			tone: "flat" as const,
		},
	];

	const runs = [
		{
			name: t("appearancePreviewRunIntake", "Invoice intake"),
			status: t("appearancePreviewStatusRunning", "Running"),
			variant: "default" as const,
			time: t("appearancePreviewTimeNow", "now"),
		},
		{
			name: t("appearancePreviewRunReindex", "Nightly reindex"),
			status: t("appearancePreviewStatusDone", "Done"),
			variant: "secondary" as const,
			time: "14:02",
		},
		{
			name: t("appearancePreviewRunSync", "CRM sync"),
			status: t("appearancePreviewStatusRetried", "Retried"),
			variant: "outline" as const,
			time: "13:41",
		},
		{
			name: t("appearancePreviewRunExtract", "PDF extract"),
			status: t("appearancePreviewStatusFailed", "Failed"),
			variant: "destructive" as const,
			time: "13:07",
		},
	];

	return (
		<div className="flex flex-col gap-3 p-4">
			<div className="flex items-end gap-3">
				<div>
					<h1 className="font-semibold text-lg tracking-tight">
						{t("appearancePreviewNavOverview", "Overview")}
					</h1>
					<p className="text-muted-foreground text-xs">
						{t("appearancePreviewSubtitle", "Last 24 hours · 4 workflows live")}
					</p>
				</div>
				<span className="flex-1" />
				<Button size="sm" variant="outline" className="h-7 text-xs">
					{t("appearancePreviewExport", "Export")}
				</Button>
			</div>

			<div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
				{tiles.map((tile) => (
					<Card key={tile.label} className="gap-1 p-3">
						<span className="font-medium text-[10px] text-muted-foreground uppercase tracking-wider">
							{tile.label}
						</span>
						<span className="font-semibold text-xl tabular-nums tracking-tight">
							{tile.value}
						</span>
						<span
							className={cn(
								"text-[11px]",
								tile.tone === "up" && "text-emerald-500",
								tile.tone === "down" && "text-amber-500",
								tile.tone === "flat" && "text-muted-foreground",
							)}
						>
							{tile.delta}
						</span>
					</Card>
				))}
			</div>

			<div className="grid gap-3 lg:grid-cols-[1.4fr_1fr]">
				<Card className="gap-3 p-3">
					<div className="flex items-baseline gap-2">
						<h2 className="font-semibold text-sm">
							{t("appearancePreviewRunsPerHour", "Runs per hour")}
						</h2>
						<span className="ml-auto text-[11px] text-muted-foreground tabular-nums">
							{t("appearancePreviewPeak", "peak {{count}} at 14:00", {
								count: peak,
							})}
						</span>
					</div>
					<div className="flex h-20 items-end gap-1">
						{SERIES.map((value, index) => (
							<span
								key={`${index * 2}:00`}
								className={cn(
									"flex-1 rounded-t-sm",
									value === peak ? "bg-primary" : "bg-primary/35",
								)}
								style={{ height: `${(value / peak) * 100}%` }}
							/>
						))}
					</div>
					<div className="flex justify-between text-[10px] text-muted-foreground tabular-nums">
						<span>00:00</span>
						<span>12:00</span>
						<span>23:00</span>
					</div>
				</Card>

				<Card className="gap-2 p-3">
					<h2 className="font-semibold text-sm">
						{t("appearancePreviewRecentRuns", "Recent runs")}
					</h2>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead className="h-7 text-[10px]">
									{t("appearancePreviewWorkflow", "Workflow")}
								</TableHead>
								<TableHead className="h-7 text-right text-[10px]">
									{t("appearancePreviewWhen", "When")}
								</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{runs.map((run) => (
								<TableRow key={run.name}>
									<TableCell className="py-1.5 text-xs">
										<span className="flex items-center gap-2">
											{run.name}
											<Badge
												variant={run.variant}
												className="h-4 px-1.5 font-normal text-[10px]"
											>
												{run.status}
											</Badge>
										</span>
									</TableCell>
									<TableCell className="py-1.5 text-right text-[11px] text-muted-foreground tabular-nums">
										{run.time}
									</TableCell>
								</TableRow>
							))}
							<TableRow>
								<TableCell className="py-1.5">
									<Skeleton className="h-3 w-2/3" />
								</TableCell>
								<TableCell className="py-1.5 text-right text-[11px] text-muted-foreground">
									{t("appearancePreviewQueued", "queued")}
								</TableCell>
							</TableRow>
						</TableBody>
					</Table>
				</Card>
			</div>
		</div>
	);
}

function ChatSurface() {
	const { t } = useTranslation("flow");

	return (
		<div className="flex min-h-[320px] flex-col gap-4 p-4">
			<div className="flex flex-col gap-3">
				<div className="ml-auto max-w-[78%] rounded-xl bg-primary px-3 py-2 text-primary-foreground text-xs leading-relaxed">
					{t(
						"appearancePreviewChatUser",
						"Which runs failed since noon, and why?",
					)}
				</div>
				<div className="flex flex-col gap-2">
					<Badge
						variant="outline"
						className="w-fit gap-1.5 font-normal text-[10px]"
					>
						<ActivityIcon className="size-3" />
						{t("appearancePreviewChatTool", "read run history · 42 rows")}
					</Badge>
					<p className="max-w-[85%] text-xs leading-relaxed">
						{t(
							"appearancePreviewChatReply",
							"One failure since 12:00. PDF extract stopped at 13:07 — the source file was 82 MB, over the 64 MB parser limit. Both CRM sync retries succeeded on the second attempt.",
						)}
					</p>
				</div>
			</div>

			<div className="flex flex-wrap gap-2">
				<Badge variant="secondary" className="font-normal text-[11px]">
					<SparklesIcon className="size-3" />
					{t("appearancePreviewChatChipRaise", "Raise the parser limit")}
				</Badge>
				<Badge variant="secondary" className="font-normal text-[11px]">
					{t("appearancePreviewChatChipShow", "Show the failing run")}
				</Badge>
			</div>

			<div className="mt-auto flex items-center gap-2">
				<Input
					readOnly
					className="h-9 text-xs"
					placeholder={t(
						"appearancePreviewChatPlaceholder",
						"Ask about a run…",
					)}
					aria-label={t("appearancePreviewChatPlaceholder", "Ask about a run…")}
				/>
				<Button size="sm" className="h-9 gap-1.5 text-xs">
					<SendIcon className="size-3.5" />
					{t("appearancePreviewChatSend", "Send")}
				</Button>
			</div>
		</div>
	);
}
