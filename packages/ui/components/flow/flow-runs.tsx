"use client";

import { useReactFlow } from "@xyflow/react";
import {
	BanIcon,
	CheckCircle2Icon,
	CircleXIcon,
	CornerRightUpIcon,
	EllipsisVerticalIcon,
	LogsIcon,
	RefreshCcwIcon,
	ScrollIcon,
	TriangleAlertIcon,
} from "lucide-react";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";
import { useInvoke } from "../../hooks";
import {
	type IBoard,
	ILogLevel,
	type ILogMetadata,
	type INode,
	formatDuration,
	formatRelativeTime,
} from "../../lib";
import { logLevelFromNumber } from "../../lib/log-level";
import { parseUint8ArrayToJson } from "../../lib/uint8";
import { useBackend } from "../../state/backend-state";
import { useLogAggregation } from "../../state/log-aggregation-state";
import {
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
	EmptyState,
} from "../ui";

export const FlowRuns = ({
	appId,
	boardId,
	nodes,
	executeBoard,
}: {
	appId: string;
	boardId: string;
	nodes: {
		[key: string]: INode;
	};
	executeBoard: (node: INode, payload?: object) => Promise<void>;
}) => {
	const backend = useBackend();
	const {
		currentMetadata,
		setCurrentMetadata,
		currentLogs,
		setFilter,
		refetchLogs,
	} = useLogAggregation();
	const { fitView } = useReactFlow();

	useEffect(() => {
		setFilter(backend, {
			appId,
			boardId,
			limit: 100,
		});
	}, [appId, boardId, backend, setFilter]);

	const zoomNode = useCallback(
		(nodeId: string) => {
			fitView({
				nodes: [
					{
						id: nodeId,
					},
				],
			});
		},
		[fitView],
	);

	return (
		<div className="flex flex-col gap-2 p-4 bg-background flex-grow h-full max-h-full overflow-hidden">
			<div className="flex flex-row items-center justify-between">
				<h3>Runs</h3>
				<Button
					variant={"outline"}
					size={"icon"}
					onClick={() => refetchLogs(backend)}
				>
					<RefreshCcwIcon className="w-4 h-4" />
				</Button>
			</div>
			{(!currentLogs || currentLogs.length === 0) && (
				<EmptyState
					className="mt-2 h-full"
					icons={[LogsIcon, ScrollIcon, CheckCircle2Icon]}
					description="No runs found yet, start an event to see your results here!"
					title="No Logs"
				/>
			)}
			<div className="flex flex-col gap-2 max-h-full overflow-y-auto">
				{currentLogs.map((run) => (
					<button
						key={run.run_id}
						className={`flex flex-row gap-2 items-center justify-between border p-2 rounded-md ${currentMetadata?.run_id === run.run_id ? "bg-muted/50" : "hover:bg-muted/50"}`}
						onClick={() => {
							if (currentMetadata?.run_id === run.run_id) {
								setCurrentMetadata(undefined);
								return;
							}

							setCurrentMetadata(run);
							zoomNode(run.node_id);
						}}
					>
						<div className="flex flex-col gap-2 items-start justify-center">
							<small className="leading-none">
								{nodes[run.node_id]?.friendly_name ?? "Deleted Event"}
							</small>
							<small className="text-muted-foreground leading-none">
								{formatRelativeTime(
									{
										nanos_since_epoch: run.start * 1000,
										secs_since_epoch: Math.floor(run.start / 1_000_000),
									},
									"narrow",
								)}
							</small>
						</div>
						<div className="flex flex-row items-center gap-2">
							<div className="flex flex-row gap-2 items-center">
								<small className="text-muted-foreground">
									{formatDuration(Math.abs(run.end - run.start))}
								</small>
								<div>
									{logLevelFromNumber(run.log_level) === ILogLevel.Debug && (
										<CheckCircle2Icon className="w-3 h-3 text-green-500" />
									)}
									{logLevelFromNumber(run.log_level) === ILogLevel.Info && (
										<CheckCircle2Icon className="w-3 h-3 text-green-500" />
									)}
									{logLevelFromNumber(run.log_level) === ILogLevel.Warn && (
										<TriangleAlertIcon className="w-3 h-3 text-yellow-500" />
									)}
									{logLevelFromNumber(run.log_level) === ILogLevel.Error && (
										<CircleXIcon className="w-3 h-3 text-red-500" />
									)}
									{logLevelFromNumber(run.log_level) === ILogLevel.Fatal && (
										<BanIcon className="w-3 h-3 text-red-800" />
									)}
								</div>
							</div>

							<DropdownMenu>
								<DropdownMenuTrigger>
									<Button
										size={"icon"}
										className="px-0 mx-0 w-4"
										variant={"ghost"}
									>
										<EllipsisVerticalIcon className="w-4 h-4" />
									</Button>
								</DropdownMenuTrigger>
								<DropdownMenuContent>
									<DropdownMenuLabel>Log Actions</DropdownMenuLabel>
									<DropdownMenuSeparator />
									<DropdownMenuItem
										onClick={() => {
											zoomNode(run.node_id);
										}}
										className="flex flex-row gap-2 items-center"
									>
										<CornerRightUpIcon className="w-4 h-4" />
										Go to Event
									</DropdownMenuItem>
									<DropdownMenuItem
										onClick={() => {
											const node = nodes[run.node_id];
											if (!node) {
												toast.error("Node not found");
												return;
											}
											executeBoard(node, parseUint8ArrayToJson(run.payload));
										}}
										className="flex flex-row gap-2 items-center"
									>
										<RefreshCcwIcon className="w-4 h-4" />
										Re-Run
									</DropdownMenuItem>
								</DropdownMenuContent>
							</DropdownMenu>
						</div>
					</button>
				))}
			</div>
		</div>
	);
};
