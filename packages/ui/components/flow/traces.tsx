import { useDebounce } from "@uidotdev/usehooks";
import {
	BombIcon,
	CheckCircle2Icon,
	CircleXIcon,
	CopyIcon,
	CornerRightUpIcon,
	InfoIcon,
	LogsIcon,
	ScrollIcon,
	TriangleAlertIcon,
} from "lucide-react";
import {
	type RefObject,
	memo,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { AutoSizer } from "react-virtualized";
import "react-virtualized/styles.css";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { VariableSizeList as List, type VariableSizeList } from "react-window";
import { toast } from "sonner";
import {
	type IBoard,
	type ILog,
	type INode,
	useBackend,
	useInfiniteInvoke,
} from "../..";
import { parseTimespan } from "../../lib/date";
import { logLevelToNumber } from "../../lib/log-level";
import { ILogLevel, type ILogMessage } from "../../lib/schema/flow/run";
import { useLogAggregation } from "../../state/log-aggregation-state";
import { DynamicImage, EmptyState } from "../ui";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Input } from "../ui/input";

interface IEnrichedLogMessage extends ILogMessage {
	node_id: string;
}

export function Traces({
	appId,
	boardId,
	board,
	onFocusNode,
}: Readonly<{
	appId: string;
	boardId: string;
	board: IBoard;
	onFocusNode: (nodeId: string) => void;
}>) {
	const backend = useBackend();
	const { currentMetadata } = useLogAggregation();
	const boardRef = useRef<IBoard>(board);

	const [queryParts, setQueryParts] = useState<string[]>([]);
	const [query, setQuery] = useState("");

	const [logFilter, setLogFilter] = useState<Set<ILogLevel>>(
		new Set([
			ILogLevel.Debug,
			ILogLevel.Info,
			ILogLevel.Warn,
			ILogLevel.Error,
			ILogLevel.Fatal,
		]),
	);

	const { data, hasNextPage, fetchNextPage, isFetchingNextPage } =
		useInfiniteInvoke(
			backend.boardState.queryRun,
			backend.boardState,
			[currentMetadata!, query],
			1000,
			typeof currentMetadata !== "undefined",
		);

	const messages = useMemo(() => {
		return data?.pages.flat() ?? [];
	}, [data]);
	const [search, setSearch] = useState<string>("");
	const debouncedSearch = useDebounce(search, 300);
	const rowHeights = useRef(new Map());
	const listRef = useRef<VariableSizeList>(null);

	const toggleLogFilter = useCallback((level: ILogLevel) => {
		setLogFilter((prev) => {
			const newFilter = new Set(prev);
			if (newFilter.has(level)) {
				newFilter.delete(level);
			} else {
				newFilter.add(level);
			}
			return newFilter;
		});
	}, []);

	const buildQuery = useCallback((parts: string[]) => {
		if (parts.length === 0) return "";
		if (parts.length === 1) return parts[0];
		return parts.map((part) => `(${part})`).join(" AND ");
	}, []);

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
		if (hasNextPage && index === (messages?.length ?? 0)) {
			return 50;
		}
		return (rowHeights.current.get(index) ?? 88) + 6;
	}

	const renderItem = useCallback(
		(props: any) => {
			if (!messages) return null;
			const { index, style } = props;

			if (hasNextPage && index === messages.length) {
				return (
					<div style={style} className="p-2">
						<Button
							className="w-full"
							onClick={async () => {
								if (isFetchingNextPage) return;
								await fetchNextPage();
							}}
							disabled={isFetchingNextPage}
						>
							Load more logs
						</Button>
					</div>
				);
			}

			const log = messages[index];
			return (
				<LogMessage
					key={index}
					log={log}
					index={index}
					style={style}
					board={boardRef}
					onSetHeight={setRowHeight}
					onSelectNode={onFocusNode}
				/>
			);
		},
		[messages, hasNextPage, isFetchingNextPage, fetchNextPage],
	);

	useEffect(() => {
		setQuery(buildQuery(queryParts));
	}, [queryParts, buildQuery]);

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
			<div className="flex flex-row items-stretch overflow-hidden grow h-full">
				<div className="ml-2 flex flex-col w-full gap-1 overflow-x-hidden max-h-full grow h-full">
					<div className="w-full flex flex-row items-center justify-between my-1">
						<div className="flex flex-row items-center gap-1">
							<LogFilterBadge
								level={ILogLevel.Debug}
								label="Debug"
								logFilter={logFilter}
								toggleLogFilter={toggleLogFilter}
							/>
							<LogFilterBadge
								level={ILogLevel.Info}
								label="Info"
								logFilter={logFilter}
								toggleLogFilter={toggleLogFilter}
							/>
							<LogFilterBadge
								level={ILogLevel.Warn}
								label="Warning"
								logFilter={logFilter}
								toggleLogFilter={toggleLogFilter}
							/>
							<LogFilterBadge
								level={ILogLevel.Error}
								label="Error"
								logFilter={logFilter}
								toggleLogFilter={toggleLogFilter}
							/>
							<LogFilterBadge
								level={ILogLevel.Fatal}
								label="Fatal"
								logFilter={logFilter}
								toggleLogFilter={toggleLogFilter}
							/>
						</div>

						<div className="flex flex-row items-stretch">
							<Input
								value={search}
								onChange={(e) => setSearch(e.target.value)}
								placeholder="Search..."
							/>
						</div>
					</div>
					<div className="flex flex-col w-full gap-1 overflow-x-auto max-h-full grow h-full">
						{(messages?.length ?? 0) === 0 && (
							<EmptyState
								className="h-full w-full max-w-full"
								icons={[LogsIcon, ScrollIcon, CheckCircle2Icon]}
								description="No logs found yet, start an event to see your results here!"
								title="No Logs"
							/>
						)}
						{(messages?.length ?? 0) > 0 && (
							<AutoSizer
								className="h-full grow flex flex-col min-h-full"
								disableWidth
							>
								{({ height, width }) => (
									<List
										className="log-container h-full grow flex flex-col"
										height={height}
										itemCount={(messages?.length ?? 0) + (hasNextPage ? 1 : 0)}
										itemSize={getRowHeight}
										ref={listRef}
										width={width}
									>
										{renderItem}
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
	board,
	onSetHeight,
	onSelectNode,
}: Readonly<{
	log: ILog;
	style: any;
	index: number;
	board: RefObject<IBoard>;
	onSetHeight: (index: number, height: number) => void;
	onSelectNode: (nodeId: string) => void;
}>) {
	const [node, setNode] = useState<INode | undefined>();
	const rowRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (log.node_id) {
			const node = board.current.nodes[log.node_id];
			setNode(node);
		}
	}, [log.node_id, board]);

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
						<div className="m-0! mr-2 p-0!">
							{!!node ? (
								<span className={`flex flex-row items-center gap-2`}>
									<DynamicImage
										url={node.icon ?? ""}
										className={`w-4 h-4 size-4 ${logLevelToColor(log.log_level, true)}`}
									/>
									{node.friendly_name || node.name}
								</span>
							) : (
								<span className="flex flex-row items-center gap-2">
									<QuestionMarkCircledIcon className="w-4 h-4 size-4" />
									Unknown Node
								</span>
							)}
						</div>
						<Button
							variant={"outline"}
							size={"icon"}
							className="p-1! h-6 w-6"
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
								className="p-1! h-6 w-6"
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

function logLevelToColor(logLevel: ILogLevel, icon = false) {
	switch (logLevel) {
		case ILogLevel.Debug:
			return !icon
				? "bg-muted/20 text-muted-foreground"
				: "bg-muted-foreground";
		case ILogLevel.Info:
			return !icon ? "bg-background/20" : "bg-foreground";
		case ILogLevel.Warn:
			return !icon ? "bg-yellow-400/20" : "bg-yellow-400";
		case ILogLevel.Error:
			return icon ? "bg-rose-400/20" : "bg-rose-400";
		case ILogLevel.Fatal:
			return !icon ? "bg-pink-400/30" : "bg-pink-400";
	}

	return icon ? "bg-foreground" : "bg-background";
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

function LogFilterBadge({
	level,
	label,
	logFilter,
	toggleLogFilter,
}: Readonly<{
	level: ILogLevel;
	label: string;
	logFilter: Set<ILogLevel>;
	toggleLogFilter: (level: ILogLevel) => void;
}>) {
	return (
		<Badge
			className="cursor-pointer"
			variant={logFilter.has(level) ? "default" : "outline"}
			onClick={() => toggleLogFilter(level)}
		>
			{label}
		</Badge>
	);
}
