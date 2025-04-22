"use client";

import { createId } from "@paralleldrive/cuid2";
import { useDebounce } from "@uidotdev/usehooks";
import { type Node, type NodeProps, useReactFlow } from "@xyflow/react";
import {
	AlignCenterVerticalIcon,
	AlignEndVerticalIcon,
	AlignHorizontalSpaceAroundIcon,
	AlignStartVerticalIcon,
	AlignVerticalJustifyCenterIcon,
	AlignVerticalJustifyEndIcon,
	AlignVerticalJustifyStartIcon,
	AlignVerticalSpaceAroundIcon,
	BanIcon,
	CircleXIcon,
	ClockIcon,
	CopyIcon,
	MessageSquareIcon,
	PlayCircleIcon,
	ScrollTextIcon,
	SquareCheckIcon,
	SquarePenIcon,
	TriangleAlertIcon,
	WorkflowIcon,
} from "lucide-react";
import { useTheme } from "next-themes";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import PuffLoader from "react-spinners/PuffLoader";
import { toast } from "sonner";
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
	IPinType,
	IValueType,
	handleCopy,
	updateNodeCommand,
	upsertPinCommand,
} from "../../lib";
import type { INode } from "../../lib/schema/flow/node";
import { type IPin, IVariableType } from "../../lib/schema/flow/pin";
import { ILogLevel, type ITrace } from "../../lib/schema/flow/run";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../lib/uint8";
import { useBackend } from "../../state/backend-state";
import { useRunExecutionStore } from "../../state/run-execution-state";
import {
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	Input,
	Label,
	Textarea,
} from "../ui";
import { DynamicImage } from "../ui/dynamic-image";
import { useUndoRedo } from "./flow-history";
import { FlowNodeCommentMenu } from "./flow-node/flow-node-comment-menu";
import { FlowPinAction } from "./flow-node/flow-node-pin-action";
import { FlowNodeRenameMenu } from "./flow-node/flow-node-rename-menu";
import { FlowPin } from "./flow-pin";
import { typeToColor } from "./utils";

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
		traces: ITrace[];
		onExecute: (node: INode, payload?: object) => Promise<void>;
		openTrace: (trace: ITrace[]) => Promise<void>;
	},
	"node"
>;

const FlowNodeInner = memo(
	({
		props,
		onHover,
	}: {
		props: NodeProps<FlowNode>;
		onHover: (hover: boolean) => void;
	}) => {
		const { pushCommand } = useUndoRedo(props.data.appId, props.data.boardId);
		const { resolvedTheme } = useTheme();
		const backend = useBackend();
		const invalidate = useInvalidateInvoke();
		const [payload, setPayload] = useState({
			open: false,
			payload: "",
		});
		const [executing, setExecuting] = useState(false);
		const [isExec, setIsExec] = useState(false);
		const [eventId, setEventId] = useState("");
		const [inputPins, setInputPins] = useState<(IPin | IPinAction)[]>([]);
		const [outputPins, setOutputPins] = useState<(IPin | IPinAction)[]>([]);
		const { runs } = useRunExecutionStore();
		const [executionState, setExecutionState] = useState<
			"done" | "running" | "none"
		>("none");
		const debouncedExecutionState = useDebounce(executionState, 100);
		const div = useRef<HTMLDivElement>(null);
		const reactFlow = useReactFlow();
		const { getNode } = useReactFlow();
		const severity = useMemo(() => {
			let severity = ILogLevel.Debug;

			for (const trace of props.data.traces) {
				for (const log of trace.logs) {
					if (severity === ILogLevel.Fatal) break;

					if (severity === ILogLevel.Error) {
						if (log.log_level === ILogLevel.Fatal) severity = ILogLevel.Fatal;
						continue;
					}

					if (severity === ILogLevel.Warn) {
						if (log.log_level === ILogLevel.Fatal) severity = ILogLevel.Fatal;
						if (log.log_level === ILogLevel.Error) severity = ILogLevel.Error;
						continue;
					}

					if (log.log_level === ILogLevel.Fatal) severity = ILogLevel.Fatal;
					if (log.log_level === ILogLevel.Error) severity = ILogLevel.Error;
					if (log.log_level === ILogLevel.Warn) severity = ILogLevel.Warn;
				}
			}

			return severity;
		}, [props.data.traces]);

		const eventRegistration = useMemo(() => {
			if (!props.data.node.start) return undefined;
			const typeNode = Object.values(props.data.node.pins).find(
				(pin) => pin.name === "type" && pin.pin_type === IPinType.Input,
			);
			if (!typeNode) return undefined;
			const otherPin = Object.values(props.data.node.pins).find(
				(pin) => pin.name !== "type" && pin.pin_type === IPinType.Input,
			);
			if (!otherPin) return undefined;
			const defaultValue = parseUint8ArrayToJson(otherPin.default_value);
			if (typeof defaultValue !== "undefined" && defaultValue !== null)
				return undefined;
			return {
				type: parseUint8ArrayToJson(typeNode.default_value),
				pin: otherPin,
			};
		}, [props.data.node]);

		const registerEvent = useCallback(async () => {
			if (!eventId) return;
			if (!eventRegistration) return;
			try {
				const node = props.data.node;
				const pin = eventRegistration.pin;
				const appId = props.data.appId;
				const boardId = props.data.boardId;
				const nodeId = props.data.node.id;
				const eventType = eventRegistration.type;

				await backend.registerEvent(appId, boardId, nodeId, eventType, eventId);

				const command = updateNodeCommand({
					node: {
						...node,
						pins: {
							...node.pins,
							[pin.id]: {
								...pin,
								default_value: convertJsonToUint8Array(eventId),
							},
						},
					},
				});

				const result = await backend.executeCommand(appId, boardId, command);
				await pushCommand(result, false);
				await invalidate(backend.getBoard, [appId, boardId]);
			} catch (e) {
				console.warn(e);
				toast.error(
					`Failed to register event. Try a different ${eventRegistration.pin.friendly_name}`,
				);
			}
		}, [
			backend,
			eventId,
			eventRegistration,
			props.data.appId,
			props.data.boardId,
			props.data.node,
			pushCommand,
			invalidate,
		]);

		const isReroute = useMemo(() => {
			return props.data.node.name === "reroute";
		}, [props.data.node.name]);

		const nodeStyle = useMemo(
			() => ({
				backgroundColor: props.selected
					? typeToColor(Object.values(props.data.node.pins)[0].data_type)
					: undefined,
				borderColor: typeToColor(
					Object.values(props.data.node.pins)[0].data_type,
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

			for (const [_, run] of runs) {
				if (run.nodes.has(props.id)) {
					isRunning = true;
					break;
				}

				if (run.already_executed.has(props.id)) {
					already_executed = true;
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

				const pins = Object.values(node.pins).sort(sortPins);
				pins.splice(index, 0, newPin);
				node.pins = {};
				pins.forEach(
					(pin, index) => (node.pins[pin.id] = { ...pin, index: index }),
				);

				const command = updateNodeCommand({
					node: {
						...node,
						coordinates: [nodeGuard.position.x, nodeGuard.position.y, 0],
					},
				});

				const result = await backend.executeCommand(
					props.data.appId,
					props.data.boardId,
					command,
				);

				await pushCommand(result, false);

				await invalidate(backend.getBoard, [
					props.data.appId,
					props.data.boardId,
				]);
			},
			[reactFlow],
		);

		const pinRemoveCallback = useCallback(
			async (pin: IPin) => {
				const nodeGuard = getNode(props.id);
				if (!nodeGuard) return;

				if (!props.data.node.pins) return;
				const node = nodeGuard?.data.node as INode;
				const pins = Object.values(node.pins)
					.filter((p) => p.id !== pin.id)
					.sort(sortPins);
				node.pins = {};
				pins.forEach(
					(pin, index) =>
						(props.data.node.pins[pin.id] = { ...pin, index: index }),
				);
				const command = updateNodeCommand({
					node: {
						...node,
						coordinates: [nodeGuard.position.x, nodeGuard.position.y, 0],
					},
				});

				const result = await backend.executeCommand(
					props.data.appId,
					props.data.boardId,
					command,
				);

				await pushCommand(result, false);

				await invalidate(backend.getBoard, [
					props.data.appId,
					props.data.boardId,
				]);
			},
			[inputPins, outputPins, getNode],
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

		return (
			<div
				key={`${props.id}__node`}
				ref={div}
				className={`bg-card p-2 react-flow__node-default selectable focus:ring-2 relative rounded-md group ${props.selected && "!border-primary border-2"} ${executionState === "done" ? "opacity-60" : "opacity-100"} ${isReroute && "w-4 max-w-4 !max-h-3 overflow-y !rounded-lg !p-[0.4rem]"}`}
				style={isReroute ? nodeStyle : {}}
				onMouseEnter={() => onHover(true)}
				onMouseLeave={() => onHover(false)}
			>
				{eventRegistration && (
					<Dialog open={typeof eventRegistration !== "undefined"}>
						<DialogContent>
							<DialogHeader>
								<DialogTitle>Event Registration</DialogTitle>
								<DialogDescription>
									The Node you placed requires an Event Configuration. Please
									provide a unique {eventRegistration.pin.friendly_name}.
									{eventRegistration.pin.description}
								</DialogDescription>
							</DialogHeader>
							<div className="grid w-full items-center gap-1.5">
								<Label htmlFor="eventid">
									{eventRegistration.pin.friendly_name}
								</Label>
								<Input
									id="eventid"
									value={eventId}
									onChange={(e) => setEventId(e.target.value)}
								/>
							</div>
							<Button onClick={async () => await registerEvent()}>
								Register Event
							</Button>
						</DialogContent>
					</Dialog>
				)}
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
					</div>
				)}
				{props.data.node.error && (
					<div className="absolute bottom-0 translate-y-[calc(100%+1rem)] left-3 right-3 mb-2 text-destructive-foreground bg-destructive p-1 rounded-md">
						<small className="font-normal text-extra-small leading-extra-small">
							{props.data.node.error}
						</small>
					</div>
				)}
				{useMemo(
					() =>
						!(props.data.node.start ?? false) &&
						inputPins
							.filter((pin) => isPinAction(pin) || pin.name !== "var_ref")
							.map((pin, index) =>
								isPinAction(pin) ? (
									<FlowPinAction
										key={`${pin.pin.id}__action`}
										action={pin}
										index={index}
										input
									/>
								) : (
									<FlowPin
										appId={props.data.appId}
										key={pin.id}
										node={props.data.node}
										boardId={props.data.boardId}
										index={index}
										pin={pin}
										onPinRemove={pinRemoveCallback}
									/>
								),
							),
					[inputPins, props.data.node, props.data.boardId, pinRemoveCallback],
				)}
				{!isReroute && (
					<div
						className={`header absolute top-0 left-0 right-0 h-4 gap-1 flex flex-row items-center border-b-1 border-b-foreground p-1 justify-between rounded-md rounded-b-none bg-card ${props.data.node.event_callback && "bg-gradient-to-l  from-card via-primary/50 to-primary"} ${!isExec && "bg-gradient-to-r  from-card via-tertiary/50 to-tertiary"} ${props.data.node.start && "bg-gradient-to-r  from-card via-primary/50 to-primary"} ${isReroute && "w-6"}`}
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
							{useMemo(() => {
								return props.data.traces.length > 0 ? (
									<ScrollTextIcon
										onClick={() => props.data.openTrace(props.data.traces)}
										className="w-2 h-2 cursor-pointer hover:text-primary"
									/>
								) : null;
							}, [props.data.traces.length, props.data.openTrace])}

							{useMemo(() => {
								if (!props.data.node.start || executing) return null;
								if (Object.keys(props.data.node.pins).length <= 1)
									return (
										<PlayCircleIcon
											className="w-2 h-2 cursor-pointer hover:text-primary"
											onClick={async (e) => {
												if (executing) return;
												setExecuting(true);
												await props.data.onExecute(props.data.node);
												setExecuting(false);
											}}
										/>
									);

								return (
									<Dialog
										open={payload.open}
										onOpenChange={(open) =>
											setPayload((old) => ({ ...old, open }))
										}
									>
										<DialogTrigger asChild>
											<PlayCircleIcon className="w-2 h-2 cursor-pointer hover:text-primary" />
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
								executing,
								props.data.onExecute,
								props.data.node,
							])}

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
				{useMemo(
					() =>
						outputPins.map((pin, index) =>
							isPinAction(pin) ? (
								<FlowPinAction
									action={pin}
									index={index}
									input={false}
									key={`${pin.pin.id}__action`}
								/>
							) : (
								<FlowPin
									appId={props.data.appId}
									node={props.data.node}
									boardId={props.data.boardId}
									index={index}
									pin={pin}
									key={pin.id}
									onPinRemove={pinRemoveCallback}
								/>
							),
						),
					[outputPins, props.data.node, props.data.boardId, pinRemoveCallback],
				)}
			</div>
		);
	},
);

function FlowNode(props: NodeProps<FlowNode>) {
	const backend = useBackend();
	const [isHovered, setIsHovered] = useState(false);
	const [isOpen, setIsOpen] = useState(false);
	const [commentMenu, setCommentMenu] = useState(false);
	const [renameMenu, setRenameMenu] = useState(false);
	const flow = useReactFlow();
	const { pushCommand } = useUndoRedo(props.data.appId, props.data.boardId);
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
		handleCopy(flow.getNodes());
	}, [flow]);

	const handleError = useCallback(async () => {
		const node = flow.getNodes().find((node) => node.id === props.id);
		if (!node) return;

		const innerNode = node.data.node as INode;

		const handleErrorPin = Object.values(innerNode.pins).find(
			(pin) =>
				pin.name === "auto_handle_error" && pin.pin_type === IPinType.Output,
		);

		if (handleErrorPin) {
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
			const updateNode = updateNodeCommand({
				node: {
					...innerNode,
				},
			});

			await backend.executeCommand(
				props.data.appId,
				props.data.boardId,
				updateNode,
			);
			await pushCommand(updateNode, false);
			invalidate(backend.getBoard, [props.data.appId, props.data.boardId]);
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

		await backend.executeCommand(props.data.appId, props.data.boardId, command);
		await backend.executeCommand(
			props.data.appId,
			props.data.boardId,
			stringCommand,
		);
		await pushCommand(command, false);
		await pushCommand(stringCommand, true);

		invalidate(backend.getBoard, [props.data.appId, props.data.boardId]);
	}, [backend, props.data.node, props.data.appId, props.data.boardId, flow]);

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
					{flow.getNodes().filter((node) => node.selected).length <= 1 && (
						<ContextMenuItem onClick={() => setCommentMenu(true)}>
							<div className="flex flex-row items-center gap-2 text-nowrap">
								<MessageSquareIcon className="w-4 h-4" />
								Comment
							</div>
						</ContextMenuItem>
					)}
					{isExec &&
						flow.getNodes().filter((node) => node.selected).length <= 1 && (
							<ContextMenuItem onClick={() => handleError()}>
								<div className="flex flex-row items-center gap-2 text-nowrap">
									<CircleXIcon className="w-4 h-4" />
									{errorHandled ? "Remove Handling" : "Handle Errors"}
								</div>
							</ContextMenuItem>
						)}
					<ContextMenuItem onClick={async () => await copy()}>
						<div className="flex flex-row items-center gap-2 text-nowrap">
							<CopyIcon className="w-4 h-4" />
							Copy
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
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignStartVerticalIcon className="w-4 h-4" />
											Start
										</div>
									</ContextMenuItem>
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignCenterVerticalIcon className="w-4 h-4" />
											Center
										</div>
									</ContextMenuItem>
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignEndVerticalIcon className="w-4 h-4" />
											End
										</div>
									</ContextMenuItem>
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignHorizontalSpaceAroundIcon className="w-4 h-4" />
											Space Around
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
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignVerticalJustifyStartIcon className="w-4 h-4" />
											Start
										</div>
									</ContextMenuItem>
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignVerticalJustifyCenterIcon className="w-4 h-4" />
											Center
										</div>
									</ContextMenuItem>
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignVerticalJustifyEndIcon className="w-4 h-4" />
											End
										</div>
									</ContextMenuItem>
									<ContextMenuItem>
										<div className="flex flex-row items-center gap-2 text-nowrap">
											<AlignVerticalSpaceAroundIcon className="w-4 h-4" />
											Space Around
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
			<FlowNodeInner props={props} onHover={(hover) => setIsHovered(hover)} />
		</>
	);
}

const node = memo(FlowNode);
export { node as FlowNode };
