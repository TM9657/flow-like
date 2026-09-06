"use client";

import { useTranslation } from "@flow-like/locales";
import { Network, Play, Table2, X } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Button } from "../button";
import { ScrollArea } from "../scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../tabs";
import { PropertyValue } from "./graph-node-inspector";

export interface GraphQueryPanelProps {
	onRunCypher: (query: string) => void;
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
	results,
	propertyMetadata,
	loading,
	error,
	onAddToCanvas,
	addToCanvasCount,
	onClose,
}: GraphQueryPanelProps) {
	const { t } = useTranslation("common");
	const [query, setQuery] = useState("");
	const [activeTab, setActiveTab] = useState("table");
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
		if (query.trim()) {
			onRunCypher(query.trim());
		}
	}, [query, onRunCypher]);

	const handleKeyDown = useCallback(
		(e: React.KeyboardEvent<HTMLTextAreaElement>) => {
			if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
				e.preventDefault();
				handleRun();
			}
		},
		[handleRun],
	);

	return (
		<div
			className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border bg-background"
			aria-busy={loading || undefined}
		>
			<div className="shrink-0 space-y-2 border-b p-2 sm:p-3">
				<div className="flex items-center justify-between gap-2">
					<p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
						{t("cypherQuery", "Cypher Query")}
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
								{t("addCountToCanvas", "Add {{count}} to canvas", {
									count: addToCanvasCount,
								})}
							</Button>
						)}
						<Button
							size="sm"
							onClick={handleRun}
							disabled={loading || !query.trim()}
							title={t("runQueryShortcut", "Run query (Ctrl/Cmd + Enter)")}
						>
							<Play className="h-3.5 w-3.5 mr-1" />
							{loading ? t("running", "Running...") : t("run", "Run")}
						</Button>
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
				<textarea
					aria-label={t("cypherQuery", "Cypher Query")}
					value={query}
					onChange={(e) => setQuery(e.target.value)}
					onKeyDown={handleKeyDown}
					placeholder="MATCH (n:Person)-[r]->(m) RETURN n, r, m LIMIT 100"
					className="min-h-16 max-h-28 w-full resize-none rounded-md border bg-muted/50 px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
					spellCheck={false}
				/>
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
