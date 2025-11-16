"use client";
import { createId } from "@paralleldrive/cuid2";
import { useDebounce } from "@uidotdev/usehooks";
import {
	Handle,
	type Node,
	type NodeProps,
	Position,
	useReactFlow,
} from "@xyflow/react";
import {
	AlignCenterVerticalIcon,
	AlignEndVerticalIcon,
	AlignStartVerticalIcon,
	AlignVerticalJustifyCenterIcon,
	AlignVerticalJustifyEndIcon,
	AlignVerticalJustifyStartIcon,
	BanIcon,
	CircleStopIcon,
	CircleXIcon,
	ClockIcon,
	CopyIcon,
	FoldVerticalIcon,
	MessageSquareIcon,
	PlayCircleIcon,
	ScrollTextIcon,
	SlidersHorizontalIcon,
	SquareCheckIcon,
	SquarePenIcon,
	Trash2Icon,
	TriangleAlertIcon,
	WorkflowIcon,
} from "lucide-react";
import { useTheme } from "next-themes";
import {
	type RefObject,
	memo,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import PuffLoader from "react-spinners/PuffLoader";
import { useLogAggregation } from "../..";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuLabel,
	ContextMenuSeparator,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import { useInvalidateInvoke } from "../../hooks";
import {
	ILogLevel,
	IPinType,
	IValueType,
	moveNodeCommand,
	removeNodeCommand,
	updateNodeCommand,
	upsertLayerCommand,
	upsertPinCommand,
} from "../../lib";
import { logLevelFromNumber } from "../../lib/log-level";
import type { IBoard, IComment, ILayer } from "../../lib/schema/flow/board";
import { ILayerType } from "../../lib/schema/flow/board/commands/upsert-layer";
import type { INode } from "../../lib/schema/flow/node";
import { type IPin, IVariableType } from "../../lib/schema/flow/pin";
import { convertJsonToUint8Array } from "../../lib/uint8";
import { useBackendStore } from "../../state/backend-state";
import { useRunExecutionStore } from "../../state/run-execution-state";
import {
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	Textarea,
} from "../ui";
import { DynamicImage } from "../ui/dynamic-image";
import { useUndoRedo } from "./flow-history";
import { FlowNodeCommentMenu } from "./flow-node/flow-node-comment-menu";
import { FlowPinAction } from "./flow-node/flow-node-pin-action";
import { FlowNodeRenameMenu } from "./flow-node/flow-node-rename-menu";
import { FlowPin } from "./flow-pin";
import { LayerEditMenu } from "./layer-editing-menu";
import { typeToColor } from "./utils";

export interface RemoteSelectionParticipant {
	clientId: number;
	userId?: string;
	name: string;
	color: string;
}

export interface IPinAction {
	action: "create";
	pin: IPin;
	onAction: (pin: IPin) => Promise<void>;
}

export type FlowNode = Node<
	{
		hash: string;
		node: INode;
		boardId: string;
		appId: string;
		transparent?: boolean;
		boardRef: RefObject<IBoard | undefined>;
		version?: [number, number, number];
		onExecute: (node: INode, payload?: object) => Promise<void>;
		onCopy: () => Promise<void>;
		remoteSelections?: RemoteSelectionParticipant[];
	},
	"node"
>;

const FlowNodeInner = memo(
	({
		props,
		onHover,
	}: {
		props: NodeProps<FlowNode>;
		onHover: (hovered: boolean) => void;
	}) => {
		const { pushCommand } = useUndoRedo(props.data.appId, props.data.boardId);
		const { resolvedTheme } = useTheme();
		const invalidate = useInvalidateInvoke();
		const { currentMetadata } = useLogAggregation();

		const [payload, setPayload] = useState({
			open: false,
			payload: "",
		});
		const [executing, setExecuting] = useState(false);
		const [isExec, setIsExec] = useState(false);
		const [inputPins, setInputPins] = useState<(IPin | IPinAction)[]>([]);
		const [outputPins, setOutputPins] = useState<(IPin | IPinAction)[]>([]);
		const { runs } = useRunExecutionStore();
		const [executionState, setExecutionState] = useState<
			"done" | "running" | "none"
		>("none");
		const [runId, setRunId] = useState<string | undefined>(undefined);
		const debouncedExecutionState = useDebounce(executionState, 100);
		const div = useRef<HTMLDivElement>(null);
		const reactFlow = useReactFlow();
		const { getNode } = useReactFlow();
		const remoteSelections = props.data.remoteSelections ?? [];
		const displayedRemoteSelections = useMemo(
			() => remoteSelections.slice(0, 3),
			[remoteSelections],
		);
		const extraRemoteSelections =
			remoteSelections.length - displayedRemoteSelections.length;
		const [executed, severity] = useMemo(() => {
			const severity = ILogLevel.Debug;

			const nodeId = props.data.node.id;
			if (!currentMetadata) return [false, severity];
			const result = currentMetadata.nodes?.find(([localNodeId, severity]) => {
				if (localNodeId === nodeId) {
					return true;
				}
			}) as [string, number] | undefined;

			if (result) {
				return [true, logLevelFromNumber(result?.[1] ?? 0)];
			}

			return [false, severity];
		}, [props.data.node, currentMetadata]);

		const isReroute = useMemo(() => {
			return props.data.node.name === "reroute";
		}, [props.data.node.name]);

		const nodeStyle = useMemo(
			() => ({
				backgroundColor: props.selected
					? typeToColor(
							Object.values(props.data.node.pins)?.[0]?.data_type ??
								IVariableType.Generic,
						)
					: undefined,
				borderColor: typeToColor(
					Object.values(props.data.node.pins)?.[0]?.data_type ??
						IVariableType.Generic,
				),
				borderWidth: "1px",
				borderStyle: "solid",
			}),
			[isReroute, props.selected],
		);

		const sortPins = useCallback((a: IPin, b: IPin) => {
			// Step 1: Compare by type - Input comes before Output
			if (a.pin_type === "Input" && b.pin_type === "Output") return -1;
			if (a.pin_type === "Output" && b.pin_type === "Input") return 1;

			// Step 2: If types are the same, compare by index
			return a.index - b.index;
		}, []);

		useEffect(() => {
			const height = Math.max(inputPins.length, outputPins.length);
			if (isReroute) {
				return;
			}
			if (div.current)
				div.current.style.height = `calc(${height * 15}px + 1.25rem + 0.5rem)`;
		}, [isReroute, inputPins, outputPins]);

		useEffect(() => {
			parsePins(Object.values(props.data.node?.pins || []));
		}, [
			props.data.node.pins,
			props.positionAbsoluteX,
			props.positionAbsoluteY,
		]);

		useEffect(() => {
			let isRunning = false;
			let already_executed = false;

			for (const [runId, run] of runs) {
				if (run.nodes.has(props.id)) {
					isRunning = true;
					setRunId(runId);
					break;
				}

				if (run.already_executed.has(props.id)) {
					already_executed = true;
					setRunId(runId);
				}
			}

			if (isRunning) {
				setExecutionState("running");
				return;
			}

			if (already_executed) {
				setExecutionState("done");
				return;
			}

			setExecutionState("none");
		}, [runs, props.id]);

		const addPin = useCallback(
			async (node: INode, pin: IPin, index: number) => {
				if (typeof props.data.version !== "undefined") {
					return;
				}

				const backend = useBackendStore.getState().backend;
				if (!backend) return;
				const nodeGuard = reactFlow
					.getNodes()
					.find((node) => node.id === props.id);
				if (!nodeGuard) return;

				node = nodeGuard.data.node as INode;
				if (!node.pins) return;

				const newPin: IPin = {
					...pin,
					depends_on: [],
					connected_to: [],
					id: createId(),
					index: index,
				};

				const allPins = Object.values(node.pins);
				const inputPins = allPins
					.filter((p) => p.pin_type === "Input")
					.sort(sortPins);
				const outputPins = allPins
					.filter((p) => p.pin_type === "Output")
					.sort(sortPins);

				if (newPin.pin_type === "Input") {
					// Insert the new input pin at the specified index
					inputPins.splice(index - 1, 0, newPin); // Convert to 0-based index for splice

					// Update indices for input pins only, starting from the insertion point
					for (let i = index - 1; i < inputPins.length; i++) {
						inputPins[i].index = i + 1; // Convert back to 1-based index
					}
				} else {
					// Insert the new output pin at the specified index
					outputPins.splice(index - 1, 0, newPin); // Convert to 0-based index for splice

					// Update indices for output pins only, starting from the insertion point
					for (let i = index - 1; i < outputPins.length; i++) {
						outputPins[i].index = i + 1; // Convert back to 1-based index
					}
				}

				// Rebuild the pins object with updated pins
				node.pins = {};
				[...inputPins, ...outputPins].forEach((pin) => {
					node.pins[pin.id] = pin;
				});

				const command = updateNodeCommand({
					node: {
						...node,
						coordinates: [nodeGuard.position.x, nodeGuard.position.y, 0],
					},
				});

				const result = await backend.boardState.executeCommand(
					props.data.appId,
					props.data.boardId,
					command,
				);

				await pushCommand(result, false);

				await invalidate(backend.boardState.getBoard, [
					props.data.appId,
					props.data.boardId,
				]);
			},
			[reactFlow, sortPins, pushCommand, invalidate, props.data.version],
		);
		const pinRemoveCallback = useCallback(
			async (pinToRemove: IPin) => {
				if (typeof props.data.version !== "undefined") {
					return;
				}

				const backend = useBackendStore.getState().backend;
				if (!backend) return;

				const nodeGuard = getNode(props.id);
				if (!nodeGuard) return;

				if (!props?.data?.node?.pins) return;
				const node = nodeGuard?.data?.node as INode | undefined;
				if (!node) return;
				const allPins = Object.values(node.pins);

				const inputPins = allPins
					.filter((p) => p.pin_type === "Input" && p.id !== pinToRemove.id)
					.sort(sortPins)
					.map((p, i) => ({ ...p, index: i + 1 }));

				const outputPins = allPins
					.filter((p) => p.pin_type === "Output" && p.id !== pinToRemove.id)
					.sort(sortPins)
					.map((p, i) => ({ ...p, index: i + 1 }));

				const updatedPins: Record<string, IPin> = {};
				[...inputPins, ...outputPins].forEach((p) => {
					updatedPins[p.id] = p;
				});
				node.pins = updatedPins;

				const command = updateNodeCommand({
					node: {
						...node,
						coordinates: [nodeGuard.position.x, nodeGuard.position.y, 0],
					},
				});

				const result = await backend.boardState.executeCommand(
					props.data.appId,
					props.data.boardId,
					command,
				);

				await pushCommand(result, false);

				await invalidate(backend.boardState.getBoard, [
					props.data.appId,
					props.data.boardId,
				]);
			},
			[inputPins, outputPins, getNode, props.data.version],
		);

		const parsePins = useCallback(
			(pins: IPin[]) => {
				const inputPins: (IPin | IPinAction)[] = [];
				const outputPins: (IPin | IPinAction)[] = [];
				let isExec = false;

				let pastPinWithCount: [string, number, IPin | undefined] = [
					"",
					0,
					undefined,
				];

				Object.values(pins)
					.sort(sortPins)
					.forEach((pin, index) => {
						if (pin.data_type === "Execution") isExec = true;

						const pastPinId = `${pin.name}_${pin.pin_type}`;

						if (pastPinWithCount[0] === pastPinId) {
							pastPinWithCount[1] += 1;
						}

						if (pastPinWithCount[0] !== pastPinId && pastPinWithCount[1] > 0) {
							const action: IPinAction = {
								action: "create",
								pin: { ...pastPinWithCount[2]! },
								onAction: async (pin) => {
									await addPin(props.data.node, pin, index - 1);
								},
							};

							if (pastPinWithCount[2]?.pin_type === "Input") {
								inputPins.push(action);
							} else {
								outputPins.push(action);
							}
						}

						// update to past pin information
						if (pastPinWithCount[0] !== pastPinId)
							pastPinWithCount = [pastPinId, 0, pin];
						pin = { ...pin, dynamic: pastPinWithCount[1] > 1 };

						if (pin.pin_type === "Input") {
							inputPins.push(pin);
						} else {
							outputPins.push(pin);
						}
					});

				if (pastPinWithCount[1] > 0 && pastPinWithCount[2]) {
					const action: IPinAction = {
						action: "create",
						pin: { ...pastPinWithCount[2] },
						onAction: async (pin) => {
							await addPin(
								props.data.node,
								pin,
								Object.values(props.data.node?.pins || []).length,
							);
						},
					};

					if (pastPinWithCount[2].pin_type === "Input") {
						inputPins.push(action);
					} else {
						outputPins.push(action);
					}
				}

				setInputPins(inputPins);
				setOutputPins(outputPins);
				setIsExec(isExec);
			},
			[props.data.node.pins],
		);

		function isPinAction(pin: IPin | IPinAction): pin is IPinAction {
			return typeof (pin as IPinAction).onAction === "function";
		}

		const renderInputPins = useMemo(
			() =>
				!(props.data.node.start ?? false) &&
				inputPins
					.filter((pin) => isPinAction(pin) || pin.pin_type === "Input")
					.map((pin, arrayIndex) => {
						return isPinAction(pin) ? (
							<FlowPinAction
								key={`${pin.pin.id}__action`}
								action={pin}
								index={arrayIndex}
								input
							/>
						) : (
							<FlowPin
								appId={props.data.appId}
								key={pin.id}
								node={props.data.node}
								boardId={props.data.boardId}
								pin={pin}
								onPinRemove={pinRemoveCallback}
								skipOffset={isReroute}
								version={props.data.version}
							/>
						);
					}),
			[
				inputPins,
				props.data.node,
				props.data.boardId,
				pinRemoveCallback,
				isReroute,
				props.data.version,
			],
		);

		const renderOutputPins = useMemo(
			() =>
				outputPins.map((pin, arrayIndex) => {
					return isPinAction(pin) ? (
						<FlowPinAction
							action={pin}
							index={arrayIndex}
							input={false}
							key={`${pin.pin.id}__action`}
						/>
					) : (
						<FlowPin
							appId={props.data.appId}
							node={props.data.node}
							boardId={props.data.boardId}
							pin={pin}
							key={pin.id}
							onPinRemove={pinRemoveCallback}
							skipOffset={isReroute}
							version={props.data.version}
						/>
					);
				}),
			[
				outputPins,
				props.data.node,
				props.data.boardId,
				pinRemoveCallback,
				isReroute,
				props.data.version,
			],
		);

		const renderFnRefInputs = useMemo(() => {
			const canBeReferencedByFns =
				props.data.node.fn_refs?.can_be_referenced_by_fns ?? false;
			if (!canBeReferencedByFns) return null;

			return (
				<Handle
					position={Position.Top}
					type={"target"}
					className={`relative ml-auto right-0 z-50`}
					id={`ref_in_${props.data.node.id}`}
					style={{
						width: 12,
						height: 12,
						borderRadius: 2,
						background: `
					linear-gradient(
						135deg,
						var(--pin-fn-ref) 0%,
						color-mix(in oklch, var(--pin-fn-ref) 90%, white) 50%,
						var(--pin-fn-ref) 100%
					)
				`,
						border: "1px solid var(--pin-fn-ref)",
						padding: 0,
						boxShadow: `
			0 0 6px color-mix(in oklch, var(--pin-fn-ref) 30%, transparent),
			inset 0 1px 1px color-mix(in oklch, white 15%, transparent)
		`,
					}}
				/>
			);
		}, [props.data.node]);

		const renderFnRefOutputs = useMemo(() => {
			const canBeReferencedByFns =
				props.data.node.fn_refs?.can_reference_fns ?? false;
			if (!canBeReferencedByFns) return null;

			return (
				<Handle
					position={Position.Bottom}
					type={"source"}
					className={`relative z-50`}
					id={`ref_out_${props.data.node.id}`}
					style={{
						width: 12,
						height: 12,
						borderRadius: 2,
						background: `
				radial-gradient(
					circle at 30% 30%,
					color-mix(in oklch, var(--pin-fn-ref) 100%, white 20%),
					var(--pin-fn-ref) 70%
				)
			`,
						border: "1px solid var(--pin-fn-ref)",
						padding: 0,
						boxShadow: `
				0 0 8px color-mix(in oklch, var(--pin-fn-ref) 40%, transparent),
				0 1px 2px color-mix(in oklch, black 20%, transparent),
				inset 0 1px 1px color-mix(in oklch, white 20%, transparent)
			`,
					}}
				/>
			);
		}, [props.data.node]);
		const playNode = useMemo(() => {
			if (!props.data.node.start) return null;
			if (executionState === "done" || executing)
				return (
					<button
						className="bg-background hover:bg-card group/play transition-all rounded-md hover:rounded-lg border p-1 absolute left-0 top-0 translate-x-[calc(-120%)] opacity-200!"
						onClick={async (e) => {
							const backend = useBackendStore.getState().backend;
							if (!backend) return;
							if (runId) await backend.eventState.cancelExecution(runId);
						}}
					>
						<CircleStopIcon className="w-3 h-3 group-hover/play:scale-110 text-primary" />
					</button>
				);
			if (Object.keys(props.data.node.pins).length <= 1)
				return (
					<button
						className="bg-background hover:bg-card group/play transition-all rounded-md hover:rounded-lg border p-1 absolute left-0 top-0 translate-x-[calc(-120%)]"
						onClick={async (e) => {
							if (executing) return;
							setExecuting(true);
							await props.data.onExecute(props.data.node);
							setExecuting(false);
						}}
					>
						<PlayCircleIcon className="w-3 h-3 group-hover/play:scale-110" />
					</button>
				);

			return (
				<Dialog
					open={payload.open}
					onOpenChange={(open) => setPayload((old) => ({ ...old, open }))}
				>
					<DialogTrigger>
						<button className="bg-background hover:bg-card group/play transition-all rounded-md hover:rounded-lg border p-1 absolute left-0 top-0 translate-x-[calc(-120%)]">
							<PlayCircleIcon className="w-3 h-3 group-hover/play:scale-110" />
						</button>
					</DialogTrigger>
					<DialogContent>
						<DialogHeader>
							<DialogTitle>Execution Payload</DialogTitle>
							<DialogDescription>
								JSON Payload for the Event. Please have a look at the
								documentation for example Payloads.
							</DialogDescription>
						</DialogHeader>
						<Textarea
							rows={10}
							placeholder="JSON payload"
							value={payload.payload}
							onChange={(e) =>
								setPayload((old) => ({
									...old,
									payload: e.target.value,
								}))
							}
						/>
						<Button
							onClick={async () => {
								if (executing) return;
								setExecuting(true);
								await props.data.onExecute(
									props.data.node,
									JSON.parse(payload.payload),
								);
								setExecuting(false);
								setPayload((old) => ({ ...old, open: false }));
							}}
						>
							Send
						</Button>
					</DialogContent>
				</Dialog>
			);
		}, [
			props.data.node.start,
			payload,
			runId,
			executing,
			executionState,
			props.data.onExecute,
			props.data.node,
		]);

		return (
			<div
				key={`${props.id}__node`}
				ref={div}
				className={`bg-card! p-2 react-flow__node-default rounded-md! selectable focus:ring-2 relative group ${props.selected && "border-primary! border-2"} ${executionState === "done" ? "opacity-60" : "opacity-100"} ${isReroute && "w-4 max-w-4 max-h-3! overflow-y rounded-lg! p-[0.4rem]!"} ${!isReroute && "border-border!"}`}
				style={isReroute ? nodeStyle : {}}
				onMouseEnter={() => onHover(true)}
				onMouseLeave={() => onHover(false)}
			>
				{remoteSelections.length > 0 && (
					<div className="pointer-events-none absolute -top-3 left-0 flex flex-col gap-1">
						{displayedRemoteSelections.map((participant) => (
							<div
								key={`${participant.clientId}-${participant.userId ?? participant.name}`}
								className="flex items-center gap-1 rounded-md border bg-background/80 px-1.5 py-0.5 text-[0.625rem] leading-none shadow-sm"
								style={{ borderColor: participant.color }}
							>
								<span
									className="h-1.5 w-1.5 rounded-full"
									style={{ backgroundColor: participant.color }}
								/>
								<span className="font-medium">{participant.name}</span>
							</div>
						))}
						{extraRemoteSelections > 0 && (
							<div className="rounded-md border bg-background/80 px-1.5 py-0.5 text-[0.625rem] leading-none shadow-sm">
								+{extraRemoteSelections}
							</div>
						)}
					</div>
				)}
				{playNode}
				{props.data.node.long_running && (
					<div className="absolute top-0 z-10 translate-y-[calc(-50%)] translate-x-[calc(-50%)] left-0 text-center bg-background rounded-full">
						{useMemo(
							() => (
								<ClockIcon className="w-2 h-2 text-foreground" />
							),
							[],
						)}
					</div>
				)}
				{severity !== ILogLevel.Debug && (
					<div className="absolute top-0 z-10 translate-y-[calc(-50%)] translate-x-[calc(50%)] right-0 text-center bg-background rounded-full">
						{severity === ILogLevel.Fatal && (
							<BanIcon className="w-3 h-3 text-red-800" />
						)}
						{severity === ILogLevel.Error && (
							<CircleXIcon className="w-3 h-3 text-red-500" />
						)}
						{severity === ILogLevel.Warn && (
							<TriangleAlertIcon className="w-3 h-3 text-yellow-500" />
						)}
					</div>
				)}
				{props.data.node.comment && (
					<div className="absolute top-0 translate-y-[calc(-100%-0.5rem)] left-3 right-3 mb-2 text-center bg-foreground/70 text-background p-1 rounded-md">
						<small className="font-normal text-extra-small leading-extra-small">
							{props.data.node.comment}
						</small>
						<div
							className="
											absolute
											-bottom-1
											left-1/2
											transform -translate-x-1/2
											w-0 h-0
											border-l-4 border-l-transparent
											border-r-4 border-r-transparent
											border-t-4 border-t-foreground/70
										"
						/>
					</div>
				)}
				{props.data.node.error && (
					<div className="absolute bottom-0 translate-y-[calc(100%+1rem)] left-3 right-3 mb-2 text-destructive-foreground bg-destructive p-1 rounded-md">
						<small className="font-normal text-extra-small leading-extra-small">
							{props.data.node.error}
						</small>
					</div>
				)}
				{renderInputPins}
				{renderFnRefInputs}
				{renderFnRefOutputs}
				{!isReroute && (
					<div
						className={`header absolute top-0 left-0 right-0 h-4 gap-1 flex flex-row items-center border-b p-1 justify-between rounded-md rounded-b-none bg-card ${props.data.node.event_callback && "bg-linear-to-l  from-card via-primary/50 to-primary"} ${!isExec && "bg-linear-to-r  from-card via-tertiary/50 to-tertiary"} ${props.data.node.start && "bg-linear-to-r  from-card via-primary/50 to-primary"} ${isReroute && "w-6"}`}
					>
						<div className={"flex flex-row items-center gap-1"}>
							{useMemo(
								() =>
									props.data.node?.icon ? (
										<DynamicImage
											className="w-2 h-2 bg-foreground"
											url={props.data.node.icon}
										/>
									) : (
										<WorkflowIcon className="w-2 h-2" />
									),
								[props.data.node?.icon],
							)}
							<small className="font-medium leading-none text-start line-clamp-1">
								{props.data.node?.friendly_name}
							</small>
						</div>
						<div className="flex flex-row items-center gap-1">
							{executed && (
								<ScrollTextIcon
									// onClick={() => props.data.openTrace(props.data.traces)}
									className="w-2 h-2 cursor-pointer hover:text-primary"
								/>
							)}
							{useMemo(() => {
								if (debouncedExecutionState !== "running") return null;
								return (
									<PuffLoader
										color={resolvedTheme === "dark" ? "white" : "black"}
										size={10}
										speedMultiplier={1}
									/>
								);
							}, [debouncedExecutionState, resolvedTheme])}

							{useMemo(() => {
								return debouncedExecutionState === "done" ? (
									<SquareCheckIcon className="w-2 h-2 text-primary" />
								) : null;
							}, [debouncedExecutionState])}
						</div>
					</div>
				)}
				{renderOutputPins}
			</div>
		);
	},
);

function FlowNode(props: NodeProps<FlowNode>) {
	const [isHovered, setIsHovered] = useState(false);
	const [isOpen, setIsOpen] = useState(false);
	const [commentMenu, setCommentMenu] = useState(false);
	const [renameMenu, setRenameMenu] = useState(false);
	const [editingMenu, setEditingMenu] = useState(false);
	const flow = useReactFlow();
	const { pushCommand, pushCommands } = useUndoRedo(
		props.data.appId,
		props.data.boardId,
	);
	const invalidate = useInvalidateInvoke();
	const errorHandled = useMemo(() => {
		return Object.values(props.data.node.pins).some(
			(pin) =>
				pin.name === "auto_handle_error" && pin.pin_type === IPinType.Output,
		);
	}, [props.data.node.pins]);

	const isExec = useMemo(() => {
		return Object.values(props.data.node.pins).some(
			(pin) => pin.data_type === IVariableType.Execution,
		);
	}, [props.data.node.pins]);

	const copy = useCallback(async () => {
		props.data.onCopy();
	}, [flow]);

	const handleError = useCallback(async () => {
		if (typeof props.data.version !== "undefined") {
			return;
		}

		const node = flow.getNodes().find((node) => node.id === props.id);
		if (!node) return;

		const innerNode = node.data.node as INode;

		const handleErrorPin = Object.values(innerNode.pins).find(
			(pin) =>
				pin.name === "auto_handle_error" && pin.pin_type === IPinType.Output,
		);

		if (handleErrorPin) {
			const backend = useBackendStore.getState().backend;
			if (!backend) return;
			const filteredPins = Object.values(innerNode.pins).filter(
				(pin) =>
					pin.name !== "auto_handle_error" &&
					pin.name !== "auto_handle_error_string",
			);
			innerNode.pins = {};
			filteredPins
				.toSorted((a, b) => a.index - b.index)
				.forEach(
					(pin, index) => (innerNode.pins[pin.id] = { ...pin, index: index }),
				);
			let updateNode = updateNodeCommand({
				node: {
					...innerNode,
				},
			});

			updateNode = await backend.boardState.executeCommand(
				props.data.appId,
				props.data.boardId,
				updateNode,
			);
			await pushCommand(updateNode, false);
			invalidate(backend.boardState.getBoard, [
				props.data.appId,
				props.data.boardId,
			]);
			return;
		}

		const newPin: IPin = {
			name: "auto_handle_error",
			description: "Handles Node Errors for you.",
			pin_type: IPinType.Output,
			value_type: IValueType.Normal,
			data_type: IVariableType.Execution,
			id: createId(),
			index: 0,
			connected_to: [],
			depends_on: [],
			friendly_name: "On Error",
			default_value: convertJsonToUint8Array(false),
		};

		const stringPin: IPin = {
			name: "auto_handle_error_string",
			description: "Handles Node Errors for you.",
			pin_type: IPinType.Output,
			value_type: IValueType.Normal,
			data_type: IVariableType.String,
			id: createId(),
			index: 0,
			connected_to: [],
			depends_on: [],
			friendly_name: "Error",
			default_value: convertJsonToUint8Array(""),
		};

		const command = upsertPinCommand({
			node_id: innerNode.id,
			pin: newPin,
		});

		const stringCommand = upsertPinCommand({
			node_id: innerNode.id,
			pin: stringPin,
		});

		const backend = useBackendStore.getState().backend;
		if (!backend) return;

		const commands = await backend.boardState.executeCommands(
			props.data.appId,
			props.data.boardId,
			[command, stringCommand],
		);

		await pushCommands(commands);

		invalidate(backend.boardState.getBoard, [
			props.data.appId,
			props.data.boardId,
		]);
	}, [props.data.node, props.data.appId, props.data.boardId, flow]);

	const handleCollapse = useCallback(
		async (x: number, y: number) => {
			if (typeof props.data.version !== "undefined") {
				return;
			}

			const selectedNodes = flow.getNodes().filter((node) => node.selected);
			const flowCords = flow.screenToFlowPosition({
				x: x,
				y: y,
			});
			if (selectedNodes.length <= 1) return;

			const nodeIds = selectedNodes.map((node) => {
				const isNode = node.data.node as INode;
				if (isNode) return isNode.id;
				const isLayer = node.data.layer as ILayer;
				if (isLayer) return isLayer.id;
				const isComment = node.data.comment as IComment;
				if (isComment) return isComment.id;
				return "";
			});
			const command = upsertLayerCommand({
				layer: {
					id: createId(),
					comments: {},
					nodes: {},
					pins: {},
					parent_id: (selectedNodes[0].data.node as INode).layer,
					coordinates: [flowCords.x, flowCords.y, 0],
					in_coordinates: undefined,
					name: "Collapsed",
					type: ILayerType.Collapsed,
					variables: {},
				},
				node_ids: nodeIds,
				current_layer: (selectedNodes[0].data.node as INode).layer,
			});

			const backend = useBackendStore.getState().backend;
			if (!backend) return;

			const result = await backend.boardState.executeCommand(
				props.data.appId,
				props.data.boardId,
				command,
			);
			await pushCommand(result, false);
			await invalidate(backend.boardState.getBoard, [
				props.data.appId,
				props.data.boardId,
			]);
		},
		[props.data.node, invalidate, pushCommands, flow],
	);

	const deleteNodes = useCallback(async () => {
		if (typeof props.data.version !== "undefined") {
			return;
		}

		const nodes = flow.getNodes().filter((node) => node.selected);
		if (!nodes || nodes.length === 0) return;

		const commands = nodes.map((node) => {
			return removeNodeCommand({
				node: node.data.node as INode,
				connected_nodes: [],
			});
		});
		const backend = useBackendStore.getState().backend;
		if (!backend) return;
		const result = await backend.boardState.executeCommands(
			props.data.appId,
			props.data.boardId,
			commands,
		);
		setIsOpen(false);
		await pushCommands(result);
		await invalidate(backend.boardState.getBoard, [
			props.data.appId,
			props.data.boardId,
		]);
	}, [props.data.node, invalidate, pushCommands, flow]);

	const orderNodes = useCallback(
		async (type: "align" | "justify", dir: "start" | "end" | "center") => {
			if (typeof props.data.version !== "undefined") {
				return;
			}

			const selectedNodes = flow.getNodes().filter((node) => node.selected);
			if (selectedNodes.length <= 1) return;
			let currentLayer: string | undefined = undefined;

			let start = Number.POSITIVE_INFINITY;
			let end = Number.NEGATIVE_INFINITY;

			selectedNodes.forEach((node) => {
				const nodeData = node.data.node as INode;
				if (nodeData?.layer) currentLayer = nodeData.layer;

				start = Math.min(
					start,
					type === "align" ? node.position.x : node.position.y,
				);
				end = Math.max(
					end,
					type === "align" ? node.position.x : node.position.y,
				);
			});

			if (
				start === Number.POSITIVE_INFINITY ||
				end === Number.NEGATIVE_INFINITY
			)
				return;

			const center = (start + end) / 2;

			const commands = selectedNodes.map((node) => {
				return moveNodeCommand({
					node_id: node.id,
					from_coordinates: [node.position.x, node.position.y, 0],
					to_coordinates: [
						type === "align"
							? dir === "start"
								? start
								: dir === "end"
									? end
									: center
							: node.position.x,
						type === "align"
							? node.position.y
							: dir === "start"
								? start
								: dir === "end"
									? end
									: center,
						0,
					],
					current_layer: currentLayer,
				});
			});

			const backend = useBackendStore.getState().backend;
			if (!backend) return;

			const result = await backend.boardState.executeCommands(
				props.data.appId,
				props.data.boardId,
				commands,
			);

			pushCommands(result);
			await invalidate(backend.boardState.getBoard, [
				props.data.appId,
				props.data.boardId,
			]);
		},
		[props.data.node, invalidate, pushCommands, flow],
	);

	if (isOpen || isHovered) {
		return (
			<ContextMenu
				onOpenChange={(open) => {
					setIsOpen(open);
				}}
				key={props.id}
			>
				<ContextMenuTrigger>
					<FlowNodeInner props={props} onHover={setIsHovered} />
				</ContextMenuTrigger>
				<ContextMenuContent className="">
					<ContextMenuLabel>Node Actions</ContextMenuLabel>
					{flow.getNodes().filter((node) => node.selected).length <= 1 &&
						props.data.node.start && (
							<ContextMenuItem onClick={() => setRenameMenu(true)}>
								<div className="flex flex-row items-center gap-2 text-nowrap">
									<SquarePenIcon className="w-4 h-4" />
									Rename
								</div>
							</ContextMenuItem>
						)}
					{flow.getNodes().filter((node) => node.selected).length <= 1 &&
						props.data.node.name === "events_generic" && (
							<ContextMenuItem onClick={() => setEditingMenu(true)}>
								<div className="flex flex-row items-center gap-2 text-nowrap">
									<SlidersHorizontalIcon className="w-4 h-4" />
									Edit
								</div>
							</ContextMenuItem>
						)}
					{flow.getNodes().filter((node) => node.selected).length <= 1 && (
						<ContextMenuItem onClick={() => setCommentMenu(true)}>
							<div className="flex flex-row items-center gap-2 text-nowrap">
								<MessageSquareIcon className="w-4 h-4" />
								Comment
							</div>
						</ContextMenuItem>
					)}
					{flow.getNodes().filter((node) => node.selected).length > 1 && (
						<ContextMenuItem
							onClick={(e) => {
								e.preventDefault();
								const screenCoords = e.currentTarget.getBoundingClientRect();
								const x = screenCoords.x + screenCoords.width / 2;
								const y = screenCoords.y + screenCoords.height / 2;
								handleCollapse(x, y);
							}}
						>
							<div className="flex flex-row items-center gap-2 text-nowrap">
								<FoldVerticalIcon className="w-4 h-4" />
								Collapse
							</div>
						</ContextMenuItem>
					)}

					<ContextMenuItem onClick={async () => await copy()}>
						<div className="flex flex-row items-center gap-2 text-nowrap">
							<CopyIcon className="w-4 h-4" />
							Copy
						</div>
					</ContextMenuItem>
					{isExec &&
						flow.getNodes().filter((node) => node.selected).length <= 1 && (
							<>
								<ContextMenuSeparator />
								<ContextMenuItem onClick={() => handleError()}>
									<div className="flex flex-row items-center gap-2 text-nowrap">
										<CircleXIcon className="w-4 h-4" />
										{errorHandled ? "Remove Handling" : "Handle Errors"}
									</div>
								</ContextMenuItem>
							</>
						)}
					<ContextMenuSeparator />
					<ContextMenuItem onClick={async () => await deleteNodes()}>
						<div className="flex flex-row items-center gap-2 text-nowrap">
							<Trash2Icon className="w-4 h-4" />
							Delete
						</div>
					</ContextMenuItem>
					{flow.getNodes().filter((node) => node.selected).length > 1 && (
						<>
							<ContextMenuSeparator />
							<ContextMenuSub>
								<ContextMenuSubTrigger>
									<div className="flex flex-row items-center gap-2 text-nowrap">
										<AlignStartVerticalIcon className="w-4 h-4" />
										Align
									</div>
								</ContextMenuSubTrigger>
								<ContextMenuSubContent>
									<ContextMenuItem onClick={() => orderNodes("align", "start")}>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignStartVerticalIcon className="w-4 h-4" />
											Start
										</div>
									</ContextMenuItem>
									<ContextMenuItem
										onClick={() => orderNodes("align", "center")}
									>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignCenterVerticalIcon className="w-4 h-4" />
											Center
										</div>
									</ContextMenuItem>
									<ContextMenuItem onClick={() => orderNodes("align", "end")}>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignEndVerticalIcon className="w-4 h-4" />
											End
										</div>
									</ContextMenuItem>
								</ContextMenuSubContent>
							</ContextMenuSub>

							<ContextMenuSeparator />
							<ContextMenuSub>
								<ContextMenuSubTrigger>
									<div className="flex flex-row items-center gap-2 text-nowrap">
										<AlignVerticalJustifyStartIcon className="w-4 h-4" />
										Justify
									</div>
								</ContextMenuSubTrigger>
								<ContextMenuSubContent>
									<ContextMenuItem
										onClick={() => orderNodes("justify", "start")}
									>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignVerticalJustifyStartIcon className="w-4 h-4" />
											Start
										</div>
									</ContextMenuItem>
									<ContextMenuItem
										onClick={() => orderNodes("justify", "center")}
									>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignVerticalJustifyCenterIcon className="w-4 h-4" />
											Center
										</div>
									</ContextMenuItem>
									<ContextMenuItem onClick={() => orderNodes("justify", "end")}>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignVerticalJustifyEndIcon className="w-4 h-4" />
											End
										</div>
									</ContextMenuItem>
								</ContextMenuSubContent>
							</ContextMenuSub>
						</>
					)}
				</ContextMenuContent>
			</ContextMenu>
		);
	}

	return (
		<>
			{commentMenu && (
				<FlowNodeCommentMenu
					appId={props.data.appId}
					boardId={props.data.boardId}
					node={props.data.node}
					open={commentMenu}
					onOpenChange={(open) => setCommentMenu(open)}
				/>
			)}
			{renameMenu && (
				<FlowNodeRenameMenu
					appId={props.data.appId}
					boardId={props.data.boardId}
					node={props.data.node}
					open={renameMenu}
					onOpenChange={(open) => setRenameMenu(open)}
				/>
			)}
			{editingMenu && props.data.node.name === "events_generic" && (
				<LayerEditMenu
					open={editingMenu}
					onOpenChange={setEditingMenu}
					node={props.data.node}
					boardRef={props.data.boardRef}
					onApply={async (updated) => {
						const backend = useBackendStore.getState().backend;
						if (!backend) return;

						const currentNode = flow.getNode(props.id);
						if (!currentNode) return;

						const updatedNode = updated as INode;
						const command = updateNodeCommand({
							node: {
								...updatedNode,
								coordinates: [
									currentNode.position.x,
									currentNode.position.y,
									0,
								],
							},
						});

						const result = await backend.boardState.executeCommand(
							props.data.appId,
							props.data.boardId,
							command,
						);

						await pushCommand(result, false);
						await invalidate(backend.boardState.getBoard, [
							props.data.appId,
							props.data.boardId,
						]);
						setEditingMenu(false);
					}}
					mode="node"
				/>
			)}
			<FlowNodeInner props={props} onHover={setIsHovered} />
		</>
	);
}

const node = memo(FlowNode);
export { node as FlowNode };
