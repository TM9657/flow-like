import {
	type Edge,
	Handle,
	type Node,
	type NodeProps,
	Position,
	ReactFlow,
	type ReactFlowInstance,
	reconnectEdge,
} from "@xyflow/react";
import { WorkflowIcon } from "lucide-react";
import { memo } from "react";
import { createRoot } from "react-dom/client";
import { FlowDataEdge } from "../../packages/ui/components/flow/flow-data-edge";
import { FlowExecutionEdge } from "../../packages/ui/components/flow/flow-execution-edge";
import { isWebkitLite } from "../../packages/ui/lib/platform";
import type { BoardStyle, Fixture, RendererHandle } from "./types";
import "@xyflow/react/dist/style.css";
import "./react-renderer.css";

type FixtureNodeData = {
	label: string;
	kind: string;
	running: boolean;
	width: number;
	height: number;
	pinStart: number;
	pinStep: number;
	pinLabels: string[];
};
type FixtureNode = Node<FixtureNodeData, "benchmark">;

const PIN_COLORS = [
	"var(--foreground)",
	"var(--pin-string)",
	"var(--pin-integer)",
	"var(--pin-float)",
];

// The fixture reproduces the canvas chrome without mounting editor services.
// Edges below are the production components, including their platform rules.
const BenchmarkNode = memo(function BenchmarkNode({
	data,
	selected,
}: NodeProps<FixtureNode>) {
	return (
		<div
			className={`benchmark-fixture-node${selected ? " is-selected" : ""}`}
			style={{ width: data.width, height: data.height }}
		>
			<div className={`benchmark-fixture-header kind-${data.kind}`}>
				<WorkflowIcon className="benchmark-fixture-icon" />
				<span className="benchmark-fixture-title">{data.label}</span>
				{data.running && <span className="benchmark-fixture-running" />}
			</div>
			{data.pinLabels.map((label, index) => (
				<div key={label}>
					{(["input", "output"] as const).map((side) => (
						<Handle
							key={side}
							id={`${side}-${index}`}
							type={side === "input" ? "target" : "source"}
							position={side === "input" ? Position.Left : Position.Right}
							className={`benchmark-fixture-handle ${side}`}
							style={{ top: data.pinStart + index * data.pinStep - 1 }}
						>
							<span
								className={`benchmark-fixture-pin ${index === 0 ? "execution" : "data"}`}
								style={{ backgroundColor: PIN_COLORS[index] }}
							/>
							<span className="benchmark-fixture-label">{label}</span>
						</Handle>
					))}
				</div>
			))}
		</div>
	);
});

const NODE_TYPES = { benchmark: BenchmarkNode };
const EDGE_TYPES = {
	execution: FlowExecutionEdge,
	data: FlowDataEdge,
};

const rgba = (color: number[]) =>
	`rgba(${color[0]}, ${color[1]}, ${color[2]}, ${(color[3] ?? 255) / 255})`;

export async function mountReactRenderer(
	host: HTMLElement,
	fixture: Fixture,
	style: BoardStyle,
): Promise<RendererHandle> {
	const element = document.createElement("div");
	element.className = "benchmark-react-renderer";
	const variables: Partial<Record<keyof BoardStyle, string>> = {
		background: "--background",
		card: "--card",
		foreground: "--foreground",
		border: "--border",
		primary: "--primary",
		tertiary: "--tertiary",
		pinString: "--pin-string",
		pinInteger: "--pin-integer",
		pinFloat: "--pin-float",
	};
	for (const [key, variable] of Object.entries(variables)) {
		element.style.setProperty(variable, rgba(style[key as keyof BoardStyle]));
	}
	host.append(element);
	const root = createRoot(element);
	let motion = false;
	let activityTick = -1;
	let viewportUpdates = 0;
	let movementUpdates = 0;
	let dragEvents = 0;
	let dragStarts = 0;
	let dragStops = 0;
	let reconnectCount = 0;
	let lastDragAt: number | null = null;
	let lastReconnectAt: number | null = null;
	let lastMutationAt: number | null = null;
	let destroyed = false;
	let instance: ReactFlowInstance<FixtureNode, Edge> | undefined;
	const nodeIds = fixture.nodes.map((node) => node.id);
	const nodeIndex = new Map(nodeIds.map((id, index) => [id, index]));
	const requireNodeIndex = (id: string) => {
		const index = nodeIndex.get(id);
		if (index === undefined) throw new Error(`Unknown fixture node ${id}`);
		return index;
	};
	const defaultNodes: FixtureNode[] = fixture.nodes.map((node) => ({
		id: node.id,
		type: "benchmark",
		position: { x: node.x, y: node.y },
		width: fixture.nodeWidth,
		height: fixture.nodeHeight,
		data: {
			label: node.label,
			kind: node.kind,
			running: false,
			width: fixture.nodeWidth,
			height: fixture.nodeHeight,
			pinStart: fixture.pinStart,
			pinStep: fixture.pinStep,
			pinLabels: fixture.pinLabels,
		},
	}));
	const defaultEdges: Edge[] = fixture.edges.map((edge) => ({
		id: edge.id,
		source: nodeIds[edge.source],
		target: nodeIds[edge.target],
		sourceHandle: `output-${edge.sourcePin}`,
		targetHandle: `input-${edge.targetPin}`,
		type: edge.kind,
		style: {
			stroke: PIN_COLORS[edge.kind === "execution" ? 0 : edge.sourcePin],
		},
		data: { pathType: "default", reduceMotion: true },
	}));

	await new Promise<void>((resolve) => {
		root.render(
			<ReactFlow<FixtureNode, Edge>
				defaultNodes={defaultNodes}
				defaultEdges={defaultEdges}
				nodeTypes={NODE_TYPES}
				edgeTypes={EDGE_TYPES}
				onlyRenderVisibleElements
				minZoom={0.01}
				maxZoom={3}
				defaultViewport={{ x: 24, y: 24, zoom: 0.8 }}
				deleteKeyCode={null}
				edgesReconnectable
				isValidConnection={(connection) =>
					connection.source !== connection.target &&
					(connection.sourceHandle === "output-0") ===
						(connection.targetHandle === "input-0")
				}
				onNodesChange={(changes) => {
					if (
						changes.some(
							(change) => change.type === "position" && change.position,
						)
					)
						lastMutationAt = performance.now();
				}}
				onNodeDragStart={() => {
					dragStarts++;
				}}
				onNodeDrag={() => {
					dragEvents++;
					lastDragAt = performance.now();
				}}
				onNodeDragStop={() => {
					dragStops++;
					lastDragAt = performance.now();
				}}
				onReconnect={(edge, connection) => {
					instance?.setEdges((edges) =>
						reconnectEdge(edge, connection, edges, { shouldReplaceId: false }),
					);
					reconnectCount++;
					lastReconnectAt = performance.now();
					lastMutationAt = lastReconnectAt;
				}}
				proOptions={{ hideAttribution: true }}
				onInit={(flow) => {
					instance = flow;
					requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
				}}
			/>,
		);
	});

	return {
		async prepareInteraction() {
			if (destroyed || !instance)
				throw new Error("The renderer is not mounted");
			const viewport = instance.getViewport();
			const bounds = element.getBoundingClientRect();
			const visible = instance.getNodes().filter((node) => {
				const x = node.position.x * viewport.zoom + viewport.x;
				const y = node.position.y * viewport.zoom + viewport.y;
				return (
					x > 24 &&
					y > 24 &&
					x + fixture.nodeWidth * viewport.zoom < bounds.width - 24 &&
					y + fixture.nodeHeight * viewport.zoom < bounds.height - 24
				);
			});
			const visibleIds = new Set(visible.map((node) => node.id));
			const center = (node: FixtureNode) => ({
				x:
					(node.position.x + fixture.nodeWidth / 2) * viewport.zoom +
					viewport.x,
				y:
					(node.position.y + fixture.nodeHeight / 2) * viewport.zoom +
					viewport.y,
			});
			visible.sort((a, b) => {
				const pa = center(a);
				const pb = center(b);
				return (
					Math.hypot(pa.x - bounds.width / 2, pa.y - bounds.height / 2) -
					Math.hypot(pb.x - bounds.width / 2, pb.y - bounds.height / 2)
				);
			});
			const edges = instance.getEdges();
			const incoming = new Map(
				edges
					.filter(
						(candidate) =>
							candidate.type === "execution" &&
							visibleIds.has(candidate.source),
					)
					.map((edge) => [edge.target, edge]),
			);
			const edge = visible.map((node) => incoming.get(node.id)).find(Boolean);
			if (!edge)
				throw new Error(
					"No visible execution edge is available for the interaction trial",
				);
			const alternative = visible.find(
				(node) => node.id !== edge.target && node.id !== edge.source,
			);
			const dragNode =
				visible.find(
					(node) =>
						node.id !== edge.target &&
						node.id !== edge.source &&
						node.id !== alternative?.id,
				) ?? visible[0];
			if (!alternative || !dragNode)
				throw new Error(
					"The interaction trial needs at least three visible nodes",
				);
			instance.setEdges((current) =>
				current.map((candidate) => ({
					...candidate,
					selected: candidate.id === edge.id,
				})),
			);
			await new Promise<void>((resolve) =>
				requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
			);
			const anchor = element.querySelector(
				`[data-id="${CSS.escape(edge.id)}"] .react-flow__edgeupdater-target`,
			);
			const target = element.querySelector(
				`[data-id="${CSS.escape(alternative.id)}"] [data-handleid="input-0"]`,
			);
			if (!anchor || !target)
				throw new Error("React Flow did not mount the reconnect handles");
			const localCenter = (child: Element) => {
				const rect = child.getBoundingClientRect();
				return {
					x: (rect.left + rect.right) / 2 - bounds.left,
					y: (rect.top + rect.bottom) / 2 - bounds.top,
				};
			};
			return {
				node: { index: requireNodeIndex(dragNode.id), ...center(dragNode) },
				reconnect: {
					edgeIndex: fixture.edges.findIndex(
						(candidate) => candidate.id === edge.id,
					),
					from: localCenter(anchor),
					to: localCenter(target),
					targetIndex: requireNodeIndex(alternative.id),
					targetPin: 0,
				},
			};
		},
		getNodePosition(index) {
			const node = instance?.getNode(nodeIds[index]);
			if (!node) throw new Error(`Unknown node index ${index}`);
			return { ...node.position };
		},
		getEdgeTarget(index) {
			const edge = instance?.getEdge(fixture.edges[index]?.id);
			if (!edge) throw new Error(`Unknown edge index ${index}`);
			return {
				targetIndex: requireNodeIndex(edge.target),
				targetPin: Number(edge.targetHandle?.split("-").at(-1)),
			};
		},
		async setViewport(viewport) {
			if (destroyed || !instance) return;
			viewportUpdates++;
			await instance.setViewport(viewport, { duration: 0 });
		},
		setMotion(enabled) {
			if (destroyed || motion === enabled || !instance) return;
			motion = enabled;
			instance.setEdges((edges) =>
				edges.map((edge) => ({
					...edge,
					data: { ...edge.data, reduceMotion: !enabled },
				})),
			);
		},
		setActivity(tick) {
			if (destroyed || activityTick === tick || !instance) return;
			activityTick = tick;
			const remainder = tick < 0 || tick === 0xffffffff ? -1 : tick % 100;
			instance.setNodes((nodes) =>
				nodes.map((node) => {
					const running = (nodeIndex.get(node.id) ?? -1) % 100 === remainder;
					return node.data.running === running
						? node
						: { ...node, data: { ...node.data, running } };
				}),
			);
		},
		moveNodes(indices, dx, dy) {
			if (destroyed || !instance) return;
			movementUpdates++;
			const ids = new Set(indices.map((index) => nodeIds[index]));
			instance.setNodes((nodes) =>
				nodes.map((node) =>
					ids.has(node.id)
						? {
								...node,
								position: { x: node.position.x + dx, y: node.position.y + dy },
							}
						: node,
				),
			);
		},
		stats() {
			return {
				renderer: "React Flow",
				scope: "renderer-only fixture; production execution and data edges",
				nodeComponent: "benchmark chrome subset, not production FlowNode",
				nodeCount: instance?.getNodes().length ?? 0,
				edgeCount: instance?.getEdges().length ?? 0,
				renderedNodes: element.querySelectorAll(".react-flow__node").length,
				renderedEdges: element.querySelectorAll(".react-flow__edge").length,
				svgPaths: element.querySelectorAll(".react-flow__edges path").length,
				onlyRenderVisibleElements: true,
				minimumZoom: 0.01,
				motionRequested: motion,
				motionEffective: motion && !isWebkitLite(),
				activityTick,
				activeNodeCount:
					instance?.getNodes().filter((node) => node.data.running).length ?? 0,
				activitySource: "deterministic fixture updates, no execution services",
				viewportUpdates,
				movementUpdates,
				dragEvents,
				dragStarts,
				dragStops,
				reconnectCount,
				lastDragAt,
				lastReconnectAt,
				lastMutationAt,
				latestMutationAtMs: lastMutationAt,
			};
		},
		destroy() {
			if (destroyed) return;
			destroyed = true;
			root.unmount();
			element.remove();
		},
	};
}
