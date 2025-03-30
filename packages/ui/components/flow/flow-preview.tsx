import {
	Background,
	BackgroundVariant,
	Controls,
	MiniMap,
	ReactFlow,
	useEdgesState,
	useNodesState,
} from "@xyflow/react";
import { useTheme } from "next-themes";
import { useEffect, useMemo } from "react";
import {
	type IBoard,
	IExecutionStage,
	ILogLevel,
	type INode,
	parseBoard,
} from "../../lib";
import { CommentNode } from "./comment-node";
import { FlowNode } from "./flow-node";
import "@xyflow/react/dist/style.css";
import { PreviewFlowNode } from "./preview/preview-node";

export function FlowPreview({ nodes }: Readonly<{ nodes: INode[] }>) {
	const [boardNodes, setNodes] = useNodesState<any>([]);
	const { resolvedTheme } = useTheme();
	const [edges, setEdges] = useEdgesState<any>([]);
	const nodeTypes = useMemo(
		() => ({ flowNode: PreviewFlowNode, commentNode: CommentNode }),
		[],
	);

	useEffect(() => {
		const parsed: {
			[key: string]: INode;
		} = {};

		nodes.forEach((node) => {
			parsed[node.id] = node;
		});

		const board: IBoard = {
			comments: {},
			created_at: {
				nanos_since_epoch: 0,
				secs_since_epoch: 0,
			},
			description: "",
			id: "",
			log_level: ILogLevel.Info,
			name: "",
			nodes: parsed,
			refs: {},
			stage: IExecutionStage.Dev,
			updated_at: {
				nanos_since_epoch: 0,
				secs_since_epoch: 0,
			},
			version: [0, 0, 0],
			variables: {},
			viewport: [0, 0, 0, 0],
		};

		const parsedBoard = parseBoard(
			board,
			"",
			async () => {},
			async () => {},
			async () => {},
			new Set(),
		);

		console.dir(parsedBoard);

		setNodes(parsedBoard.nodes);
		setEdges(parsedBoard.edges);
	}, [nodes]);

	return (
		<main className="w-full h-full min-h-56 rounded-md flow-preview not-content">
			<ReactFlow
				suppressHydrationWarning
				className="w-0 h-0 min-h-0 dark:w-full dark:h-full dark:min-h-56 rounded-lg"
				colorMode={"dark"}
				nodes={boardNodes}
				nodeTypes={nodeTypes}
				fitView
				fitViewOptions={{
					padding: 0.8,
				}}
				edges={edges}
				proOptions={{ hideAttribution: true }}
			>
				<Controls />
				<Background variant={BackgroundVariant.Dots} gap={12} size={1} />
			</ReactFlow>
			<ReactFlow
				suppressHydrationWarning
				className="w-full h-full min-h-56 dark:min-h-0 dark:w-0 dark:h-0 rounded-lg"
				colorMode={"light"}
				fitView
				fitViewOptions={{
					padding: 0.8,
				}}
				nodes={boardNodes}
				nodeTypes={nodeTypes}
				edges={edges}
				proOptions={{ hideAttribution: true }}
			>
				<Controls />
				<Background variant={BackgroundVariant.Dots} gap={12} size={1} />
			</ReactFlow>
		</main>
	);
}
