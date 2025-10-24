"use client";
import { useStore } from "@xyflow/react";
import { ArrowRight } from "lucide-react";
import { memo, useMemo } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "../ui/avatar";

interface CursorPeer {
	clientId: number;
	cursor?: { x: number; y: number };
	user: { id?: string; name: string; color: string; avatar?: string };
	layerPath: string;
}

interface CursorDisplay {
	key: string;
	name: string;
	color: string;
	avatar?: string;
	x: number;
	y: number;
	offScreen: boolean;
	edgeX?: number;
	edgeY?: number;
	angle?: number;
}

function calculateEdgePosition(
	screenX: number,
	screenY: number,
	viewportWidth: number,
	viewportHeight: number,
	margin: number
): { edgeX: number; edgeY: number; angle: number } {
	const centerX = viewportWidth / 2;
	const centerY = viewportHeight / 2;
	const dx = screenX - centerX;
	const dy = screenY - centerY;
	const angle = Math.atan2(dy, dx);

	const edgeMargin = 20;
	const clampedY = Math.max(edgeMargin, Math.min(viewportHeight - edgeMargin, screenY));
	const clampedX = Math.max(edgeMargin, Math.min(viewportWidth - edgeMargin, screenX));

	let edgeX: number;
	let edgeY: number;

	const isLeftEdge = screenX < margin;
	const isRightEdge = screenX > viewportWidth - margin;
	const isTopEdge = screenY < margin;
	const isBottomEdge = screenY > viewportHeight - margin;

	if (isLeftEdge) {
		edgeX = edgeMargin;
		edgeY = clampedY;
	} else if (isRightEdge) {
		edgeX = viewportWidth - edgeMargin;
		edgeY = clampedY;
	} else if (isTopEdge) {
		edgeX = clampedX;
		edgeY = edgeMargin;
	} else if (isBottomEdge) {
		edgeX = clampedX;
		edgeY = viewportHeight - edgeMargin;
	} else {
		// Shouldn't reach here, but default to bottom edge
		edgeX = clampedX;
		edgeY = viewportHeight - edgeMargin;
	}

	return { edgeX, edgeY, angle };
}

export const FlowCursors = memo(function FlowCursors({
	peers,
	currentLayerPath,
	className,
}: {
	peers: CursorPeer[];
	currentLayerPath: string;
	className?: string;
}) {
	const transform = useStore((state) => state.transform);
	const [tx, ty, zoom] = transform;

	const cursors = useMemo(
		() => {
			if (typeof window === "undefined") return [];

			const viewportWidth = window.innerWidth;
			const viewportHeight = window.innerHeight;
			const margin = 80;

			return peers
				.filter((peer) => peer.cursor && peer.layerPath === currentLayerPath)
				.map((peer) => {
					const cursor = peer.cursor!;
					const screenX = cursor.x * zoom + tx;
					const screenY = cursor.y * zoom + ty;

					const offScreen =
						screenX < margin ||
						screenX > viewportWidth - margin ||
						screenY < margin ||
						screenY > viewportHeight - margin;

					const edge = offScreen
						? calculateEdgePosition(screenX, screenY, viewportWidth, viewportHeight, margin)
						: undefined;

					return {
						key: `${peer.user.id ?? "user"}-${peer.clientId}`,
						name: peer.user.name,
						color: peer.user.color,
						avatar: peer.user.avatar,
						x: screenX,
						y: screenY,
						offScreen,
						edgeX: edge?.edgeX,
						edgeY: edge?.edgeY,
						angle: edge?.angle,
					} satisfies CursorDisplay;
				});
		},
		[peers, currentLayerPath, tx, ty, zoom],
	);

	return (
		<div className={"pointer-events-none absolute inset-0 z-40 " + (className ?? "")}>
			{cursors.map((cursor) =>
				cursor.offScreen ? (
					<EdgeIndicator key={cursor.key} cursor={cursor} />
				) : (
					<RemoteCursor key={cursor.key} cursor={cursor} />
				)
			)}
		</div>
	);
});

const EdgeIndicator = memo(function EdgeIndicator({ cursor }: { cursor: {
	key: string;
	name: string;
	color: string;
	avatar?: string;
	edgeX?: number;
	edgeY?: number;
	angle?: number;
}; }) {
	if (!cursor.edgeX || !cursor.edgeY) return null;

	return (
		<div
			className="absolute"
			style={{ transform: `translate(${cursor.edgeX}px, ${cursor.edgeY}px)` }}
		>
			<div className="flex items-center gap-1.5 rounded-full border bg-card/95 px-2 py-1.5 shadow-lg backdrop-blur-sm animate-pulse" style={{ borderColor: cursor.color }}>
				<Avatar className="h-4 w-4 border" style={{ borderColor: cursor.color }}>
					<AvatarImage src={cursor.avatar} alt={cursor.name} />
					<AvatarFallback className="text-[8px] font-semibold" style={{ backgroundColor: cursor.color, color: "white" }}>
						{cursor.name.slice(0, 2).toUpperCase()}
					</AvatarFallback>
				</Avatar>
				<ArrowRight
					className="h-3 w-3"
					style={{
						color: cursor.color,
						transform: cursor.angle !== undefined ? `rotate(${cursor.angle}rad)` : undefined
					}}
				/>
			</div>
		</div>
	);
});

const RemoteCursor = memo(function RemoteCursor({ cursor }: { cursor: {
	key: string;
	name: string;
	color: string;
	avatar?: string;
	x: number;
	y: number;
}; }) {
	return (
		<div className="absolute" style={{ transform: `translate(${cursor.x}px, ${cursor.y}px)` }}>
			<div className="flex items-start gap-1.5 select-none">
				<CursorPointer color={cursor.color} />
				<div className="flex items-center gap-1.5 rounded-full border bg-card/95 px-2 py-1 shadow-lg backdrop-blur-sm" style={{ borderColor: cursor.color }}>
					<Avatar className="h-4 w-4 border" style={{ borderColor: cursor.color }}>
						<AvatarImage src={cursor.avatar} alt={cursor.name} />
						<AvatarFallback className="text-[8px] font-semibold" style={{ backgroundColor: cursor.color, color: "white" }}>
							{cursor.name.slice(0, 2).toUpperCase()}
						</AvatarFallback>
					</Avatar>
					<span className="text-[10px] font-medium leading-none">{cursor.name}</span>
				</div>
			</div>
		</div>
	);
});

const CursorPointer = memo(function CursorPointer({ color }: { color: string }) {
	return (
		<svg width="20" height="20" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg" className="drop-shadow-md">
			<path
				d="M3.5 2.5L16.5 10L9.5 11.5L6 18L3.5 2.5Z"
				fill={color}
				stroke="white"
				strokeWidth="1.5"
				strokeLinejoin="round"
			/>
		</svg>
	);
});
