import type { UseQueryResult } from "@tanstack/react-query";
import type { ReactFlowInstance } from "@xyflow/react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { RemoteSelectionParticipant } from "../components/flow/flow-node";
import { createRealtimeSession } from "../lib";
import { normalizeSelectionNodes } from "../lib/flow-board-utils";
import type { IBoard } from "../lib/schema/flow/board";

interface PeerPresence {
	clientId: number;
	cursor?: { x: number; y: number };
	user: { id?: string; name: string; color: string; avatar?: string };
	layerPath: string;
	selection: { nodes: string[] };
}

interface UseRealtimeCollaborationProps {
	appId: string;
	boardId: string;
	board: UseQueryResult<IBoard>;
	version: [number, number, number] | undefined;
	backend: any;
	currentProfile: any;
	hub: any;
	mousePosition: { x: number; y: number };
	layerPath: string | undefined;
	screenToFlowPosition: ReactFlowInstance["screenToFlowPosition"];
	commandAwarenessRef: React.MutableRefObject<any>;
	setNodes: any;
}

export function useRealtimeCollaboration({
	appId,
	boardId,
	board,
	version,
	backend,
	currentProfile,
	hub,
	mousePosition,
	layerPath,
	screenToFlowPosition,
	commandAwarenessRef,
	setNodes,
}: UseRealtimeCollaborationProps) {
	const [awareness, setAwareness] = useState<any | undefined>(undefined);
	const awarenessRef = useRef<any | undefined>(undefined);
	const [connectionStatus, setConnectionStatus] = useState<
		"connected" | "disconnected" | "reconnecting"
	>("disconnected");
	const sessionRef = useRef<{
		dispose: () => void;
		reconnect: () => Promise<void>;
	} | null>(null);
	const [peerStates, setPeerStates] = useState<PeerPresence[]>([]);
	const remoteSelectionsRef = useRef<Map<string, RemoteSelectionParticipant[]>>(
		new Map(),
	);

	const hasBoardData = !!board.data;

	// Setup realtime session
	useEffect(() => {
		let disposed = false;
		const setup = async () => {
			try {
				const offline = await backend.isOffline(appId);
				console.log(
					"[FlowBoard] Offline status:",
					offline,
					"Version:",
					version,
					"Board data:",
					hasBoardData,
				);

				if (!hasBoardData || typeof version !== "undefined") return;
				if (offline) return;

				const room = `${appId}:${boardId}`;
				const [access, jwks] = await Promise.all([
					backend.boardState.getRealtimeAccess(appId, boardId),
					backend.boardState
						.getRealtimeJwks(appId, boardId)
						.catch(() => undefined as any),
				]);
				const name =
					currentProfile.data?.name ||
					currentProfile.data?.settings?.display_name ||
					"Anonymous";
				const userId = currentProfile.data?.id;

				const session = await createRealtimeSession({
					room,
					access,
					jwks,
					name,
					userId,
					signalingServers: hub.hub?.signaling ?? [],
					onStatusChange: (status) => {
						console.log(`[FlowBoard] Connection status changed: ${status}`);
						setConnectionStatus(status);
					},
				});

				if (disposed) {
					session.dispose();
					return;
				}

				sessionRef.current = {
					dispose: session.dispose,
					reconnect: session.reconnect,
				};
				awarenessRef.current = session.awareness;
				commandAwarenessRef.current = session.awareness;
				setAwareness(session.awareness);
				setConnectionStatus("connected");
			} catch (e) {
				console.warn("Realtime setup failed:", e);
				setConnectionStatus("disconnected");
			}
		};
		void setup();

		return () => {
			disposed = true;
			try {
				sessionRef.current?.dispose();
			} catch {}
			sessionRef.current = null;
			awarenessRef.current = undefined;
			commandAwarenessRef.current = undefined;
			setAwareness(undefined);
			setConnectionStatus("disconnected");
		};
	}, [
		backend,
		appId,
		boardId,
		hasBoardData,
		version,
		currentProfile.data?.id,
		currentProfile.data?.name,
		hub.hub,
		commandAwarenessRef,
	]);

	// Update peer states
	useEffect(() => {
		if (!awareness) {
			setPeerStates([]);
			return;
		}

		const updatePeers = () => {
			const states = awareness.getStates() as Map<number, any>;
			const invalidPeers: Set<number> | undefined = (awareness as any)
				?.__invalidPeers;
			const next: PeerPresence[] = [];
			states.forEach((state, clientId) => {
				const isSelf = clientId === awareness.clientID;
				const isInvalid = invalidPeers?.has(clientId) ?? false;
				if (isSelf || isInvalid) return;
				const user = state?.user ?? {};
				const cursor = state?.cursor;
				next.push({
					clientId,
					cursor: cursor ? { x: cursor.x, y: cursor.y } : undefined,
					user: {
						id: user?.id,
						name: user?.name ?? "User",
						color: user?.color ?? "#22c55e",
						avatar: user?.avatar,
					},
					layerPath: state?.layerPath ?? "root",
					selection: {
						nodes: normalizeSelectionNodes(state?.selection?.nodes),
					},
				});
			});
			setPeerStates(next);
		};

		const handleChange = () => updatePeers();
		awareness.on("change", handleChange);
		updatePeers();

		return () => {
			try {
				awareness.off("change", handleChange);
			} catch {}
		};
	}, [awareness]);

	// Listen for peer board updates
	useEffect(() => {
		if (!awareness) return;

		const handleBoardUpdate = ({
			added,
			updated,
		}: { added: number[]; updated: number[] }) => {
			const states = awareness.getStates() as Map<number, any>;
			const changedPeers = [...added, ...updated];

			for (const clientId of changedPeers) {
				if (clientId === awareness.clientID) continue;
				const state = states.get(clientId);
				if (state?.boardUpdate) {
					void board.refetch();
					break;
				}
			}
		};

		awareness.on("update", handleBoardUpdate);
		return () => {
			try {
				awareness.off("update", handleBoardUpdate);
			} catch {}
		};
	}, [awareness, board]);

	// Broadcast cursor position
	useEffect(() => {
		if (!awareness) return;
		const flowPoint = screenToFlowPosition({
			x: mousePosition.x,
			y: mousePosition.y,
		});
		awareness.setLocalStateField("cursor", {
			x: flowPoint.x,
			y: flowPoint.y,
		});
	}, [mousePosition.x, mousePosition.y, awareness, screenToFlowPosition]);

	// Broadcast layer path
	useEffect(() => {
		if (!awareness) return;
		awareness.setLocalStateField("layerPath", layerPath ?? "root");
	}, [awareness, layerPath]);

	// Initialize selection state
	useEffect(() => {
		if (!awareness) return;
		awareness.setLocalStateField("selection", { nodes: [] });
	}, [awareness]);

	// Update user info
	useEffect(() => {
		if (!awareness) return;
		const profileName =
			currentProfile.data?.name ?? currentProfile.data?.settings?.display_name;
		if (!profileName && !currentProfile.data?.id) return;
		const localUser = awareness.getLocalState()?.user ?? {};
		awareness.setLocalStateField("user", {
			...localUser,
			name: profileName ?? localUser.name ?? "Anonymous",
			id: currentProfile.data?.id ?? localUser.id,
		});
	}, [
		awareness,
		currentProfile.data?.id,
		currentProfile.data?.name,
		currentProfile.data?.settings?.display_name,
	]);

	// Update remote selections on nodes
	useEffect(() => {
		const map = new Map<string, RemoteSelectionParticipant[]>();
		for (const peer of peerStates) {
			if (!peer.selection.nodes.length) continue;
			for (const nodeId of peer.selection.nodes) {
				if (!nodeId) continue;
				const participant: RemoteSelectionParticipant = {
					clientId: peer.clientId,
					userId: peer.user.id,
					name: peer.user.name,
					color: peer.user.color,
				};
				const existing = map.get(nodeId) ?? [];
				map.set(nodeId, [...existing, participant]);
			}
		}

		map.forEach((participants, key) => {
			map.set(
				key,
				participants
					.slice()
					.sort((a, b) =>
						a.clientId === b.clientId
							? a.name.localeCompare(b.name)
							: a.clientId - b.clientId,
					),
			);
		});

		// Check if selections actually changed
		let hasChanges = false;
		if (map.size !== remoteSelectionsRef.current.size) {
			hasChanges = true;
		} else {
			for (const [nodeId, participants] of map.entries()) {
				const prev = remoteSelectionsRef.current.get(nodeId);
				if (!prev || prev.length !== participants.length) {
					hasChanges = true;
					break;
				}
				for (let i = 0; i < participants.length; i++) {
					const p = participants[i];
					const prevP = prev[i];
					if (
						!prevP ||
						p.clientId !== prevP.clientId ||
						p.userId !== prevP.userId ||
						p.name !== prevP.name ||
						p.color !== prevP.color
					) {
						hasChanges = true;
						break;
					}
				}
				if (hasChanges) break;
			}
		}

		if (!hasChanges) return;

		remoteSelectionsRef.current = map;

		setNodes((nds: any) => {
			if (nds.length === 0) return nds;
			const updated = nds.map((node: any) => {
				if (node.type !== "node") return node;
				const participants = map.get(node.id) ?? [];
				const hasSelections = participants.length > 0;
				const hadSelections =
					!!node.data.remoteSelections && node.data.remoteSelections.length > 0;

				if (!hasSelections && !hadSelections) return node;

				return {
					...node,
					data: {
						...node.data,
						remoteSelections: hasSelections ? participants : undefined,
					},
				};
			});
			return updated;
		});
	}, [peerStates, setNodes]);

	const reconnect = useCallback(() => {
		sessionRef.current?.reconnect();
	}, []);

	return {
		awareness,
		connectionStatus,
		peerStates,
		reconnect,
	};
}
