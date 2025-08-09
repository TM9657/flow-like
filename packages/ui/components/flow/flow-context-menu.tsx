import { SquarePlusIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useMiniSearch } from "react-minisearch";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import { type IBoard, doPinsMatch } from "../../lib";
import type { INode } from "../../lib/schema/flow/node";
import type { IPin } from "../../lib/schema/flow/pin";
import { convertJsonToUint8Array } from "../../lib/uint8";
import { Checkbox } from "../ui/checkbox";
import { Input } from "../ui/input";
import { ScrollArea } from "../ui/scroll-area";
import { Separator } from "../ui/separator";
import { FlowContextMenuNodes } from "./flow-context-menu-nodes";

export function FlowContextMenu({
	nodes,
	board,
	refs,
	children,
	droppedPin,
	onNodePlace,
	onCommentPlace,
	onClose,
}: Readonly<{
	nodes: INode[];
	board: IBoard | undefined;
	refs: { [key: string]: string };
	children: React.ReactNode;
	droppedPin?: IPin;
	onNodePlace: (node: INode) => void;
	onCommentPlace: () => void;
	onClose: () => void;
}>) {
	const inputRef = useRef<HTMLInputElement>(null);
	const [filter, setFilter] = useState("");
	const [contextSensitive, setContextSensitive] = useState(true);
	const sortedNodes = useMemo(() => {
		if (!nodes) return [];

		let callRefNode: INode | undefined = undefined;
		let variableGetNode: INode | undefined = undefined;
		let variableSetNode: INode | undefined = undefined;

		const normalNodes =
			nodes
				// .filter(
				// 	(node) => node.name !== "variable_set" && node.name !== "variable_get",
				// )
				.toSorted((a, b) => {
					// Counter Intuitive, but we save one iteration by doing this
					if (a.name === "control_call_reference") {
						callRefNode = a;
					}

					if (a.name === "variable_get") {
						variableGetNode = a;
					}

					if (a.name === "variable_set") {
						variableSetNode = a;
					}

					if (a.friendly_name === b.friendly_name) {
						return a.category.localeCompare(b.category);
					}
					return a.friendly_name.localeCompare(b.friendly_name);
				}) ?? [];

		if (board && callRefNode) {
			Object.values(board.nodes).forEach((node) => {
				if (!node.start) return;
				const pins = Object.values(callRefNode?.pins ?? {}).map((pin) =>
					pin.name === "fn_ref"
						? { ...pin, default_value: convertJsonToUint8Array(node.id) }
						: pin,
				);
				const newPins = Object.fromEntries(pins.map((pin) => [pin.id, pin]));

				normalNodes.push({
					...(callRefNode as INode),
					pin_in_names: Object.values(newPins)
						.filter((pin) => pin.pin_type === "Input")
						.map((pin) => pin.friendly_name),
					pin_out_names: Object.values(newPins)
						.filter((pin) => pin.pin_type === "Output")
						.map((pin) => pin.friendly_name),
					friendly_name: `Call ${node.friendly_name}`,
					category: "Events/Call",
					pins: newPins,
				});
			});
		}

		if (board && variableGetNode && variableSetNode) {
			Object.values(board.variables).forEach((variable) => {
				const getPins = Object.values(variableGetNode?.pins ?? {}).map((pin) =>
					pin.name === "var_ref"
						? { ...pin, default_value: convertJsonToUint8Array(variable.id) }
						: pin,
				);
				const setPins = Object.values(variableSetNode?.pins ?? {}).map((pin) =>
					pin.name === "var_ref"
						? { ...pin, default_value: convertJsonToUint8Array(variable.id) }
						: pin,
				);
				const newGetPins = Object.fromEntries(
					getPins.map((pin) => [pin.id, pin]),
				);
				const newSetPins = Object.fromEntries(
					setPins.map((pin) => [pin.id, pin]),
				);

				normalNodes.push({
					...(variableGetNode as INode),
					id: variable.id,
					pin_in_names: Object.values(newGetPins)
						.filter((pin) => pin.pin_type === "Input")
						.map((pin) => pin.friendly_name),
					pin_out_names: Object.values(newGetPins)
						.filter((pin) => pin.pin_type === "Output")
						.map((pin) => pin.friendly_name),
					friendly_name: `Get ${variable.name}`,
					category: "Variables/Get",
					pins: newGetPins,
				});

				normalNodes.push({
					...(variableSetNode as INode),
					id: variable.id,
					pin_in_names: Object.values(newSetPins)
						.filter((pin) => pin.pin_type === "Input")
						.map((pin) => pin.friendly_name),
					pin_out_names: Object.values(newSetPins)
						.filter((pin) => pin.pin_type === "Output")
						.map((pin) => pin.friendly_name),
					friendly_name: `Set ${variable.name}`,
					category: "Variables/Set",
					pins: newSetPins,
				});
			});
		}

		return normalNodes;
	}, [nodes, board]);
	const { search, searchResults, addAllAsync, removeAll } =
		useMiniSearch<INode>([], {
			fields: [
				"name",
				"friendly_name",
				"category",
				"description",
				"pin_in_names",
				"pin_out_names",
			],
			storeFields: ["id"],
			searchOptions: {
				prefix: true,
				fuzzy: true,
				boost: {
					name: 3,
					friendly_name: 2,
					category: 1.5,
					description: 0.75,
					pin_in_names: 1,
					pin_out_names: 1,
				},
			},
		});

	useEffect(() => {
		inputRef.current?.focus();
	}, [filter]);

	useEffect(() => {
		removeAll();
		(async () => {
			if (!nodes) return;
			const dedupedNodes = new Map<string, INode>();
			let callRefNode: INode | undefined = undefined;
			let variableGetNode: INode | undefined = undefined;
			let variableSetNode: INode | undefined = undefined;

			nodes.forEach((node) => {
				if (node.name === "control_call_reference") {
					callRefNode = node;
				}
				if (node.name === "variable_get") {
					variableGetNode = node;
				}
				if (node.name === "variable_set") {
					variableSetNode = node;
				}
				dedupedNodes.set(node.name, {
					...node,
					pin_in_names: Object.values(node.pins)
						.filter((pin) => pin.pin_type === "Input")
						.map((pin) => pin.friendly_name),
					pin_out_names: Object.values(node.pins)
						.filter((pin) => pin.pin_type === "Output")
						.map((pin) => pin.friendly_name),
				});
			});

			if (board && callRefNode) {
				Object.values(board.nodes).forEach((node) => {
					if (!node.start) return;
					const pins = Object.values(callRefNode?.pins ?? {}).map((pin) =>
						pin.name === "fn_ref"
							? { ...pin, default_value: convertJsonToUint8Array(node.id) }
							: pin,
					);
					const newPins = Object.fromEntries(pins.map((pin) => [pin.id, pin]));

					dedupedNodes.set(node.id, {
						...(callRefNode as INode),
						id: node.id,
						pin_in_names: Object.values(newPins)
							.filter((pin) => pin.pin_type === "Input")
							.map((pin) => pin.friendly_name),
						pin_out_names: Object.values(newPins)
							.filter((pin) => pin.pin_type === "Output")
							.map((pin) => pin.friendly_name),
						friendly_name: `Call ${node.friendly_name}`,
						category: "Events/Call",
						pins: newPins,
					});
				});
			}

			if (board && variableGetNode && variableSetNode) {
				Object.values(board.variables).forEach((variable) => {
					const getPins = Object.values(variableGetNode?.pins ?? {}).map(
						(pin) =>
							pin.name === "var_ref"
								? {
										...pin,
										default_value: convertJsonToUint8Array(variable.id),
									}
								: pin,
					);
					const setPins = Object.values(variableSetNode?.pins ?? {}).map(
						(pin) =>
							pin.name === "var_ref"
								? {
										...pin,
										default_value: convertJsonToUint8Array(variable.id),
									}
								: pin,
					);
					const newGetPins = Object.fromEntries(
						getPins.map((pin) => [pin.id, pin]),
					);
					const newSetPins = Object.fromEntries(
						setPins.map((pin) => [pin.id, pin]),
					);
					dedupedNodes.set(variable.id, {
						...(variableGetNode as INode),
						id: "get" + variable.id,
						pin_in_names: Object.values(newGetPins)
							.filter((pin) => pin.pin_type === "Input")
							.map((pin) => pin.friendly_name),
						pin_out_names: Object.values(newGetPins)
							.filter((pin) => pin.pin_type === "Output")
							.map((pin) => pin.friendly_name),
						friendly_name: `Get ${variable.name}`,
						category: "Variables/Get",
						pins: newGetPins,
					});
					dedupedNodes.set("set" + variable.id, {
						...(variableSetNode as INode),
						id: variable.id,
						pin_in_names: Object.values(newSetPins)
							.filter((pin) => pin.pin_type === "Input")
							.map((pin) => pin.friendly_name),
						pin_out_names: Object.values(newSetPins)
							.filter((pin) => pin.pin_type === "Output")
							.map((pin) => pin.friendly_name),
						friendly_name: `Set ${variable.name}`,
						category: "Variables/Set",
						pins: newSetPins,
					});
				});
			}

			await addAllAsync(Array.from(dedupedNodes.values()));
		})();
	}, [sortedNodes, board]);

	return (
		<ContextMenu
			onOpenChange={(open) => {
				if (!open) {
					onClose();
					setFilter("");
				}
			}}
		>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContextMenuContent className="w-80 max-h-96 overflow-y-hidden overflow-x-hidden">
				<div className="sticky">
					<div className="flex flex-row w-full items-center justify-between bg-accent text-accent-foreground p-1 mb-1">
						<small className="font-bold">Actions</small>
						{droppedPin && (
							<div className="flex flex-row items-center gap-2">
								<div className="grid gap-1.5 leading-none">
									<small>Context Sensitive</small>
								</div>
								<Checkbox
									id="context-sensitive"
									checked={contextSensitive}
									onCheckedChange={(checked) =>
										setContextSensitive(checked.valueOf() as boolean)
									}
								/>
							</div>
						)}
					</div>
					<ContextMenuItem
						className="flex flex-row gap-1 items-center"
						onClick={() => onCommentPlace()}
					>
						<SquarePlusIcon className="w-4 h-4" />
						Comment
					</ContextMenuItem>
					<ContextMenuItem
						className="flex flex-row gap-1 items-center"
						onClick={() => {
							const node_ref = sortedNodes.find(
								(node) => node.name === "events_simple",
							);
							if (node_ref) onNodePlace(node_ref);
						}}
					>
						<SquarePlusIcon className="w-4 h-4" />
						Event
					</ContextMenuItem>
					<Separator className="my-1" />
					<Input
						ref={inputRef}
						autoComplete="off"
						spellCheck="false"
						autoCorrect="off"
						autoCapitalize="off"
						className="mb-1"
						autoFocus
						type="search"
						placeholder="Search..."
						value={filter}
						onChange={(e) => {
							setFilter(e.target.value);
							search(e.target.value);
						}}
					/>
				</div>
				<div className="pr-1">
					<ScrollArea
						className="h-52 w-[calc(20rem-0.5rem)] border rounded-md"
						onFocusCapture={() => {
							if (inputRef.current && filter !== "") {
								inputRef.current.focus();
							}
						}}
					>
						{nodes && (
							<FlowContextMenuNodes
								items={
									droppedPin && contextSensitive
										? [
												...(filter === ""
													? sortedNodes
													: (searchResults ?? [])
												).filter((node) => {
													const pins = Object.values(node.pins);
													return pins.some((pin) => {
														if (pin.pin_type === droppedPin.pin_type)
															return false;
														return doPinsMatch(pin, droppedPin, refs);
													});
												}),
											]
										: [...(filter === "" ? sortedNodes : (searchResults ?? []))]
								}
								filter={filter}
								onNodePlace={async (node) => onNodePlace(node)}
							/>
						)}
					</ScrollArea>
				</div>
			</ContextMenuContent>
		</ContextMenu>
	);
}
