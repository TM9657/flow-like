"use client";
import { useStore, type Node } from "@xyflow/react";
import { memo, useMemo } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "../ui/avatar";

interface PeerPresence {
	clientId: number;
	user: { id?: string; name: string; color: string; avatar?: string };
	layerPath: string;
}

interface LayerIndicator {
	nodeId: string;
	screenX: number;
	screenY: number;
	peers: Array<{ name: string; color: string; avatar?: string; userId?: string }>;
}

export const FlowLayerIndicators = memo(function FlowLayerIndicators({
	peers,
	currentLayerPath,
	nodes,
}: {
	peers: PeerPresence[];
	currentLayerPath: string;
	nodes: Node[];
}) {
	const transform = useStore((state) => state.transform);
	const [tx, ty, zoom] = transform;

	const indicators = useMemo(() => {
		// Group peers by their layer (excluding those on current layer)
		const peersByLayer = new Map<string, PeerPresence[]>();

		// Normalize currentLayerPath for comparison (treat undefined/root as empty string)
		const normalizedCurrentPath = !currentLayerPath || currentLayerPath === "root" ? "" : currentLayerPath;

		for (const peer of peers) {
			const normalizedPeerPath = !peer.layerPath || peer.layerPath === "root" ? "" : peer.layerPath;
			if (normalizedPeerPath === normalizedCurrentPath) continue;

			const existing = peersByLayer.get(peer.layerPath) ?? [];
			existing.push(peer);
			peersByLayer.set(peer.layerPath, existing);
		}

		if (peersByLayer.size === 0) return [];

		// Find layer nodes that match or contain the peer layers
		const result: LayerIndicator[] = [];

		for (const node of nodes) {
			if (node.type !== "layer") continue;

			// Check if any peers are in this layer or a sublayer
			const matchingPeers: PeerPresence[] = [];

			// Build the full path for this layer node from the current layer
			// If we're at root, the node path is just the node.id
			// If we're in a layer, the node path is currentPath/node.id
			const nodePath = normalizedCurrentPath
				? `${normalizedCurrentPath}/${node.id}`
				: node.id;

			for (const [peerLayer, layerPeers] of peersByLayer.entries()) {
				// Normalize peer layer path
				const normalizedPeerLayer = !peerLayer || peerLayer === "root" ? "" : peerLayer;

				// Check if peer is in this exact layer
				if (normalizedPeerLayer === nodePath) {
					matchingPeers.push(...layerPeers);
					continue;
				}

				// Check if peer is in a sublayer of this layer node
				// e.g., nodePath = "layer1", peerLayer = "layer1/layer2" or "layer1/layer2/layer3"
				if (normalizedPeerLayer.startsWith(`${nodePath}/`)) {
					matchingPeers.push(...layerPeers);
				}
			}

			if (matchingPeers.length === 0) continue;

			// Calculate screen position for top-right of the node
			const nodeScreenX = (node.position.x + (node.measured?.width ?? 0)) * zoom + tx;
			const nodeScreenY = node.position.y * zoom + ty;

			result.push({
				nodeId: node.id,
				screenX: nodeScreenX,
				screenY: nodeScreenY,
				peers: matchingPeers.map(p => ({
					name: p.user.name,
					color: p.user.color,
					avatar: p.user.avatar,
					userId: p.user.id,
				})),
			});
		}

		return result;
	}, [peers, currentLayerPath, nodes, tx, ty, zoom]);

	return (
		<div className="pointer-events-none absolute inset-0 z-30">
			{indicators.map((indicator) => (
				<div
					key={indicator.nodeId}
					className="absolute flex items-center gap-1"
					style={{
						transform: `translate(${indicator.screenX}px, ${indicator.screenY}px)`,
					}}
				>
					<div className="flex items-center -space-x-2">
						{indicator.peers.slice(0, 3).map((peer, idx) => (
							<Avatar
								key={`${peer.userId}-${idx}`}
								className="h-6 w-6 border-2 shadow-lg"
								style={{ borderColor: peer.color }}
							>
								<AvatarImage src={peer.avatar} alt={peer.name} />
								<AvatarFallback
									className="text-[10px] font-semibold"
									style={{ backgroundColor: peer.color, color: "white" }}
								>
									{peer.name.slice(0, 2).toUpperCase()}
								</AvatarFallback>
							</Avatar>
						))}
						{indicator.peers.length > 3 && (
							<div
								className="flex h-6 w-6 items-center justify-center rounded-full border-2 border-background bg-muted text-[10px] font-semibold shadow-lg"
							>
								+{indicator.peers.length - 3}
							</div>
						)}
					</div>
				</div>
			))}
		</div>
	);
});
