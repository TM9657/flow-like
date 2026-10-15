"use client";
import { NodeToolbar } from "../board-runtime-primitives";

import { useTranslation } from "@flow-like/locales";
import { Position } from "@xyflow/react";
import { ChevronDown, Minus, Plus } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { INode } from "../../../lib/schema/flow/node";
import {
	type IPin,
	IPinType,
	IVariableType,
} from "../../../lib/schema/flow/pin";
import { Popover, PopoverContent, PopoverTrigger } from "../../ui/popover";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../ui/tooltip";

export interface EguiPinControlsProps {
	nodeId: string;
	node: INode;
	visible: boolean;
	onAdd: (pin: IPin, index: number) => Promise<void>;
	onRemove: (pin: IPin) => Promise<void>;
}

export function EguiPinControls({
	nodeId,
	node,
	visible,
	onAdd,
	onRemove,
}: EguiPinControlsProps) {
	const { t } = useTranslation("flow");
	const [openFamily, setOpenFamily] = useState<string | null>(null);
	const [hovered, setHovered] = useState(false);
	const [focused, setFocused] = useState(false);
	const [retainedVisible, setRetainedVisible] = useState(visible);
	const [pending, setPending] = useState(false);
	const pendingRef = useRef(false);
	const focusAfterRemoval = useRef<string | null>(null);
	const stripRef = useRef<HTMLDivElement>(null);
	const managerRef = useRef<HTMLDivElement>(null);
	const families = useMemo(() => {
		const groups = new Map<string, IPin[]>();
		for (const pin of Object.values(node.pins)) {
			const key = JSON.stringify([pin.pin_type, pin.name]);
			const group = groups.get(key);
			if (group) group.push(pin);
			else groups.set(key, [pin]);
		}
		return Array.from(groups, ([key, pins]) => ({
			key,
			pins: pins.sort((a, b) => a.index - b.index || a.id.localeCompare(b.id)),
		})).filter((family) => family.pins.length >= 2);
	}, [node.pins]);

	useEffect(() => {
		if (visible || hovered || focused || openFamily !== null || pending) {
			setRetainedVisible(true);
			return;
		}
		const timer = setTimeout(() => setRetainedVisible(false), 220);
		return () => clearTimeout(timer);
	}, [visible, hovered, focused, openFamily, pending]);
	useEffect(() => {
		const removedId = focusAfterRemoval.current;
		if (pending || !removedId || node.pins[removedId]) return;
		focusAfterRemoval.current = null;
		const manager = managerRef.current;
		const next = manager?.querySelector<HTMLButtonElement>(
			'button[data-pin-action="remove"]:not(:disabled)',
		);
		(next ?? manager)?.focus();
	}, [node.pins, pending]);

	const updateFocus = (next: EventTarget | null) => {
		setFocused(
			next instanceof Node &&
				!!(
					stripRef.current?.contains(next) || managerRef.current?.contains(next)
				),
		);
	};
	const runMutation = async (
		action: () => Promise<void>,
		kind: "add" | "remove",
		restoreAfterRemoval?: string,
	) => {
		if (pendingRef.current) return;
		pendingRef.current = true;
		setPending(true);
		try {
			await action();
			if (restoreAfterRemoval) focusAfterRemoval.current = restoreAfterRemoval;
		} catch (cause) {
			toast.error(
				cause instanceof Error
					? cause.message
					: kind === "add"
						? t("addPinFailed", "Could not add the pin.")
						: t("deletePinFailed", "Could not delete the pin."),
			);
		} finally {
			pendingRef.current = false;
			setPending(false);
		}
	};

	const active =
		visible ||
		retainedVisible ||
		hovered ||
		focused ||
		openFamily !== null ||
		pending;
	if (families.length === 0 || !active) return null;

	return (
		<NodeToolbar
			nodeId={nodeId}
			position={node.error ? Position.Right : Position.Bottom}
			offset={10}
			isVisible
			data-board-node-id={nodeId}
			data-board-dom-control
			data-board-canvas-controls
			className="nodrag nopan nowheel"
		>
			<div
				ref={stripRef}
				data-egui-pin-controls
				data-board-node-id={nodeId}
				aria-busy={pending}
				onPointerEnter={() => setHovered(true)}
				onPointerLeave={() => setHovered(false)}
				onFocusCapture={() => setFocused(true)}
				onBlurCapture={(event) => updateFocus(event.relatedTarget)}
				onPointerDown={(event) => event.stopPropagation()}
				onClick={(event) => event.stopPropagation()}
				onDoubleClick={(event) => event.stopPropagation()}
				onWheel={(event) => event.stopPropagation()}
				onKeyDown={(event) => {
					if (!event.metaKey && !event.ctrlKey) event.stopPropagation();
				}}
				className="flex flex-col items-center gap-1.5"
			>
				{families.map(({ key, pins }) => {
					const first = pins[0];
					const last = pins[pins.length - 1];
					const execution = first.data_type === IVariableType.Execution;
					const direction =
						first.pin_type === IPinType.Input
							? t("eguiPinInput", "Input")
							: t("eguiPinOutput", "Output");
					const label = execution
						? t("eguiPinBranches", "Branches")
						: first.friendly_name.trim() || direction;
					const itemLabel = execution
						? t("eguiPinBranch", "Branch")
						: first.friendly_name.trim() || direction;
					const addLabel = execution
						? t("eguiPinAddBranch", "Add branch")
						: t("eguiPinAddNamed", "Add {{name}}", { name: itemLabel });
					const connectedPins = pins.filter(
						(pin) => pin.connected_to.length > 0 || pin.depends_on.length > 0,
					).length;

					return (
						<Popover
							key={key}
							open={openFamily === key}
							onOpenChange={(open) => setOpenFamily(open ? key : null)}
							modal={false}
						>
							<div className="flex max-w-[min(24rem,calc(100vw-2rem))] items-center rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-md">
								<PopoverTrigger asChild>
									<button
										type="button"
										data-egui-pin-manage
										data-pin-family={key}
										disabled={pending}
										title={t(
											"eguiPinManageNamed",
											"Manage {{name}} ({{direction}})",
											{
												name: label,
												direction,
											},
										)}
										className="flex h-7 min-w-0 items-center gap-1.5 rounded-md px-2 text-xs font-medium hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-wait disabled:opacity-50"
									>
										<span className="truncate" title={label}>
											{label}
										</span>
										<span className="shrink-0 text-muted-foreground">
											· {pins.length}
										</span>
										<ChevronDown
											aria-hidden="true"
											className="size-4 shrink-0"
										/>
									</button>
								</PopoverTrigger>
								<div aria-hidden="true" className="mx-1 h-4 w-px bg-border" />
								<button
									type="button"
									data-pin-action="add"
									data-pin-family={key}
									aria-label={t("addPin", "Add pin")}
									title={`${addLabel} (${direction})`}
									disabled={pending}
									onClick={() =>
										void runMutation(() => onAdd(last, last.index + 1), "add")
									}
									className="flex h-7 min-w-0 items-center gap-1 rounded-md px-2 text-xs font-medium text-primary hover:bg-primary/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-wait disabled:opacity-50"
								>
									<Plus aria-hidden="true" className="size-4 shrink-0" />
									<span className="truncate" title={addLabel}>
										{addLabel}
									</span>
								</button>
							</div>
							<PopoverContent
								ref={managerRef}
								data-egui-pin-manager
								data-board-node-id={nodeId}
								data-board-dom-control
								data-board-canvas-controls
								aria-label={t(
									"eguiPinManageNamed",
									"Manage {{name}} ({{direction}})",
									{
										name: label,
										direction,
									},
								)}
								aria-busy={pending}
								tabIndex={-1}
								side="bottom"
								sideOffset={6}
								onPointerEnter={() => setHovered(true)}
								onPointerLeave={() => setHovered(false)}
								onFocusCapture={() => setFocused(true)}
								onBlurCapture={(event) => updateFocus(event.relatedTarget)}
								onPointerDown={(event) => event.stopPropagation()}
								onClick={(event) => event.stopPropagation()}
								onDoubleClick={(event) => event.stopPropagation()}
								onWheel={(event) => event.stopPropagation()}
								onKeyDown={(event) => {
									if (!event.metaKey && !event.ctrlKey) event.stopPropagation();
								}}
								className="nodrag nopan nowheel w-72 bg-popover p-3 backdrop-blur-none"
							>
								<div className="mb-2 flex items-baseline justify-between gap-3">
									<h3
										className="min-w-0 truncate text-sm font-semibold"
										title={label}
									>
										{label}
									</h3>
									<span className="shrink-0 text-xs text-muted-foreground">
										{t("eguiPinConnectedCount", "{{count}} connected", {
											count: connectedPins,
										})}
									</span>
								</div>
								<div className="max-h-64 space-y-1 overflow-y-auto">
									{pins.map((pin, index) => {
										const rowLabel = `${itemLabel} ${index + 1}`;
										const connections = new Set([
											...pin.depends_on,
											...pin.connected_to,
										]).size;
										const removeTitle = t(
											"eguiPinRemoveNamed",
											"Remove {{name}}",
											{ name: rowLabel },
										);
										return (
											<div
												key={pin.id}
												data-pin-id={pin.id}
												className="flex min-h-8 items-center gap-2 rounded-md bg-muted/40 px-2 py-0.5"
											>
												<div className="min-w-0 flex-1">
													<p
														className="truncate text-xs font-medium"
														title={rowLabel}
													>
														{rowLabel}
													</p>
													<p className="text-[10px] text-muted-foreground">
														{connections > 0
															? t(
																	"eguiPinConnectionCount",
																	"{{count}} connections",
																	{ count: connections },
																)
															: t("eguiPinNotConnected", "Not connected")}
													</p>
												</div>
												{index >= 2 ? (
													<Tooltip>
														<TooltipTrigger asChild>
															<button
																type="button"
																data-pin-action="remove"
																data-pin-id={pin.id}
																aria-label={t("deletePin", "Delete Pin")}
																title={removeTitle}
																disabled={pending}
																onClick={(event) =>
																	void runMutation(
																		() => onRemove(pin),
																		"remove",
																		event.currentTarget ===
																			document.activeElement
																			? pin.id
																			: undefined,
																	)
																}
																className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border hover:border-destructive/40 hover:bg-destructive/10 hover:text-destructive focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-wait disabled:opacity-50"
															>
																<Minus aria-hidden="true" className="size-4" />
															</button>
														</TooltipTrigger>
														<TooltipContent className="backdrop-blur-none">
															{removeTitle}
														</TooltipContent>
													</Tooltip>
												) : (
													<span className="text-[10px] text-muted-foreground">
														{t("eguiPinRequired", "Required")}
													</span>
												)}
											</div>
										);
									})}
								</div>
								<p className="mt-2 text-[11px] leading-relaxed text-muted-foreground">
									{t(
										"eguiPinProtectedExplanation",
										"The first two pins are required. Removing a pin also removes its connections.",
									)}
								</p>
							</PopoverContent>
						</Popover>
					);
				})}
			</div>
		</NodeToolbar>
	);
}
