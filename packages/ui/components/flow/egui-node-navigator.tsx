"use client";

import { useTranslation } from "@flow-like/locales";
import { useId, useMemo, useRef, useState } from "react";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogTitle,
	DialogTrigger,
} from "../ui/dialog";

export interface NavigableBoardNode {
	id: string;
	label: string;
}

/** Keyboard access to graph nodes whose visual representation is native. */
export function EguiNodeNavigator({
	nodes,
	onNavigate,
	kind = "nodes",
}: {
	nodes: readonly NavigableBoardNode[];
	kind?: "nodes" | "connections";
	onNavigate(id: string): void;
}) {
	const { t } = useTranslation("flow");
	const connections = kind === "connections";
	const id = useId();
	const search = useRef<HTMLInputElement>(null);
	const chosen = useRef(false);
	const [open, setOpen] = useState(false);
	const [query, setQuery] = useState("");
	const [active, setActive] = useState(0);
	const matches = useMemo(() => {
		const term = query.trim().toLocaleLowerCase();
		return term
			? nodes.filter((node) =>
					`${node.label} ${node.id}`.toLocaleLowerCase().includes(term),
				)
			: nodes;
	}, [nodes, query]);
	const focused = Math.min(active, Math.max(0, matches.length - 1));
	const start = Math.max(0, Math.min(focused - 10, matches.length - 40));
	const choose = () => {
		const node = matches[focused];
		if (!node) return;
		chosen.current = true;
		setOpen(false);
		onNavigate(node.id);
	};
	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				if (next) {
					setQuery("");
					setActive(0);
					chosen.current = false;
				}
				setOpen(next);
			}}
		>
			<DialogTrigger asChild>
				<button
					type="button"
					className="rounded px-1 hover:bg-muted focus-visible:outline focus-visible:outline-primary"
				>
					{connections
						? t("browseConnections", "Browse connections")
						: t("browseNodes", "Browse nodes")}
				</button>
			</DialogTrigger>
			<DialogContent
				data-board-dom-control
				onOpenAutoFocus={(event) => {
					event.preventDefault();
					search.current?.focus();
				}}
				onCloseAutoFocus={(event) => {
					if (chosen.current) event.preventDefault();
				}}
			>
				<DialogTitle>
					{connections
						? t("browseWorkflowConnections", "Browse workflow connections")
						: t("browseWorkflowNodes", "Browse workflow nodes")}
				</DialogTitle>
				<DialogDescription>
					{t(
						"browseWorkflowItemsHelp",
						"Search by name or id. Use the arrow keys, then Enter to select and focus the item.",
					)}
				</DialogDescription>
				<input
					ref={search}
					role="combobox"
					aria-label={
						connections
							? t("findWorkflowConnection", "Find a workflow connection")
							: t("findWorkflowNode", "Find a workflow node")
					}
					aria-expanded="true"
					aria-controls={`${id}-list`}
					aria-autocomplete="list"
					aria-activedescendant={
						matches[focused] ? `${id}-option-${focused}` : undefined
					}
					value={query}
					onChange={(event) => {
						setQuery(event.target.value);
						setActive(0);
					}}
					placeholder={t("searchNodeNameOrId", "Search by name or id")}
					className="rounded border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary"
					onKeyDown={(event) => {
						if (event.key === "ArrowDown" || event.key === "ArrowUp") {
							event.preventDefault();
							setActive(
								Math.max(
									0,
									Math.min(
										matches.length - 1,
										focused + (event.key === "ArrowDown" ? 1 : -1),
									),
								),
							);
						}
						if (event.key === "Enter") {
							event.preventDefault();
							choose();
						}
					}}
				/>
				<select
					id={`${id}-list`}
					aria-label={
						connections
							? t("workflowConnections", "Workflow connections")
							: t("workflowNodes", "Workflow nodes")
					}
					size={8}
					value={matches[focused]?.id ?? ""}
					className="min-h-48 rounded border border-border bg-background p-1 text-sm"
					onChange={(event) =>
						setActive(
							matches.findIndex((node) => node.id === event.target.value),
						)
					}
					onDoubleClick={choose}
					onKeyDown={(event) => {
						if (event.key === "Enter") {
							event.preventDefault();
							choose();
						}
						if (event.key === "ArrowDown" || event.key === "ArrowUp") {
							event.preventDefault();
							setActive(
								Math.max(
									0,
									Math.min(
										matches.length - 1,
										focused + (event.key === "ArrowDown" ? 1 : -1),
									),
								),
							);
						}
						if (event.key === "Home" || event.key === "End") {
							event.preventDefault();
							setActive(event.key === "Home" ? 0 : matches.length - 1);
						}
					}}
				>
					{matches.slice(start, start + 40).map((node, index) => (
						<option
							key={node.id}
							id={`${id}-option-${start + index}`}
							value={node.id}
						>
							{node.label} ({node.id})
						</option>
					))}
				</select>
				<div className="flex items-center justify-between gap-3">
					<output className="text-xs text-muted-foreground">
						{t("workflowItemsCount", "{{count}} results", {
							count: matches.length,
						})}
					</output>
					<button
						type="button"
						disabled={!matches.length}
						onClick={choose}
						className="rounded bg-primary px-3 py-2 text-primary-foreground disabled:opacity-50"
					>
						{connections
							? t("goToConnection", "Go to connection")
							: t("goToNode", "Go to node")}
					</button>
				</div>
			</DialogContent>
		</Dialog>
	);
}
