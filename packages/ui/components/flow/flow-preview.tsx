
import { useEffect, useMemo } from "react";
import { IExecutionStage, parseBoard, type IBoard, type INode } from "../../lib";
import { Background, BackgroundVariant, Controls, MiniMap, ReactFlow, useEdgesState, useNodesState } from "@xyflow/react";
import { useTheme } from "next-themes";
import { FlowNode } from "./flow-node";
import { CommentNode } from "./comment-node";
import '@xyflow/react/dist/style.css';

export function FlowPreview({ nodes }: Readonly<{ nodes: INode[] }>) {
    const [boardNodes, setNodes] = useNodesState<any>([]);
    const { resolvedTheme } = useTheme()
    const [edges, setEdges] = useEdgesState<any>([]);
    const nodeTypes = useMemo(() => ({ flowNode: FlowNode, commentNode: CommentNode }), []);

    useEffect(() => {
        const parsed: {
            [key: string]: INode;
        } = {}

        nodes.forEach((node) => {
            parsed[node.id] = node;
        })

        const board: IBoard = {
            comments: {},
            created_at: {
                nanos_since_epoch: 0,
                secs_since_epoch: 0
            },
            description: "",
            id: "",
            name: "",
            nodes: parsed,
            refs: {},
            stage: IExecutionStage.Dev,
            updated_at: {
                nanos_since_epoch: 0,
                secs_since_epoch: 0
            },
            version: [0, 0, 0],
            variables: {},
            viewport: [0, 0, 0, 0]
        }

        const parsedBoard = parseBoard(board, async () => { }, async () => { }, async () => { }, new Set())

        console.dir(parsedBoard)

        setNodes(parsedBoard.nodes)
        setEdges(parsedBoard.edges)
    }, [nodes])

    return <main className="w-full h-full min-h-28">
        <ReactFlow
            suppressHydrationWarning
            className="w-full h-full"
            colorMode={resolvedTheme === 'dark' ? 'dark' : 'light'}
            nodes={boardNodes}
            nodeTypes={nodeTypes}
            edges={edges}
            fitView
            proOptions={{ hideAttribution: true }}
        >
            <Controls />
            <MiniMap />
            <Background variant={BackgroundVariant.Dots} gap={12} size={1} />
        </ReactFlow>
    </main>
}