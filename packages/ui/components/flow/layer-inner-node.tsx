"use client";

import { createId } from "@paralleldrive/cuid2";
import { type Node, type NodeProps, useReactFlow } from "@xyflow/react";
import {
	ArrowDownIcon,
	ArrowUpIcon,
	FoldHorizontalIcon,
	GripVerticalIcon,
	MessageSquareIcon,
	PlusIcon,
	SaveIcon,
	SlidersHorizontalIcon,
	SquarePenIcon,
	Trash2Icon,
	ZapIcon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuLabel,
	ContextMenuSeparator,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import {
	type INode,
	type IPin,
	type IPinOptions,
	IValueType,
	IVariableType,
} from "../../lib";
import { type ILayer, IPinType } from "../../lib/schema/flow/board";
import {
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Input,
	Label,
	ScrollArea,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Separator,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../ui";
import { CommentDialog } from "./comment-dialog";
import { FlowPin } from "./flow-pin";
import { NameDialog } from "./name-dialog";

export enum InnerLayerNodeType {
	INPUT,
	RETURN
}

export type ILayerInnerNode = Node<
	{
		layer: ILayer;
		type: InnerLayerNodeType;
		boardId: string;
		hash: string;
		appId: string;
		pushLayer(layer: ILayer): Promise<void>;
		onLayerUpdate(layer: ILayer): Promise<void>;
		onLayerRemove(layer: ILayer, preserve_nodes: boolean): Promise<void>;
	},
	"layerInnerNode"
>;

export function LayerInnerNode(props: NodeProps<ILayerInnerNode>) {
	const divRef = useRef<HTMLDivElement>(null);
	const { getNodes } = useReactFlow();
	const [comment, setComment] = useState<string | undefined>();
	const [name, setName] = useState<string | undefined>();
	const [editing, setEditing] = useState(false);

	useEffect(() => {
		const height = Math.max(
			Object.values(props.data.layer.pins).filter(
				(pin) => pin.pin_type === IPinType.Input,
			).length,
			Object.values(props.data.layer.pins).filter(
				(pin) => pin.pin_type === IPinType.Output,
			).length,
		);

		if (divRef.current) {
			divRef.current.style.height = `calc(${height * 15}px + 1.25rem + 0.5rem)`;
			divRef.current.style.minHeight = "calc(15px + 1.25rem + 0.5rem)";
		}
	}, [props.data.hash]);

	const saveComment = useCallback(async () => {
		const node = getNodes().find((n) => n.id === props.id);
		if (!node) return;
		const layer = node.data.layer as ILayer;
		props.data.onLayerUpdate({ ...layer, comment: comment ?? "" });
		setComment(undefined);
	}, [props.id, comment]);

	const saveName = useCallback(async () => {
		const node = getNodes().find((n) => n.id === props.id);
		if (!node) return;
		const layer = node.data.layer as ILayer;
		props.data.onLayerUpdate({ ...layer, name: name ?? "Collapsed" });
		setName(undefined);
	}, [props.id, name]);

	return (
		<>
			{typeof comment === "string" && (
				<CommentDialog
					onOpenChange={(open) => {
						if (!open) {
							saveComment();
						}
					}}
					comment={comment}
					open={typeof comment === "string"}
					onUpsert={(comment) => setComment(comment)}
				/>
			)}
			{typeof name === "string" && (
				<NameDialog
					onOpenChange={(open) => {
						if (!open) {
							saveName();
						}
					}}
					name={name}
					open={typeof name === "string"}
					onUpsert={(name) => setName(name)}
				/>
			)}
			<ContextMenu>
				<ContextMenuTrigger>
					<div
						ref={divRef}
						key={`${props.data.hash}__node`}
						className={`p-1 flex flex-col justify-center items-center react-flow__node-default selectable focus:ring-2 relative bg-card! border-border! rounded-md! group ${props.selected && "border-primary! border-2"}`}
					>
						{props.data.layer.comment && props.data.layer.comment !== "" && (
							<div className="absolute top-0 translate-y-[calc(-100%-0.5rem)] left-3 right-3 mb-2 text-center bg-foreground/70 text-background p-1 rounded-md">
								<small className="font-normal text-extra-small leading-extra-small">
									{props.data.layer.comment}
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
						<div className="header absolute top-0 left-0 right-0 h-4 gap-1 flex flex-row items-center border-b p-1 justify-start rounded-t-md bg-accent! text-accent-foreground!">
							<ZapIcon className="w-2 h-2" />
							<small className="font-medium leading-none">
								"{props.data.layer.name}" {props.data.type === InnerLayerNodeType.INPUT ? "Start" : "Return"}
							</small>
						</div>
						{Object.values(props.data.layer.pins)
							.filter((pin) => pin.pin_type === IPinType.Output && props.data.type === InnerLayerNodeType.INPUT)
							.toSorted((a, b) => a.index - b.index)
							.map((pin) => (
								<FlowPin
									appId={props.data.appId}
									node={props.data.layer}
									boardId={props.data.boardId}
									pin={pin}
									key={pin.id}
									skipOffset={true}
									onPinRemove={async () => {}}
								/>
							))}
						{Object.values(props.data.layer.pins)
							.filter((pin) => pin.pin_type === IPinType.Input && props.data.type === InnerLayerNodeType.RETURN)
							.toSorted((a, b) => a.index - b.index)
							.map((pin) => (
								<FlowPin
									appId={props.data.appId}
									node={props.data.layer}
									boardId={props.data.boardId}
									pin={pin}
									key={pin.id}
									skipOffset={true}
									onPinRemove={async () => {}}
								/>
							))}
					</div>
				</ContextMenuTrigger>
				<ContextMenuContent className="max-w-20">
					<ContextMenuLabel>Layer Actions</ContextMenuLabel>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => {
							setName(props.data.layer.name ?? "");
						}}
					>
						<SquarePenIcon className="w-4 h-4" />
						Rename
					</ContextMenuItem>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => {
							setComment(props.data.layer.comment ?? "");
						}}
					>
						<MessageSquareIcon className="w-4 h-4" />
						Comment
					</ContextMenuItem>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => setEditing(true)}
					>
						<SlidersHorizontalIcon className="w-4 h-4" />
						Edit
					</ContextMenuItem>
					<ContextMenuSeparator />
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={async () => {
							await props.data.onLayerRemove(props.data.layer, true);
						}}
					>
						<FoldHorizontalIcon className="w-4 h-4" />
						Extend
					</ContextMenuItem>
					<ContextMenuSeparator />
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={async () => {
							await props.data.onLayerRemove(props.data.layer, false);
						}}
					>
						<Trash2Icon className="w-4 h-4" />
						Delete
					</ContextMenuItem>
				</ContextMenuContent>
			</ContextMenu>

			<LayerEditMenu
				open={editing}
				layer={props.data.layer}
				onOpenChange={setEditing}
				onApply={async (updated) => {
					const newLayer = {
						...props.data.layer,
						pins: updated.pins,
					};
					try {
						await props.data.onLayerUpdate(newLayer);
					} catch (error) {
						console.error(error);
						toast.error("Failed to update layer");
					}
					setEditing(false);
				}}
			/>
		</>
	);
}
function getNodes() {
	throw new Error("Function not implemented.");
}

type PinsRecord = ILayer["pins"];
type LayerPin = PinsRecord[keyof PinsRecord];

type PinEdit = {
	id: string;
	name: string;
	friendly_name: string;
	description: string;
	data_type: IVariableType;
	options?: IPinOptions | null;
	schema?: string | null;
	pin_type: IPinType;
	index: number;
	value_type: IValueType;
};

const normalizeValueType = (vt: any): IValueType => {
	const s = String(vt ?? "").toLowerCase();
	if (s === "array") return IValueType.Array;
	if (s === "hashmap" || s === "map") return IValueType.HashMap;
	if (s === "hashset" || s === "set") return IValueType.HashSet;
	return IValueType.Normal;
};

const toMachineName = (s: string) =>
	s.trim().toLowerCase().replace(/\s+/g, "_");

const buildInitialEdits = (layer: ILayer): Record<string, PinEdit> => {
	const out: Record<string, PinEdit> = {};
	for (const pin of Object.values(layer.pins)) {
		const p: any = pin;
		const friendly = p?.friendly_name ?? p?.name ?? pin.id;
		out[pin.id] = {
			id: pin.id,
			name: toMachineName(friendly),
			friendly_name: friendly,
			description: p?.description ?? "",
			data_type: p?.data_type ?? IVariableType.Generic,
			options: p?.options ?? null,
			schema: p?.schema ?? null,
			pin_type: pin.pin_type,
			index: pin.index ?? 1,
			value_type: normalizeValueType(p?.value_type ?? p?.valueType),
		};
	}
	return out;
};

const sortByIndex = <T extends { index: number }>(arr: T[]) =>
	[...arr].sort((a, b) => a.index - b.index);

const reindex = <T extends { index: number }>(arr: T[]) =>
	arr.map((p, i) => ({ ...p, index: i + 1 }));

const useGroupedPins = (edits: Record<string, PinEdit>) => {
	return useMemo(() => {
		const all = Object.values(edits);
		const inputs = sortByIndex(
			all.filter((p) => p.pin_type === IPinType.Input),
		);
		const outputs = sortByIndex(
			all.filter((p) => p.pin_type === IPinType.Output),
		);
		return { inputs, outputs };
	}, [edits]);
};

interface LayerEditMenuProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	layer: ILayer;
	onApply: (updated: ILayer) => Promise<void>;
}

const LayerEditMenu: React.FC<LayerEditMenuProps> = ({
	open,
	onOpenChange,
	layer,
	onApply,
}) => {
	const [edits, setEdits] = useState<Record<string, PinEdit>>(() =>
		buildInitialEdits(layer),
	);
	const { inputs, outputs } = useGroupedPins(edits);
	const [tab, setTab] = useState<"inputs" | "outputs">("inputs");

	useEffect(() => {
		if (open) {
			setEdits(buildInitialEdits(layer));
			setTab("inputs");
		}
	}, [open, layer]);

	const setPin = useCallback((id: string, updater: (p: PinEdit) => PinEdit) => {
		setEdits((prev) => {
			const curr = prev[id];
			if (!curr) return prev;
			return { ...prev, [id]: updater(curr) };
		});
	}, []);

	const editPin = useCallback(
		(id: string, patch: Partial<PinEdit>) => {
			setPin(id, (p) => {
				const next: PinEdit = { ...p, ...patch };
				// keep name derived from friendly_name
				if (patch.friendly_name !== undefined) {
					next.name = toMachineName(patch.friendly_name);
				}
				return next;
			});
		},
		[setPin],
	);

	const reindexGroupInState = useCallback((group: PinEdit[]) => {
		const re = reindex(group);
		setEdits((prev) => {
			const copy = { ...prev };
			for (const p of re) copy[p.id] = { ...copy[p.id], index: p.index };
			return copy;
		});
	}, []);

	const movePin = useCallback(
		(id: string, dir: "up" | "down") => {
			const group = edits[id]?.pin_type === IPinType.Input ? inputs : outputs;
			const idx = group.findIndex((p: any) => p.id === id);
			if (idx < 0) return;

			const nextIdx = dir === "up" ? idx - 1 : idx + 1;
			if (nextIdx < 0 || nextIdx >= group.length) return;

			const swapped = [...group];
			[swapped[idx], swapped[nextIdx]] = [swapped[nextIdx], swapped[idx]];
			reindexGroupInState(swapped);
		},
		[edits, inputs, outputs, reindexGroupInState],
	);

	const setIndex = useCallback(
		(id: string, newIndex: number) => {
			const pin = edits[id];
			if (!pin) return;
			const group = pin.pin_type === IPinType.Input ? inputs : outputs;
			const clamped = Math.max(1, Math.min(newIndex, group.length));

			const without = group.filter((p: any) => p.id !== id);
			const reordered = [
				...without.slice(0, clamped),
				pin,
				...without.slice(clamped),
			];
			reindexGroupInState(reordered);
		},
		[edits, inputs, outputs, reindexGroupInState],
	);

	const uniqueId = useCallback(
		(base: string) => {
			let n = 1;
			let id = `${base}_${n}`;
			const existing = new Set(Object.keys(edits));
			while (existing.has(id)) {
				n += 1;
				id = `${base}_${n}`;
			}
			return id;
		},
		[edits],
	);

	const addPin = useCallback(
		(pin_type: IPinType) => {
			setEdits((prev) => {
				const id = createId();
				const group = Object.values(prev).filter(
					(p) => p.pin_type === pin_type,
				);
				const next: PinEdit = {
					id,
					name: "New Pin",
					friendly_name: "new_pin",
					description: "",
					data_type: IVariableType.Generic,
					options: null,
					schema: null,
					pin_type,
					index: group.length,
					value_type: IValueType.Normal,
				};
				return { ...prev, [id]: next };
			});
		},
		[uniqueId],
	);

	const removePin = useCallback((id: string) => {
		setEdits((prev) => {
			const pin = prev[id];
			if (!pin) return prev;
			const copy = { ...prev };
			delete copy[id];

			// Reindex remaining pins in the same group
			const remaining = Object.values(copy).filter(
				(p) => p.pin_type === pin.pin_type,
			);
			const re = reindex(sortByIndex(remaining));
			for (const p of re) copy[p.id] = { ...copy[p.id], index: p.index };
			return copy;
		});
	}, []);

	const applyChanges = useCallback(async () => {
		const original = layer.pins;
		const nextPins: Record<string, IPin> = {};

		const zeroIndexed = Object.values(edits).find((p) => p.index <= 0);

		for (const edit of Object.values(edits)) {
			const prev = original[edit.id] as IPin | undefined;

			// Start from previous to preserve fields not configured here
			const merged: IPin = {
				...(prev as IPin),
				id: edit.id,
				pin_type: edit.pin_type,
				index: zeroIndexed ? (edit.index ?? 0) + 1 : (edit.index ?? 1),
				connected_to: prev?.connected_to ?? [],
				depends_on: prev?.depends_on ?? [],
				default_value: prev?.default_value ?? null,
				data_type: edit.data_type,
				description: edit.description ?? "",
				friendly_name: edit.friendly_name ?? edit.name,
				name: toMachineName(edit.friendly_name ?? edit.name),
				options: edit.options ?? null,
				schema: edit.schema ?? null,
				value_type: edit.value_type ?? IValueType.Normal,
			};

			nextPins[edit.id] = merged;
		}

		// Reindex per group to keep indexes contiguous
		const nextInputs = reindex(
			sortByIndex(
				Object.values(nextPins).filter((p) => p.pin_type === IPinType.Input),
			),
		);
		const nextOutputs = reindex(
			sortByIndex(
				Object.values(nextPins).filter((p) => p.pin_type === IPinType.Output),
			),
		);

		for (const p of nextInputs)
			nextPins[p.id] = { ...(nextPins[p.id] as IPin), index: p.index };
		for (const p of nextOutputs)
			nextPins[p.id] = { ...(nextPins[p.id] as IPin), index: p.index };

		const updated: ILayer = {
			...layer,
			pins: nextPins as unknown as ILayer["pins"],
		};

		await onApply(updated);
	}, [edits, layer, onApply]);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-5xl">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<SlidersHorizontalIcon className="h-5 w-5 text-primary" />
						Edit Layer Pins
					</DialogTitle>
					<DialogDescription>
						Configure pin properties and ordering for “{layer.name}”.
					</DialogDescription>
				</DialogHeader>

				<Tabs
					value={tab}
					onValueChange={(v) => setTab(v as any)}
					className="mt-2"
				>
					<TabsList className="grid grid-cols-2 w-full">
						<TabsTrigger value="inputs">Inputs</TabsTrigger>
						<TabsTrigger value="outputs">Outputs</TabsTrigger>
					</TabsList>

					<TabsContent value="inputs" className="mt-3 space-y-2">
						<div className="flex justify-end">
							<Button
								size="sm"
								onClick={() => addPin(IPinType.Input)}
								className="gap-2"
							>
								<PlusIcon className="h-4 w-4" />
								Add Input Pin
							</Button>
						</div>
						<PinList
							items={inputs}
							onEdit={editPin}
							onMoveUp={(id) => movePin(id, "up")}
							onMoveDown={(id) => movePin(id, "down")}
							onIndexChange={setIndex}
							onRemove={removePin}
						/>
					</TabsContent>

					<TabsContent value="outputs" className="mt-3 space-y-2">
						<div className="flex justify-end">
							<Button
								size="sm"
								onClick={() => addPin(IPinType.Output)}
								className="gap-2"
							>
								<PlusIcon className="h-4 w-4" />
								Add Output Pin
							</Button>
						</div>
						<PinList
							items={outputs}
							onEdit={editPin}
							onMoveUp={(id) => movePin(id, "up")}
							onMoveDown={(id) => movePin(id, "down")}
							onIndexChange={setIndex}
							onRemove={removePin}
						/>
					</TabsContent>
				</Tabs>

				<Separator className="my-3" />

				<DialogFooter className="gap-2">
					<Button variant="secondary" onClick={() => onOpenChange(false)}>
						Close
					</Button>
					<Button onClick={applyChanges} className="gap-2">
						<SaveIcon className="h-4 w-4" />
						Save
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
};

// Helpers for parsing user inputs
const toCSV = (arr?: string[] | null) =>
	arr && arr.length > 0 ? arr.join(", ") : "";
const fromCSVStrings = (s: string): string[] =>
	s
		.split(",")
		.map((x) => x.trim())
		.filter((x) => x.length > 0);
const fromCSVNumbers = (s: string): number[] =>
	s
		.split(",")
		.map((x) => Number(x.trim()))
		.filter((n) => Number.isFinite(n));

interface PinListProps {
	items: PinEdit[];
	onEdit: (id: string, patch: Partial<PinEdit>) => void;
	onMoveUp: (id: string) => void;
	onMoveDown: (id: string) => void;
	onIndexChange: (id: string, nextIndex: number) => void;
	onRemove: (id: string) => void;
}

const PinList: React.FC<PinListProps> = ({
	items,
	onEdit,
	onMoveUp,
	onMoveDown,
	onIndexChange,
	onRemove,
}) => {
	return (
		<ScrollArea className="h-96 max-h-96 overflow-auto rounded-md border">
			<div className="p-2 space-y-2">
				{items.length === 0 && (
					<div className="text-sm text-muted-foreground px-2 py-8 text-center">
						No pins in this group.
					</div>
				)}
				{items.map((pin, idx) => (
					<div
						key={pin.id}
						className="group rounded-md border bg-card hover:bg-accent/30 transition-colors"
					>
						<div className="flex items-center gap-2 px-2 py-1.5 border-b">
							<GripVerticalIcon className="h-4 w-4 text-muted-foreground" />
							<div className="flex-1 truncate text-sm font-medium">
								{pin.friendly_name ?? pin.name ?? pin.id}
								<span className="ml-2 text-xs text-muted-foreground">
									({pin.id})
								</span>
							</div>
							<div className="flex items-center gap-1">
								<PinOptionsButton
									pin={pin}
									onApply={(opts) => onEdit(pin.id, { options: opts })}
								/>
								<Button
									variant="ghost"
									size="icon"
									onClick={() => onMoveUp(pin.id)}
									disabled={idx === 0}
									title="Move up"
								>
									<ArrowUpIcon className="h-4 w-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									onClick={() => onMoveDown(pin.id)}
									disabled={idx === items.length - 1}
									title="Move down"
								>
									<ArrowDownIcon className="h-4 w-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									onClick={() => onRemove(pin.id)}
									title="Remove pin"
								>
									<Trash2Icon className="h-4 w-4 text-destructive" />
								</Button>
							</div>
						</div>

						<div className="grid grid-cols-1 md:grid-cols-3 gap-3 p-3">
							<div className="space-y-1.5">
								<Label className="text-xs">Pin Type</Label>
								<Select
									value={String(pin.pin_type)}
									onValueChange={(val) =>
										onEdit(pin.id, { pin_type: val as IPinType })
									}
								>
									<SelectTrigger className="h-8">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value={IPinType.Input}>Input</SelectItem>
										<SelectItem value={IPinType.Output}>Output</SelectItem>
									</SelectContent>
								</Select>
							</div>

							<div className="space-y-1.5">
								<Label className="text-xs">Value Type</Label>
								<Select
									value={pin.value_type}
									onValueChange={(val) =>
										onEdit(pin.id, { value_type: val as IValueType })
									}
								>
									<SelectTrigger className="h-8">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value={IValueType.Normal}>Normal</SelectItem>
										<SelectItem value={IValueType.Array}>Array</SelectItem>
										<SelectItem value={IValueType.HashMap}>HashMap</SelectItem>
										<SelectItem value={IValueType.HashSet}>HashSet</SelectItem>
									</SelectContent>
								</Select>
							</div>

							<div className="space-y-1.5">
								<Label className="text-xs">Index</Label>
								<Input
									className="h-8"
									type="number"
									min={0}
									value={pin.index}
									onChange={(e) => {
										const next = Number(e.target.value);
										if (Number.isFinite(next)) onIndexChange(pin.id, next);
									}}
								/>
							</div>

							<div className="space-y-1.5 md:col-span-1">
								<Label className="text-xs">Friendly Name</Label>
								<Input
									className="h-8"
									value={pin.friendly_name}
									onChange={(e) =>
										onEdit(pin.id, { friendly_name: e.target.value })
									}
								/>
								<small className="text-[10px] text-muted-foreground">
									Name will be saved as: {toMachineName(pin.friendly_name)}
								</small>
							</div>

							<div className="space-y-1.5 md:col-span-1">
								<Label className="text-xs">Schema</Label>
								<Input
									className="h-8"
									value={pin.schema ?? ""}
									onChange={(e) =>
										onEdit(pin.id, { schema: e.target.value || null })
									}
								/>
							</div>

							<div className="space-y-1.5 md:col-span-1">
								<Label className="text-xs">Data Type</Label>
								<Select
									value={pin.data_type}
									onValueChange={(val) =>
										onEdit(pin.id, { data_type: val as IVariableType })
									}
								>
									<SelectTrigger className="h-8">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value={IVariableType.Boolean}>
											Boolean
										</SelectItem>
										<SelectItem value={IVariableType.Byte}>Byte</SelectItem>
										<SelectItem value={IVariableType.Date}>Date</SelectItem>
										<SelectItem value={IVariableType.Execution}>
											Execution
										</SelectItem>
										<SelectItem value={IVariableType.Float}>Float</SelectItem>
										<SelectItem value={IVariableType.Generic}>
											Generic
										</SelectItem>
										<SelectItem value={IVariableType.Integer}>
											Integer
										</SelectItem>
										<SelectItem value={IVariableType.PathBuf}>
											PathBuf
										</SelectItem>
										<SelectItem value={IVariableType.String}>String</SelectItem>
										<SelectItem value={IVariableType.Struct}>Struct</SelectItem>
									</SelectContent>
								</Select>
							</div>

							<div className="space-y-1.5 md:col-span-3">
								<Label className="text-xs">Description</Label>
								<Input
									className="h-8"
									value={pin.description}
									onChange={(e) =>
										onEdit(pin.id, { description: e.target.value })
									}
								/>
							</div>

							{/* Removed: Default Value and Depends On from this editor */}
						</div>
					</div>
				))}
			</div>
		</ScrollArea>
	);
};

interface PinOptionsButtonProps {
	pin: PinEdit;
	onApply: (opts: IPinOptions | null) => void;
}

const PinOptionsButton: React.FC<PinOptionsButtonProps> = ({
	pin,
	onApply,
}) => {
	const [open, setOpen] = useState(false);
	const [local, setLocal] = useState<IPinOptions | null>(pin.options ?? null);

	useEffect(() => {
		if (open) setLocal(pin.options ?? null);
	}, [open, pin.options]);

	return (
		<>
			<Button
				variant="outline"
				size="sm"
				className="h-7 px-2"
				onClick={() => setOpen(true)}
				title="Edit pin options"
			>
				Options…
			</Button>
			<Dialog open={open} onOpenChange={setOpen}>
				<DialogContent className="sm:max-w-lg">
					<DialogHeader>
						<DialogTitle>Pin Options — {pin.friendly_name}</DialogTitle>
						<DialogDescription>Advanced, optional settings.</DialogDescription>
					</DialogHeader>

					<div className="grid grid-cols-1 md:grid-cols-6 gap-3">
						<div className="flex items-center gap-2 md:col-span-3">
							<input
								id={`opt-egvt-${pin.id}`}
								type="checkbox"
								className="h-4 w-4"
								checked={Boolean(local?.enforce_generic_value_type)}
								onChange={(e) =>
									setLocal({
										...(local ?? {}),
										enforce_generic_value_type: e.target.checked,
									} as IPinOptions)
								}
							/>
							<Label htmlFor={`opt-egvt-${pin.id}`} className="text-xs">
								Enforce Generic VT
							</Label>
						</div>

						<div className="flex items-center gap-2 md:col-span-3">
							<input
								id={`opt-es-${pin.id}`}
								type="checkbox"
								className="h-4 w-4"
								checked={Boolean(local?.enforce_schema)}
								onChange={(e) =>
									setLocal({
										...(local ?? {}),
										enforce_schema: e.target.checked,
									} as IPinOptions)
								}
							/>
							<Label htmlFor={`opt-es-${pin.id}`} className="text-xs">
								Enforce Schema
							</Label>
						</div>

						<div className="flex items-center gap-2 md:col-span-3">
							<input
								id={`opt-sens-${pin.id}`}
								type="checkbox"
								className="h-4 w-4"
								checked={Boolean(local?.sensitive)}
								onChange={(e) =>
									setLocal({
										...(local ?? {}),
										sensitive: e.target.checked,
									} as IPinOptions)
								}
							/>
							<Label htmlFor={`opt-sens-${pin.id}`} className="text-xs">
								Sensitive
							</Label>
						</div>

						<div className="space-y-1.5 md:col-span-3">
							<Label className="text-xs">Step</Label>
							<Input
								className="h-8"
								type="number"
								value={local?.step ?? ""}
								onChange={(e) =>
									setLocal({
										...(local ?? {}),
										step: e.target.value === "" ? null : Number(e.target.value),
									} as IPinOptions)
								}
							/>
						</div>

						<div className="space-y-1.5 md:col-span-3">
							<Label className="text-xs">Range Min</Label>
							<Input
								className="h-8"
								type="number"
								value={local?.range?.[0] ?? ""}
								onChange={(e) => {
									const min =
										e.target.value === "" ? undefined : Number(e.target.value);
									const max = local?.range?.[1];
									const nextRange = [
										Number.isFinite(min as number)
											? (min as number)
											: undefined,
										Number.isFinite(max as number)
											? (max as number)
											: undefined,
									].filter((x) => typeof x === "number") as number[];
									setLocal({
										...(local ?? {}),
										range:
											nextRange.length === 2
												? nextRange
												: nextRange.length === 1
													? [nextRange[0]]
													: null,
									} as IPinOptions);
								}}
							/>
						</div>

						<div className="space-y-1.5 md:col-span-3">
							<Label className="text-xs">Range Max</Label>
							<Input
								className="h-8"
								type="number"
								value={local?.range?.[1] ?? ""}
								onChange={(e) => {
									const min = local?.range?.[0];
									const max =
										e.target.value === "" ? undefined : Number(e.target.value);
									const nextRange = [
										Number.isFinite(min as number)
											? (min as number)
											: undefined,
										Number.isFinite(max as number)
											? (max as number)
											: undefined,
									].filter((x) => typeof x === "number") as number[];
									setLocal({
										...(local ?? {}),
										range:
											nextRange.length === 2
												? nextRange
												: nextRange.length === 1
													? [nextRange[0]]
													: null,
									} as IPinOptions);
								}}
							/>
						</div>

						<div className="space-y-1.5 md:col-span-6">
							<Label className="text-xs">Valid Values (comma-separated)</Label>
							<Input
								className="h-8"
								value={toCSV(local?.valid_values ?? null)}
								onChange={(e) =>
									setLocal({
										...(local ?? {}),
										valid_values:
											e.target.value.trim() === ""
												? null
												: fromCSVStrings(e.target.value),
									} as IPinOptions)
								}
							/>
						</div>
					</div>

					<DialogFooter className="gap-2">
						<Button variant="secondary" onClick={() => setOpen(false)}>
							Close
						</Button>
						<Button
							onClick={() => {
								onApply(local ?? null);
								setOpen(false);
							}}
						>
							Save
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</>
	);
};
