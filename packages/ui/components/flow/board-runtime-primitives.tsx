"use client";

import {
	type NodeChange,
	Position,
	Handle as ReactFlowHandle,
	NodeResizer as ReactFlowNodeResizer,
	NodeToolbar as ReactFlowNodeToolbar,
	Panel as ReactFlowPanel,
	ViewportPortal as ReactFlowViewportPortal,
	type ResizeDragEvent,
	type ResizeParams,
} from "@xyflow/react";
import {
	type CSSProperties,
	type ComponentProps,
	type ReactNode,
	type PointerEvent as ReactPointerEvent,
	createContext,
	forwardRef,
	useContext,
	useEffect,
	useMemo,
	useRef,
} from "react";
import { createPortal } from "react-dom";
import {
	useBoardRuntime,
	useBoardRuntimeMode,
	useStore,
} from "./board-runtime";

const BoardNodeIdContext = createContext<string | undefined>(undefined);

export function BoardNodeIdProvider({
	nodeId,
	children,
}: { nodeId: string; children: ReactNode }) {
	return (
		<BoardNodeIdContext.Provider value={nodeId}>
			{children}
		</BoardNodeIdContext.Provider>
	);
}

export const Handle = forwardRef<
	HTMLDivElement,
	ComponentProps<typeof ReactFlowHandle>
>(function BoardHandle(props, ref) {
	const mode = useBoardRuntimeMode();
	return mode === "egui" ? (
		<NativeHandle {...props} ref={ref} />
	) : (
		<ReactFlowHandle {...props} ref={ref} />
	);
});

const NativeHandle = forwardRef<
	HTMLDivElement,
	ComponentProps<typeof ReactFlowHandle>
>(function NativeHandle(
	{
		type = "source",
		position = Position.Top,
		isConnectable = true,
		isConnectableStart = true,
		isConnectableEnd = true,
		isValidConnection: _isValidConnection,
		onConnect: _onConnect,
		id,
		className,
		style,
		...props
	},
	ref,
) {
	const nodeId = useContext(BoardNodeIdContext);
	return (
		<div
			{...props}
			ref={ref}
			data-nodeid={nodeId}
			data-handleid={id}
			data-handlepos={position}
			data-board-pin-type={type}
			style={{ pointerEvents: "all", ...style }}
			className={[
				"react-flow__handle nodrag nopan",
				`react-flow__handle-${position}`,
				type,
				isConnectable && "connectable",
				isConnectable && isConnectableStart && "connectablestart",
				isConnectable && isConnectableEnd && "connectableend",
				className,
			]
				.filter(Boolean)
				.join(" ")}
		/>
	);
});

export function ViewportPortal({ children }: { children: ReactNode }) {
	return useBoardRuntimeMode() === "egui" ? (
		<NativeViewportPortal>{children}</NativeViewportPortal>
	) : (
		<ReactFlowViewportPortal>{children}</ReactFlowViewportPortal>
	);
}

function NativeViewportPortal({ children }: { children: ReactNode }) {
	const element = useStore((state) => state.viewportElement);
	return element ? createPortal(children, element) : null;
}

export function Panel(props: ComponentProps<typeof ReactFlowPanel>) {
	return useBoardRuntimeMode() === "egui" ? (
		<NativePanel {...props} />
	) : (
		<ReactFlowPanel {...props} />
	);
}

function NativePanel({
	position = "top-left",
	style,
	className,
	...props
}: ComponentProps<typeof ReactFlowPanel>) {
	const element = useStore((state) => state.overlayElement);
	const [vertical, horizontal] = position.split("-");
	const panel = (
		<div
			{...props}
			className={`react-flow__panel ${position} ${className ?? ""}`}
			style={{
				position: "absolute",
				zIndex: 50,
				[vertical]: 0,
				...(horizontal === "center"
					? { left: "50%", transform: "translateX(-50%)" }
					: { [horizontal]: 0 }),
				...style,
			}}
		/>
	);
	return element ? createPortal(panel, element) : null;
}

export function NodeToolbar(
	props: ComponentProps<typeof ReactFlowNodeToolbar>,
) {
	return useBoardRuntimeMode() === "egui" ? (
		<NativeNodeToolbar {...props} />
	) : (
		<ReactFlowNodeToolbar {...props} />
	);
}

function NativeNodeToolbar({
	nodeId,
	children,
	isVisible,
	position = Position.Top,
	offset = 10,
	align = "center",
	className,
	style,
	...props
}: ComponentProps<typeof ReactFlowNodeToolbar>) {
	const contextId = useContext(BoardNodeIdContext);
	const ids = useMemo(
		() => (Array.isArray(nodeId) ? nodeId : [nodeId ?? contextId ?? ""]),
		[nodeId, contextId],
	);
	const nodes = useStore(
		(state) =>
			ids.flatMap((id) => {
				const node = state.nodeLookup.get(id);
				return node ? [node] : [];
			}),
		(a, b) =>
			a.length === b.length && a.every((node, index) => node === b[index]),
	);
	const transform = useStore((state) => state.transform);
	const element = useStore((state) => state.overlayElement);
	const selectedCount = useStore((state) =>
		isVisible === undefined
			? state.nodes.filter((node) => node.selected).length
			: 0,
	);
	const visible =
		isVisible ??
		(nodes.length === 1 && nodes[0].selected && selectedCount === 1);
	if (!visible || !element || !nodes.length) return null;
	const left = Math.min(
		...nodes.map((node) => node.internals.positionAbsolute.x),
	);
	const top = Math.min(
		...nodes.map((node) => node.internals.positionAbsolute.y),
	);
	const right = Math.max(
		...nodes.map(
			(node) =>
				node.internals.positionAbsolute.x +
				(node.measured.width ?? node.width ?? 0),
		),
	);
	const bottom = Math.max(
		...nodes.map(
			(node) =>
				node.internals.positionAbsolute.y +
				(node.measured.height ?? node.height ?? 0),
		),
	);
	const [tx, ty, zoom] = transform;
	const factor = align === "start" ? 0 : align === "end" ? 1 : 0.5;
	const horizontal = position === Position.Top || position === Position.Bottom;
	const x =
		tx +
		(horizontal
			? left + (right - left) * factor
			: position === Position.Left
				? left
				: right) *
			zoom;
	const y =
		ty +
		(horizontal
			? position === Position.Top
				? top
				: bottom
			: top + (bottom - top) * factor) *
			zoom;
	const translateX = horizontal
		? -factor * 100
		: position === Position.Left
			? -100
			: 0;
	const translateY = horizontal
		? position === Position.Top
			? -100
			: 0
		: -factor * 100;
	return createPortal(
		<div
			{...props}
			data-id={ids.join(" ")}
			className={`react-flow__node-toolbar ${className ?? ""}`}
			style={{
				position: "absolute",
				left:
					x +
					(position === Position.Left
						? -offset
						: position === Position.Right
							? offset
							: 0),
				top:
					y +
					(position === Position.Top
						? -offset
						: position === Position.Bottom
							? offset
							: 0),
				transform: `translate(${translateX}%, ${translateY}%)`,
				zIndex: Math.max(...nodes.map((node) => node.internals.z + 1)),
				pointerEvents: "auto",
				...style,
			}}
		>
			{children}
		</div>,
		element,
	);
}

type ResizerProps = ComponentProps<typeof ReactFlowNodeResizer>;
type Direction = readonly [number, number];

export function resizeBoardNode(
	start: ResizeParams,
	direction: Direction,
	delta: { x: number; y: number },
	options: Pick<
		ResizerProps,
		"minWidth" | "minHeight" | "maxWidth" | "maxHeight" | "keepAspectRatio"
	> & { snapGrid?: readonly [number, number] },
): ResizeParams {
	const [sx, sy] = direction;
	let dx = delta.x;
	let dy = delta.y;
	if (options.snapGrid) {
		const [gx, gy] = options.snapGrid;
		if (gx > 0)
			dx =
				Math.round((start.x + (sx > 0 ? start.width : 0) + dx) / gx) * gx -
				start.x -
				(sx > 0 ? start.width : 0);
		if (gy > 0)
			dy =
				Math.round((start.y + (sy > 0 ? start.height : 0) + dy) / gy) * gy -
				start.y -
				(sy > 0 ? start.height : 0);
	}
	let width = start.width + sx * dx;
	let height = start.height + sy * dy;
	const minWidth = options.minWidth ?? 10;
	const minHeight = options.minHeight ?? 10;
	const maxWidth = options.maxWidth ?? Number.MAX_VALUE;
	const maxHeight = options.maxHeight ?? Number.MAX_VALUE;
	if (options.keepAspectRatio && start.width > 0 && start.height > 0) {
		const ratio = start.width / start.height;
		if (
			sx &&
			(!sy ||
				Math.abs(width - start.width) >=
					Math.abs(height - start.height) * ratio)
		)
			height = width / ratio;
		else width = height * ratio;
		width = Math.min(
			Math.min(maxWidth, maxHeight * ratio),
			Math.max(Math.max(minWidth, minHeight * ratio), width),
		);
		height = width / ratio;
	} else {
		width = Math.min(maxWidth, Math.max(minWidth, width));
		height = Math.min(maxHeight, Math.max(minHeight, height));
	}
	return {
		x: sx < 0 ? start.x + start.width - width : start.x,
		y: sy < 0 ? start.y + start.height - height : start.y,
		width,
		height,
	};
}

export function NodeResizer(props: ResizerProps) {
	return useBoardRuntimeMode() === "egui" ? (
		<NativeNodeResizer {...props} />
	) : (
		<ReactFlowNodeResizer {...props} />
	);
}

function NativeNodeResizer(props: ResizerProps) {
	const runtime = useBoardRuntime();
	const contextId = useContext(BoardNodeIdContext);
	const id = props.nodeId ?? contextId;
	const zoom = useStore((state) => state.transform[2]);
	const current = useRef(props);
	current.current = props;
	const active = useRef<(() => void) | undefined>(undefined);
	useEffect(() => () => active.current?.(), []);
	useEffect(() => {
		if (props.isVisible === false) active.current?.();
	}, [props.isVisible]);
	if (!runtime || !id || props.isVisible === false) return null;
	const begin = (
		event: ReactPointerEvent<HTMLDivElement>,
		direction: Direction,
	) => {
		if (event.button !== 0 || active.current) return;
		const state = runtime.store.getState();
		const node = state.nodeLookup.get(id);
		if (!node || node.draggable === false || !state.nodesDraggable) return;
		event.preventDefault();
		event.stopPropagation();
		const target = event.currentTarget;
		const pointerId = event.pointerId;
		const start = {
			x: node.position.x,
			y: node.position.y,
			width: node.measured.width ?? node.width ?? 0,
			height: node.measured.height ?? node.height ?? 0,
		};
		const origin = runtime.flow.screenToFlowPosition(
			{ x: event.clientX, y: event.clientY },
			{ snapToGrid: false },
		);
		let latest = start;
		let closed = false;
		const changes = (value: ResizeParams, resizing: boolean): NodeChange[] => [
			{ id, type: "position", position: { x: value.x, y: value.y } },
			{
				id,
				type: "dimensions",
				dimensions: { width: value.width, height: value.height },
				setAttributes: true,
				resizing,
			},
		];
		const dragEvent = (
			sourceEvent: PointerEvent,
			type: "start" | "drag" | "end",
		) =>
			({
				sourceEvent,
				type,
				x: latest.x,
				y: latest.y,
				dx: latest.x - start.x,
				dy: latest.y - start.y,
				subject: { x: start.x, y: start.y },
				identifier: pointerId,
				active: type === "end" ? 0 : 1,
			}) as ResizeDragEvent;
		const update = (sourceEvent: PointerEvent) => {
			if (closed) return;
			const point = runtime.flow.screenToFlowPosition(
				{ x: sourceEvent.clientX, y: sourceEvent.clientY },
				{ snapToGrid: false },
			);
			const state = runtime.store.getState();
			const next = resizeBoardNode(
				start,
				direction,
				{ x: point.x - origin.x, y: point.y - origin.y },
				{
					...current.current,
					snapGrid: state.snapToGrid ? state.snapGrid : undefined,
				},
			);
			const params = { ...next, direction: [...direction] };
			if (
				current.current.shouldResize?.(
					dragEvent(sourceEvent, "drag"),
					params,
				) === false
			)
				return;
			latest = next;
			state.triggerNodeChanges(changes(next, true));
			current.current.onResize?.(dragEvent(sourceEvent, "drag"), params);
		};
		const finish = (commit: boolean, sourceEvent?: PointerEvent) => {
			if (closed) return;
			if (commit && sourceEvent) update(sourceEvent);
			if (closed) return;
			closed = true;
			active.current = undefined;
			unsubscribe();
			target.removeEventListener("pointermove", move);
			target.removeEventListener("pointerup", up);
			target.removeEventListener("pointercancel", cancel);
			target.removeEventListener("lostpointercapture", cancel);
			window.removeEventListener("blur", cancel);
			window.removeEventListener("keydown", key, true);
			if (target.hasPointerCapture(pointerId))
				target.releasePointerCapture(pointerId);
			const live = runtime.store.getState().nodeLookup.get(id);
			if (live?.data !== node.data) return;
			runtime.store
				.getState()
				.triggerNodeChanges(changes(commit ? latest : start, false));
			if (commit && sourceEvent)
				current.current.onResizeEnd?.(dragEvent(sourceEvent, "end"), latest);
		};
		const move = (event: PointerEvent) => {
			if (event.pointerId === pointerId) {
				event.preventDefault();
				update(event);
			}
		};
		const up = (event: PointerEvent) => {
			if (event.pointerId === pointerId) finish(true, event);
		};
		const cancel = () => finish(false);
		const key = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				event.preventDefault();
				event.stopPropagation();
				cancel();
			}
		};
		const unsubscribe = runtime.store.subscribe((state) => {
			const live = state.nodeLookup.get(id);
			if (
				!live ||
				live.data !== node.data ||
				live.draggable === false ||
				!state.nodesDraggable
			)
				cancel();
		});
		active.current = cancel;
		target.setPointerCapture(pointerId);
		target.addEventListener("pointermove", move);
		target.addEventListener("pointerup", up);
		target.addEventListener("pointercancel", cancel);
		target.addEventListener("lostpointercapture", cancel);
		window.addEventListener("blur", cancel);
		window.addEventListener("keydown", key, true);
		current.current.onResizeStart?.(
			dragEvent(event.nativeEvent, "start"),
			start,
		);
	};
	const controls: { position: string; direction: Direction; line: boolean }[] =
		[
			{ position: "left", direction: [-1, 0], line: true },
			{ position: "right", direction: [1, 0], line: true },
			{ position: "top", direction: [0, -1], line: true },
			{ position: "bottom", direction: [0, 1], line: true },
			{ position: "top-left", direction: [-1, -1], line: false },
			{ position: "top-right", direction: [1, -1], line: false },
			{ position: "bottom-left", direction: [-1, 1], line: false },
			{ position: "bottom-right", direction: [1, 1], line: false },
		];
	return (
		<>
			{controls.map(({ position, direction, line }) => (
				<div
					key={position}
					data-board-dom-control
					className={`react-flow__resize-control nodrag nopan ${position.replaceAll("-", " ")} ${line ? "line" : "handle"} ${(line ? props.lineClassName : props.handleClassName) ?? ""}`}
					style={
						{
							...(line ? props.lineStyle : props.handleStyle),
							...(!line && props.autoScale !== false
								? { scale: Math.max(1 / zoom, 1) }
								: {}),
							...(props.color
								? { [line ? "borderColor" : "backgroundColor"]: props.color }
								: {}),
							touchAction: "none",
						} as CSSProperties
					}
					onPointerDown={(event) => begin(event, direction)}
				/>
			))}
		</>
	);
}
