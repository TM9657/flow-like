"use client";

import { useTranslation } from "@flow-like/locales";
import {
	ChevronDown,
	CircleAlert,
	Cpu,
	Image,
	LoaderCircle,
	RotateCcw,
	Zap,
} from "lucide-react";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { useBoardRenderer } from "../../hooks/use-board-renderer";
import { cn } from "../../lib/utils";
import { Popover, PopoverContent, PopoverTrigger } from "../ui/popover";

export interface BoardRendererStatusProps {
	state: "loading" | "active" | "react-flow" | "fallback" | "exporting";
	error?: string;
	onRetry?: () => void;
}

export function BoardRendererStatus({
	state,
	error,
	onRetry,
}: BoardRendererStatusProps) {
	const { t } = useTranslation("flow");
	const { setRenderer } = useBoardRenderer();
	const [open, setOpen] = useState(false);
	const triggerRef = useRef<HTMLButtonElement>(null);
	const panelRef = useRef<HTMLDivElement>(null);
	const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
	const hoverOpened = useRef(false);
	const titleId = useId();
	const cancelClose = useCallback(() => {
		if (closeTimer.current !== null) clearTimeout(closeTimer.current);
		closeTimer.current = null;
	}, []);
	useEffect(() => cancelClose, [cancelClose]);
	const scheduleClose = () => {
		cancelClose();
		closeTimer.current = setTimeout(() => {
			if (!hoverOpened.current) return;
			const focused = document.activeElement;
			if (
				triggerRef.current?.contains(focused) ||
				panelRef.current?.contains(focused)
			)
				return;
			setOpen(false);
		}, 180);
	};
	const content = {
		active: {
			label: "WASM",
			detail: t("boardRendererWasmActive", "WASM renderer is active."),
			Icon: Zap,
		},
		loading: {
			label: t("boardRendererWasmStarting", "Starting WASM…"),
			detail: t(
				"boardRendererWasmStartingDetail",
				"The canvas renderer is starting.",
			),
			Icon: LoaderCircle,
		},
		"react-flow": {
			label: "React Flow",
			detail: t(
				"boardRendererReactFlowActive",
				"React Flow is drawing this board. You can enable WASM in Settings.",
			),
			Icon: Cpu,
		},
		fallback: {
			label: t("boardRendererFallback", "React Flow · fallback"),
			detail: t(
				"boardRendererFallbackDetail",
				"WASM could not start. React Flow is keeping this board available.",
			),
			Icon: CircleAlert,
		},
		exporting: {
			label: t("boardRendererExporting", "Capturing…"),
			detail: t(
				"boardRendererExportingDetail",
				"Preparing the board for screenshot export.",
			),
			Icon: Image,
		},
	}[state];
	const Icon = content.Icon;
	const retryLabel = t("boardRendererRetryWasm", "Retry WASM");
	const canSwitch =
		state === "active" || state === "loading" || state === "fallback";

	return (
		<Popover
			open={open}
			onOpenChange={(nextOpen) => {
				cancelClose();
				setOpen(nextOpen);
			}}
			modal={false}
		>
			<div data-board-canvas-controls data-board-renderer-status={state}>
				<PopoverTrigger asChild>
					<button
						ref={triggerRef}
						type="button"
						aria-label={`${content.label}. ${content.detail}`}
						onPointerEnter={(event) => {
							cancelClose();
							if (event.pointerType !== "touch" && !open) {
								hoverOpened.current = true;
								setOpen(true);
							}
						}}
						onPointerLeave={scheduleClose}
						onClick={(event) => {
							if (open && hoverOpened.current) event.preventDefault();
							hoverOpened.current = false;
						}}
						onBlur={scheduleClose}
						onKeyDown={(event) => {
							if (event.key === "Tab" && !event.shiftKey && open) {
								const firstAction =
									panelRef.current?.querySelector<HTMLElement>(
										"button:not(:disabled), a[href]",
									);
								if (firstAction) {
									hoverOpened.current = false;
									event.preventDefault();
									firstAction.focus();
								}
							}
						}}
						className={cn(
							"inline-flex items-center gap-1 whitespace-nowrap rounded-full border border-border bg-card/95 px-2 py-1 text-[10px] font-medium leading-none text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
							state === "active" &&
								"border-primary/30 bg-primary/10 text-primary",
						)}
					>
						<Icon aria-hidden="true" className="size-3" />
						<span aria-live="polite">{content.label}</span>
						<ChevronDown aria-hidden="true" className="size-2.5 opacity-60" />
					</button>
				</PopoverTrigger>
			</div>
			<PopoverContent
				ref={panelRef}
				data-board-canvas-controls
				data-board-renderer-panel
				aria-labelledby={titleId}
				side="bottom"
				align="end"
				className="w-80 max-w-[calc(100vw-2rem)] space-y-3"
				onPointerEnter={cancelClose}
				onPointerLeave={scheduleClose}
				onPointerDown={() => {
					hoverOpened.current = false;
				}}
				onBlur={scheduleClose}
				onOpenAutoFocus={(event) => event.preventDefault()}
				onCloseAutoFocus={(event) => event.preventDefault()}
				onEscapeKeyDown={(event) => {
					event.stopPropagation();
					if (
						!hoverOpened.current ||
						panelRef.current?.contains(document.activeElement)
					)
						triggerRef.current?.focus();
				}}
				onKeyDown={(event) => {
					if (event.key !== "Tab") return;
					const actions = panelRef.current?.querySelectorAll<HTMLElement>(
						"button:not(:disabled), a[href]",
					);
					if (!actions?.length) return;
					if (event.shiftKey && document.activeElement === actions[0]) {
						event.preventDefault();
						triggerRef.current?.focus();
					} else if (
						!event.shiftKey &&
						document.activeElement === actions[actions.length - 1]
					) {
						// Resume the page's tab order at the badge instead of looping in the portal.
						triggerRef.current?.focus();
						setOpen(false);
					}
				}}
			>
				<div className="flex items-center gap-2">
					<Icon
						aria-hidden="true"
						className={cn("size-4", state === "active" && "text-primary")}
					/>
					<h3 id={titleId} className="text-sm font-semibold">
						{t("boardRendererTitle", "Workflow renderer")}
					</h3>
					<span className="ml-auto text-xs text-muted-foreground">
						{content.label}
					</span>
				</div>
				<p className="text-sm">{content.detail}</p>
				{canSwitch && (
					<div className="space-y-2 text-xs leading-relaxed text-muted-foreground">
						<p>
							{t(
								"boardRendererWasmExplanation",
								"The WebAssembly canvas draws nodes and connections. Node editors, menus, comments, and media keep their usual controls.",
							)}
						</p>
						<p>
							{t(
								"boardRendererGlowExplanation",
								"The edge glow is a visual effect; execution is shown on the nodes.",
							)}
						</p>
					</div>
				)}
				{state === "fallback" && error && (
					<p className="rounded-md bg-muted/60 p-2 text-xs break-words text-muted-foreground">
						{error}
					</p>
				)}
				<div className="space-y-2 border-t border-border pt-3">
					{state === "fallback" && onRetry && (
						<button
							type="button"
							onClick={() => {
								triggerRef.current?.focus();
								setOpen(false);
								onRetry();
							}}
							className="flex items-center gap-1.5 rounded-sm text-sm font-medium text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						>
							<RotateCcw aria-hidden="true" className="size-3.5" />
							{retryLabel}
						</button>
					)}
					{canSwitch ? (
						<button
							type="button"
							onClick={() => {
								triggerRef.current?.focus();
								setOpen(false);
								setRenderer("react-flow");
							}}
							className="block rounded-sm text-sm font-medium text-primary underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						>
							{t("boardRendererSwitchToReactFlow", "Switch to React Flow")}
						</button>
					) : (
						<a
							href="/settings#board-renderer-card"
							className="block w-fit rounded-sm text-sm font-medium text-primary underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						>
							{t("boardRendererSettings", "Renderer settings")}
						</a>
					)}
					<p className="text-xs leading-relaxed text-muted-foreground">
						{t(
							"boardRendererLocalPreference",
							"Your choice is saved for this browser or desktop installation.",
						)}
					</p>
				</div>
			</PopoverContent>
		</Popover>
	);
}
