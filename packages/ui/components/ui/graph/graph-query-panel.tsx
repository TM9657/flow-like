"use client";

import { useTranslation } from "@flow-like/locales";
import { Network, Play, Sparkles, Square, Table2, X } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import type {
	OntologyQueryLanguage,
	OntologyQueryLanguagePreference,
	OntologyQueryProposal,
	OntologyQueryReceipt,
	OntologyQueryStatusEvent,
} from "../../../lib/ontology-query";
import { Button } from "../button";
import { ScrollArea } from "../scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../tabs";
import { PropertyValue } from "./graph-node-inspector";

export interface GraphQueryPanelProps {
	onRunCypher: (query: string) => void;
	onRunQuery?: (proposal: OntologyQueryProposal) => void | Promise<void>;
	onAskFlowPilot?: (
		prompt: string,
		language: OntologyQueryLanguagePreference,
	) => Promise<unknown>;
	onCancelFlowPilot?: () => void;
	flowPilotStatus?: OntologyQueryStatusEvent | null;
	generatedProposal?: OntologyQueryProposal | null;
	receipt?: OntologyQueryReceipt | null;
	results: unknown[] | null;
	propertyMetadata?: Record<string, Record<string, string>>;
	loading?: boolean;
	error?: string | null;
	/**
	 * Puts the current results onto the canvas. Offered only when the rows
	 * could be resolved back into nodes and edges of this ontology.
	 */
	onAddToCanvas?: () => void;
	addToCanvasCount?: number;
	onClose?: () => void;
}

export function GraphQueryPanel({
	onRunCypher,
	onRunQuery,
	onAskFlowPilot,
	onCancelFlowPilot,
	flowPilotStatus,
	generatedProposal,
	receipt,
	results,
	propertyMetadata,
	loading,
	error,
	onAddToCanvas,
	addToCanvasCount,
	onClose,
}: GraphQueryPanelProps) {
	const { t } = useTranslation("common");
	const fieldId = useId();
	const promptInputId = `${fieldId}-prompt`;
	const preferenceInputId = `${fieldId}-preference`;
	const queryLanguageInputId = `${fieldId}-language`;
	const [prompt, setPrompt] = useState("");
	const [languagePreference, setLanguagePreference] =
		useState<OntologyQueryLanguagePreference>("auto");
	const [queryLanguage, setQueryLanguage] =
		useState<OntologyQueryLanguage>("cypher");
	const [query, setQuery] = useState("");
	const [params, setParams] = useState<Record<string, unknown>>({});
	const [activeTab, setActiveTab] = useState("table");
	const flowPilotBusy = flowPilotStatus != null;

	useEffect(() => {
		if (!generatedProposal) return;
		setQueryLanguage(generatedProposal.language);
		setQuery(generatedProposal.query);
		setParams(generatedProposal.params);
		setActiveTab(generatedProposal.presentation === "graph" ? "json" : "table");
	}, [generatedProposal]);
	const columns = useMemo(
		() => [
			...new Set(
				(results ?? []).flatMap((row) =>
					typeof row === "object" && row !== null ? Object.keys(row) : [],
				),
			),
		],
		[results],
	);

	const handleRun = useCallback(() => {
		const trimmed = query.trim();
		if (!trimmed) return;
		if (onRunQuery) {
			void onRunQuery({
				language: queryLanguage,
				query: trimmed,
				params,
				presentation: queryLanguage === "cypher" ? "graph" : "table",
			});
			return;
		}
		onRunCypher(trimmed);
	}, [onRunCypher, onRunQuery, params, query, queryLanguage]);

	const handleAskFlowPilot = useCallback(() => {
		const trimmed = prompt.trim();
		if (!trimmed || !onAskFlowPilot) return;
		void onAskFlowPilot(trimmed, languagePreference);
	}, [languagePreference, onAskFlowPilot, prompt]);

	const handleKeyDown = useCallback(
		(e: React.KeyboardEvent<HTMLTextAreaElement>) => {
			if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
				e.preventDefault();
				handleRun();
			}
		},
		[handleRun],
	);

	const handlePromptKeyDown = useCallback(
		(event: React.KeyboardEvent<HTMLInputElement>) => {
			if (event.key !== "Enter" || event.shiftKey) return;
			event.preventDefault();
			handleAskFlowPilot();
		},
		[handleAskFlowPilot],
	);

	const flowPilotStatusLabel = useMemo(() => {
		if (!flowPilotStatus) return null;
		const retry = flowPilotStatus.attempt > 1 ? " Retrying once." : "";
		switch (flowPilotStatus.phase) {
			case "loading-schema":
				return `${t("loadingOntologySchema", "Loading ontology schema...")}${retry}`;
			case "generating":
				return `${t("flowPilotIsWritingTheQuery", "FlowPilot is writing the query...")}${retry}`;
			case "validating":
				return `${t("validatingGeneratedQuery", "Validating generated query...")}${retry}`;
			case "running":
				return `${t("runningGeneratedQuery", "Running generated query...")}${retry}`;
		}
	}, [flowPilotStatus, t]);

	return (
		<div
			className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border bg-background"
			aria-busy={loading || flowPilotBusy || undefined}
		>
			<div
				className={`shrink-0 space-y-2 overflow-y-auto border-b p-2 sm:p-3 ${
					results && results.length > 0 ? "max-h-[60%]" : "max-h-full"
				}`}
			>
				<div className="flex items-center justify-between gap-2">
					<p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
						{t("ontologyQuery", "Ontology Query")}
					</p>
					<div className="flex items-center gap-1.5">
						{onAddToCanvas && (addToCanvasCount ?? 0) > 0 && (
							<Button
								size="sm"
								variant="outline"
								onClick={onAddToCanvas}
								title={t(
									"drawTheseResultsOnTheGraphCanvas",
									"Draw these results on the graph canvas",
								)}
							>
								<Network className="h-3.5 w-3.5 mr-1" />
								<span className="hidden sm:inline">
									{t("addCountToCanvas", "Add {{count}} to canvas", {
										count: addToCanvasCount,
									})}
								</span>
							</Button>
						)}
						{onClose && (
							<Button
								type="button"
								size="icon"
								variant="ghost"
								className="h-8 w-8"
								onClick={onClose}
								aria-label={t("closeQueryPanel", "Close query panel")}
							>
								<X className="h-4 w-4" />
							</Button>
						)}
					</div>
				</div>
				{onAskFlowPilot && (
					<div className="flex min-w-0 flex-wrap items-center gap-2">
						<label htmlFor={promptInputId} className="sr-only">
							{t("askThisOntology", "Ask this ontology")}
						</label>
						<input
							id={promptInputId}
							data-testid="ontology-natural-language-query"
							value={prompt}
							onChange={(event) => setPrompt(event.target.value)}
							onKeyDown={handlePromptKeyDown}
							placeholder={t(
								"askThisOntologyPlaceholder",
								"Ask this ontology in plain language...",
							)}
							className="h-9 min-w-48 flex-1 rounded-md border bg-background px-3 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
							disabled={flowPilotBusy}
						/>
						<label htmlFor={preferenceInputId} className="sr-only">
							{t("queryLanguage", "Query language")}
						</label>
						<select
							id={preferenceInputId}
							value={languagePreference}
							onChange={(event) =>
								setLanguagePreference(
									event.target.value as OntologyQueryLanguagePreference,
								)
							}
							className="h-9 rounded-md border bg-background px-2 text-xs focus:outline-none focus:ring-2 focus:ring-ring"
							disabled={flowPilotBusy}
						>
							<option value="auto">{t("auto", "Auto")}</option>
							<option value="cypher">Cypher</option>
							<option value="sql">SQL</option>
						</select>
						{flowPilotBusy ? (
							<Button
								type="button"
								size="sm"
								variant="outline"
								onClick={onCancelFlowPilot}
								disabled={!onCancelFlowPilot}
							>
								<Square className="mr-1 h-3.5 w-3.5" />
								{t("stop", "Stop")}
							</Button>
						) : (
							<Button
								type="button"
								size="sm"
								onClick={handleAskFlowPilot}
								disabled={loading || !prompt.trim()}
							>
								<Sparkles className="mr-1 h-3.5 w-3.5" />
								{t("askFlowPilot", "Ask FlowPilot")}
							</Button>
						)}
					</div>
				)}
				<div className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-start gap-2">
					<label htmlFor={queryLanguageInputId} className="sr-only">
						{t("generatedQueryLanguage", "Generated query language")}
					</label>
					<select
						id={queryLanguageInputId}
						value={queryLanguage}
						onChange={(event) => {
							setQueryLanguage(event.target.value as OntologyQueryLanguage);
							setParams({});
						}}
						className="h-10 rounded-md border bg-background px-2 text-xs focus:outline-none focus:ring-2 focus:ring-ring"
						disabled={!onRunQuery || loading || flowPilotBusy}
					>
						<option value="cypher">Cypher</option>
						{onRunQuery && <option value="sql">SQL</option>}
					</select>
					<textarea
						aria-label={t(
							"generatedOrManualQuery",
							"Generated or manual query",
						)}
						value={query}
						onChange={(event) => setQuery(event.target.value)}
						onKeyDown={handleKeyDown}
						wrap="off"
						placeholder="MATCH (n:Person)-[r]->(m) RETURN n, r, m LIMIT 100"
						className="h-10 min-h-10 max-h-20 w-full resize-y rounded-md border bg-muted/50 px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
						spellCheck={false}
					/>
					<Button
						size="sm"
						className="h-10"
						onClick={handleRun}
						disabled={loading || flowPilotBusy || !query.trim()}
						title={t("runQueryShortcut", "Run query (Ctrl/Cmd + Enter)")}
					>
						<Play className="mr-1 h-3.5 w-3.5" />
						{loading && !flowPilotBusy
							? t("running", "Running...")
							: t("run", "Run")}
					</Button>
				</div>
				{Object.keys(params).length > 0 && (
					<p
						className="truncate font-mono text-[11px] text-muted-foreground"
						title={JSON.stringify(params)}
					>
						{t("boundParameters", "Bound parameters")}: {JSON.stringify(params)}
					</p>
				)}
				{flowPilotStatusLabel && (
					<output
						className="block text-xs text-muted-foreground"
						aria-live="polite"
					>
						{flowPilotStatusLabel}
					</output>
				)}
				{receipt?.status === "success" && (
					<output
						className="block text-xs text-muted-foreground"
						aria-live="polite"
					>
						{t("queryResultSummary", "{{count}} rows in {{duration}} ms", {
							count: receipt.rowCount,
							duration: receipt.durationMs,
						})}
						{receipt.truncated
							? ` · ${t("resultTruncated", "Result truncated")}`
							: ""}
					</output>
				)}
			</div>
			{error && (
				<div
					className="shrink-0 border-b bg-destructive/10 px-3 py-2 text-xs text-destructive"
					role="alert"
				>
					{error}
				</div>
			)}
			{results && results.length > 0 && (
				<div className="min-h-0 flex-1" aria-live="polite">
					<Tabs
						value={activeTab}
						onValueChange={setActiveTab}
						className="h-full flex flex-col"
					>
						<TabsList className="mx-3 mt-2 w-fit">
							<TabsTrigger value="table" className="text-xs gap-1">
								<Table2 className="h-3.5 w-3.5" />
								{t("table", "Table")}
							</TabsTrigger>
							<TabsTrigger value="json" className="text-xs gap-1">
								<Network className="h-3.5 w-3.5" />
								JSON
							</TabsTrigger>
						</TabsList>
						<TabsContent value="table" className="m-0 min-h-0 flex-1 p-3">
							<ScrollArea className="h-full">
								<div className="min-w-max overflow-auto rounded border">
									<table className="w-full text-xs">
										<thead>
											<tr className="bg-muted/50">
												{columns.length > 0 ? (
													columns.map((key) => (
														<th
															key={key}
															className="px-3 py-2 text-left font-medium text-muted-foreground"
														>
															{key}
														</th>
													))
												) : (
													<th className="px-3 py-2 text-left font-medium text-muted-foreground">
														{t("value2", "Value")}
													</th>
												)}
											</tr>
										</thead>
										<tbody>
											{results.map((row, i) => (
												// biome-ignore lint/suspicious/noArrayIndexKey: Query rows have no unique ID and arrive as one result set.
												<tr key={i} className="border-t">
													{typeof row === "object" && row !== null ? (
														columns.map((key) => (
															<td
																key={key}
																className="px-3 py-1.5 min-w-[120px] max-w-[280px] align-top"
															>
																<PropertyValue
																	value={(row as Record<string, unknown>)[key]}
																	propKey={key}
																	metadata={propertyMetadata?.[key]}
																	compact
																/>
															</td>
														))
													) : (
														<td className="px-3 py-1.5">{String(row)}</td>
													)}
												</tr>
											))}
										</tbody>
									</table>
								</div>
							</ScrollArea>
						</TabsContent>
						<TabsContent value="json" className="m-0 min-h-0 flex-1 p-3">
							<ScrollArea className="h-full">
								<pre className="text-xs font-mono bg-muted/50 rounded p-3 whitespace-pre-wrap">
									{JSON.stringify(results, null, 2)}
								</pre>
							</ScrollArea>
						</TabsContent>
					</Tabs>
				</div>
			)}
			{results && results.length === 0 && (
				<div
					className="p-4 text-center text-sm text-muted-foreground"
					aria-live="polite"
				>
					{t("queryReturnedNoResults", "Query returned no results")}
				</div>
			)}
		</div>
	);
}
