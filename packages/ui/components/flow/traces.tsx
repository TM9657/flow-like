import { createId } from "@paralleldrive/cuid2";
import {
	BombIcon,
	CircleXIcon,
	FilterIcon,
	FilterXIcon,
	InfoIcon,
	ScrollIcon,
	TriangleAlertIcon,
} from "lucide-react";
import MiniSearch from "minisearch";
import { useEffect, useMemo, useRef, useState } from "react";
import { AutoSizer } from "react-virtualized";
import { parseTimespan } from "../../lib/date";
import type { INode } from "../../lib/schema/flow/node";
import {
	ILogLevel,
	type ILogMessage,
	type IRun,
	type ITrace,
} from "../../lib/schema/flow/run";
import "react-virtualized/styles.css";
import { VariableSizeList as List, type VariableSizeList } from "react-window";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Separator } from "../ui/separator";

export function Traces({
	traces,
	node,
	result,
	onOpenChange,
}: Readonly<{
	traces: ITrace[];
	node?: INode;
	result: IRun;
	onOpenChange: (open: boolean) => void;
}>) {
	const [traceFilter, setTraceFilter] = useState<ITrace | null>(null);
	const [logFilter, setLogFilter] = useState<Set<ILogLevel>>(
		new Set([
			ILogLevel.Debug,
			ILogLevel.Info,
			ILogLevel.Warn,
			ILogLevel.Error,
			ILogLevel.Fatal,
		]),
	);
	const [items, setItems] = useState<ILogMessage[]>([]);
	const [logs, setLogs] = useState<ILogMessage[]>([]);
	const [search, setSearch] = useState<string>("");
	const rowHeights = useRef(new Map());
	const listRef = useRef<VariableSizeList>(null);

	const miniSearch = useMemo(
		() =>
			new MiniSearch({
				fields: ["message", "operation_id", "log_level", "start", "end", "id"],
				storeFields: [
					"message",
					"operation_id",
					"log_level",
					"start",
					"end",
					"id",
				],
			}),
		[result],
	);

	function getRowHeight(index: number) {
		return (rowHeights.current.get(index) ?? 88) + 6;
	}

	useEffect(() => {
		const filteredTraces = traceFilter
			? result.traces.filter((trace) => traceFilter.id === trace.id)
			: result.traces;
		const logMessages = filteredTraces
			.flatMap((trace) => trace.logs)
			.filter((log) => logFilter.has(log.log_level))
			.sort((a, b) => a.start.nanos_since_epoch - b.start.nanos_since_epoch);

		miniSearch.removeAll();
		miniSearch.addAll(
			logMessages.map((log) => ({
				...log,
				id: log.start.nanos_since_epoch + createId(),
			})),
		);
		setLogs(logMessages);
	}, [traces, logFilter, traceFilter]);

	useEffect(() => {
		if (search === "") {
			setItems(logs);
			return;
		}

		const results = miniSearch.search(search);
		console.dir(results);

		setItems(results as any);
	}, [search, logs]);

	function render(props: any) {
		const { index, style } = props;
		const log = items[index];
		return (
			<LogMessage
				log={log}
				index={index}
				style={style}
				onSetHeight={(index, height) => setRowHeight(index, height)}
			/>
		);
	}

	function setRowHeight(index: number, height: number) {
		listRef.current?.resetAfterIndex(0);
		rowHeights.current = rowHeights.current.set(index, height);
	}

	if (traces.length === 0) return null;

	return (
		<div
			className={`transition-all top-0 bottom-0 right-0 h-[calc(100%-2rem)] z-10 bg-background border rounded-lg flex flex-col p-2 w-full`}
		>
			<div className="flex flex-row items-center justify-between w-full">
				<div>
					<p className="font-bold p-2">
						Execution Stats [{node?.friendly_name ?? "General"}]
					</p>
				</div>
			</div>
			<Separator className="my-1" />
			<div className="flex flex-row items-stretch overflow-hidden flex-grow h-full">
				<div>
					<div className="w-full p-2 bg-card rounded-md my-2">
						<h4>Board Stats</h4>
						<p>
							All node execution:{" "}
							<b>{parseTimespan(result.start, result.end)}</b>
						</p>
						<p>
							Execution Status: <b>{result.status}</b>
						</p>
						<p>
							Log Level: <b>{result.log_level}</b>
						</p>
					</div>
					<Separator className="my-1" />
					{node && (
						<div className="p-2 w-full">
							<h4 className="mb-2">Node Stats</h4>
							{traces
								.toSorted(
									(a, b) =>
										a.start.nanos_since_epoch - b.start.nanos_since_epoch,
								)
								.map((trace, index) => (
									<div key={trace.id} className="bg-card p-2 rounded-md mb-2">
										<div className="flex justify-between items-center gap-2">
											<div className="flex items-center">
												<div className="w-4 h-4 rounded-full bg-primary mr-2" />
												<div>
													<b>Run {index + 1}</b> [
													{parseTimespan(trace.start, trace.end)}] -{" "}
													<b>{trace.logs.length}</b> Log Entrie(s)
												</div>
											</div>
											<div className="flex items-center">
												<Button
													variant={"outline"}
													size={"icon"}
													onClick={() => {
														setTraceFilter((old) => {
															if (old) return null;
															return trace;
														});
													}}
												>
													{!traceFilter && <FilterIcon className="w-4 h-4" />}
													{traceFilter && <FilterXIcon className="w-4 h-4" />}
												</Button>
											</div>
										</div>
									</div>
								))}
						</div>
					)}
				</div>
				<div className="ml-2 flex flex-col w-full gap-1 overflow-x-hidden max-h-full flex-grow h-full">
					<div className="w-full flex flex-row items-center justify-between my-1">
						<div className="flex flex-row items-center gap-1">
							<Badge
								className="cursor-pointer"
								variant={logFilter.has(ILogLevel.Debug) ? "default" : "outline"}
								onClick={() =>
									setLogFilter((old) => {
										if (old.has(ILogLevel.Debug)) old.delete(ILogLevel.Debug);
										else old.add(ILogLevel.Debug);
										return new Set(old);
									})
								}
							>
								Debug
							</Badge>
							<Badge
								className="cursor-pointer"
								variant={logFilter.has(ILogLevel.Info) ? "default" : "outline"}
								onClick={() =>
									setLogFilter((old) => {
										if (old.has(ILogLevel.Info)) old.delete(ILogLevel.Info);
										else old.add(ILogLevel.Info);
										return new Set(old);
									})
								}
							>
								Info
							</Badge>
							<Badge
								className="cursor-pointer"
								variant={logFilter.has(ILogLevel.Warn) ? "default" : "outline"}
								onClick={() =>
									setLogFilter((old) => {
										if (old.has(ILogLevel.Warn)) old.delete(ILogLevel.Warn);
										else old.add(ILogLevel.Warn);
										return new Set(old);
									})
								}
							>
								Warning
							</Badge>
							<Badge
								className="cursor-pointer"
								variant={logFilter.has(ILogLevel.Error) ? "default" : "outline"}
								onClick={() =>
									setLogFilter((old) => {
										if (old.has(ILogLevel.Error)) old.delete(ILogLevel.Error);
										else old.add(ILogLevel.Error);
										return new Set(old);
									})
								}
							>
								Error
							</Badge>
							<Badge
								className="cursor-pointer"
								variant={logFilter.has(ILogLevel.Fatal) ? "default" : "outline"}
								onClick={() =>
									setLogFilter((old) => {
										if (old.has(ILogLevel.Fatal)) old.delete(ILogLevel.Fatal);
										else old.add(ILogLevel.Fatal);
										return new Set(old);
									})
								}
							>
								Debug
							</Badge>
						</div>
						<div className="flex flex-row items-center">
							<Input
								value={search}
								onChange={(e) => setSearch(e.target.value)}
								placeholder="Search..."
							/>
						</div>
					</div>
					<div className="flex flex-col w-full gap-1 overflow-x-auto max-h-full flex-grow h-full">
						<AutoSizer
							className="h-full flex-grow flex flex-col min-h-full"
							disableWidth
						>
							{({ height, width }) => (
								<List
									className="log-container h-full flex-grow flex flex-col"
									height={height}
									itemCount={items.length}
									itemSize={getRowHeight}
									ref={listRef}
									width={width}
								>
									{render}
								</List>
							)}
						</AutoSizer>
					</div>
				</div>
			</div>
		</div>
	);
}

function LogMessage({
	log,
	style,
	index,
	onSetHeight,
}: Readonly<{
	log: ILogMessage;
	style: any;
	index: number;
	onSetHeight: (index: number, height: number) => void;
}>) {
	const rowRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (rowRef.current) {
			onSetHeight(index, rowRef.current.clientHeight);
		}
	}, [rowRef]);

	return (
		<div style={style} className="scrollbar-gutter-stable">
			<div
				ref={rowRef}
				className={`flex flex-col items-center border rounded-md ${logLevelToColor(log.log_level)}`}
			>
				<div className="flex p-1 px-2  flex-row items-center gap-2 w-full">
					<LogIndicator logLevel={log.log_level} />
					<p>{log.message}</p>
				</div>
				{log.start.nanos_since_epoch !== log.end.nanos_since_epoch && (
					<div className="border-t w-full px-2 p-1 flex flex-row gap-1 items-center">
						<small className="text-xs">
							{parseTimespan(log.start, log.end)}
						</small>
						{log?.stats?.token_out && (
							<small className="text-xs">
								Token Out: {log.stats?.token_out}
							</small>
						)}
						{log?.stats?.token_in && (
							<small className="text-xs">Token In: {log.stats?.token_in}</small>
						)}
					</div>
				)}
			</div>
		</div>
	);
}

function logLevelToColor(logLevel: ILogLevel) {
	switch (logLevel) {
		case ILogLevel.Debug:
			return "bg-muted/20 text-muted-foreground";
		case ILogLevel.Info:
			return "bg-background/20";
		case ILogLevel.Warn:
			return "bg-yellow-400/20";
		case ILogLevel.Error:
			return "bg-rose-400/20";
		case ILogLevel.Fatal:
			return "bg-pink-400/30";
	}
}

function LogIndicator({ logLevel }: Readonly<{ logLevel: ILogLevel }>) {
	switch (logLevel) {
		case ILogLevel.Debug:
			return <ScrollIcon className="w-4 h-4 min-w-4" />;
		case ILogLevel.Info:
			return <InfoIcon className="w-4 h-4 min-w-4" />;
		case ILogLevel.Warn:
			return <TriangleAlertIcon className="w-4 h-4 min-w-4" />;
		case ILogLevel.Error:
			return <CircleXIcon className="w-4 h-4 min-w-4" />;
		case ILogLevel.Fatal:
			return <BombIcon className="w-4 h-4 min-w-4" />;
	}
}
