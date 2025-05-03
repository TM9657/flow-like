import { createId } from "@paralleldrive/cuid2";
import {
	BombIcon,
	CheckCircle2Icon,
	CircleXIcon,
	CopyIcon,
	CornerRightUpIcon,
	FilterIcon,
	FilterXIcon,
	InfoIcon,
	LogsIcon,
	ScrollIcon,
	SearchIcon,
	TriangleAlertIcon,
} from "lucide-react";
import MiniSearch from "minisearch";
import {
	type RefObject,
	memo,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
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
import { useDebounce } from "@uidotdev/usehooks";
import { useReactFlow } from "@xyflow/react";
import { VariableSizeList as List, type VariableSizeList } from "react-window";
import { toast } from "sonner";
import { type ILog, useBackend, useInvoke } from "../..";
import { logLevelToNumber } from "../../lib/log-level";
import { useLogAggregation } from "../../state/log-aggregation-state";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
	EmptyState,
} from "../ui";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Separator } from "../ui/separator";

interface IEnrichedLogMessage extends ILogMessage {
	node_id: string;
}

export function Traces({
	appId,
	boardId,
}: Readonly<{ appId: string; boardId: string }>) {
	const { fitView, updateNode, getNodes } = useReactFlow();
	const backend = useBackend();
	const { currentMetadata } = useLogAggregation();

	const [queryParts, setQueryParts] = useState<string[]>([]);
	const [query, setQuery] = useState("");
	const [limit, setLimit] = useState(1000);
	const [offset, setOffset] = useState(0);

	const [logFilter, setLogFilter] = useState<Set<ILogLevel>>(
		new Set([
			ILogLevel.Debug,
			ILogLevel.Info,
			ILogLevel.Warn,
			ILogLevel.Error,
			ILogLevel.Fatal,
		]),
	);

	const messages = useInvoke(
		backend.queryRun,
		[currentMetadata!, query, limit, offset],
		typeof currentMetadata !== "undefined",
	);

	const [search, setSearch] = useState<string>("");
	const debouncedSearch = useDebounce(search, 300);
	const rowHeights = useRef(new Map());
	const listRef = useRef<VariableSizeList>(null);

	useEffect(() => {
		const parts = [];

		if (logFilter.size > 0 && logFilter.size < 5) {
			parts.push(
				`log_level IN (${Array.from(logFilter)
					.map((level) => logLevelToNumber(level))
					.join(", ")})`,
			);
		}

		if (debouncedSearch.length > 0) {
			parts.push(`message LIKE '%${debouncedSearch}%'`);
		}

		setQueryParts(parts);
	}, [logFilter, debouncedSearch]);

	useEffect(() => {
		if (queryParts.length === 0) {
			setQuery("");
			return;
		}

		if (queryParts.length === 1) {
			setQuery(queryParts[0]);
			return;
		}

		let query = "";
		queryParts.forEach((part, index) => {
			if (index === 0) {
				query += `(${part})`;
			} else {
				query += ` AND (${part})`;
			}
		});
		setQuery(query);
	}, [queryParts]);

	function getRowHeight(index: number) {
		return (rowHeights.current.get(index) ?? 88) + 6;
	}

	function render(props: any) {
		if (!messages.data) return null;
		const { index, style } = props;
		const log = messages.data[index];
		return (
			<LogMessage
				key={index}
				log={log}
				index={index}
				style={style}
				onSetHeight={(index, height) => setRowHeight(index, height)}
				onSelectNode={(nodeId) => {
					console.log("select node", nodeId);
					const nodes = getNodes();

					nodes
						.filter((node) => node.selected && node.id !== nodeId)
						.forEach((node) => {
							updateNode(node.id, {
								selected: false,
							});
						});

					updateNode(nodeId, {
						selected: true,
					});

					fitView({
						nodes: [
							{
								id: nodeId,
							},
						],
					});
				}}
			/>
		);
	}

	function setRowHeight(index: number, height: number) {
		listRef.current?.resetAfterIndex(0);
		rowHeights.current = rowHeights.current.set(index, height);
	}

	return (
		<div
			className={
				"transition-all top-0 bottom-0 right-0 h-[calc(100%)] z-10 bg-background border rounded-lg flex flex-col p-2 w-full"
			}
		>
			<div className="flex flex-row items-stretch overflow-hidden flex-grow h-full">
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
						<div className="flex flex-row items-stretch">
							<Input
								value={search}
								onChange={(e) => setSearch(e.target.value)}
								placeholder="Search..."
							/>
						</div>
					</div>
					<div className="flex flex-col w-full gap-1 overflow-x-auto max-h-full flex-grow h-full">
						{(messages.data?.length ?? 0) === 0 && (
							<EmptyState
								className="h-full w-full max-w-full"
								icons={[LogsIcon, ScrollIcon, CheckCircle2Icon]}
								description="No logs found yet, start an event to see your results here!"
								title="No Logs"
							/>
						)}
						{(messages.data?.length ?? 0) > 0 && (
							<AutoSizer
								className="h-full flex-grow flex flex-col min-h-full"
								disableWidth
							>
								{({ height, width }) => (
									<List
										className="log-container h-full flex-grow flex flex-col"
										height={height}
										itemCount={messages.data?.length ?? 0}
										itemSize={getRowHeight}
										ref={listRef}
										width={width}
									>
										{render}
									</List>
								)}
							</AutoSizer>
						)}
					</div>
				</div>
			</div>
		</div>
	);
}

const LogMessage = memo(function LogMessage({
	log,
	style,
	index,
	onSetHeight,
	onSelectNode,
}: Readonly<{
	log: ILog;
	style: any;
	index: number;
	onSetHeight: (index: number, height: number) => void;
	onSelectNode: (nodeId: string) => void;
}>) {
	const rowRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (rowRef.current) {
			onSetHeight(index, rowRef.current.clientHeight);
		}
	}, [rowRef]);

	return (
		<button
			style={style}
			className="scrollbar-gutter-stable"
			onClick={(e) => e.preventDefault()}
		>
			<div
				ref={rowRef}
				className={`flex flex-col items-center border rounded-md ${logLevelToColor(log.log_level)}`}
			>
				<div className="flex p-1 px-2  flex-row items-center gap-2 w-full">
					<LogIndicator logLevel={log.log_level} />
					<p className="text-start text-wrap break-all">{log.message}</p>
				</div>
				<div className="flex flex-row items-center gap-1 w-full px-2 py-1 border-t justify-between">
					{log.start.nanos_since_epoch !== log.end.nanos_since_epoch ? (
						<div className="flex flex-row items-center">
							<small className="text-xs">
								{parseTimespan(log.start, log.end)}
							</small>
							{log?.stats?.token_out && (
								<small className="text-xs">
									Token Out: {log.stats?.token_out}
								</small>
							)}
							{log?.stats?.token_in && (
								<small className="text-xs">
									Token In: {log.stats?.token_in}
								</small>
							)}
						</div>
					) : (
						<div />
					)}
					<div className="flex flex-row items-center gap-1">
						<Button
							variant={"outline"}
							size={"icon"}
							className="!p-1 h-6 w-6"
							onClick={() => {
								navigator.clipboard.writeText(log.message);
								toast.success("Log message copied to clipboard");
							}}
						>
							<CopyIcon className="w-4 h-4" />
						</Button>
						{log.node_id && (
							<Button
								variant={"outline"}
								size={"icon"}
								className="!p-1 h-6 w-6"
								onClick={() => onSelectNode(log.node_id!)}
							>
								<CornerRightUpIcon className="w-4 h-4" />
							</Button>
						)}
					</div>
				</div>
			</div>
		</button>
	);
});

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
