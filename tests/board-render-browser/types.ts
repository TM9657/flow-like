export interface FixtureNode {
	id: string;
	x: number;
	y: number;
	label: string;
	kind: "event" | "function" | "pure" | "standard";
}

export interface FixtureEdge {
	id: string;
	source: number;
	target: number;
	sourcePin: number;
	targetPin: number;
	kind: "execution" | "data";
}

export interface Fixture {
	nodes: FixtureNode[];
	edges: FixtureEdge[];
	nodeWidth: number;
	nodeHeight: number;
	pinStart: number;
	pinStep: number;
	pinLabels: string[];
	width: number;
	height: number;
	seed: number;
}

export type Rgba = [number, number, number, number];
export interface BoardStyle {
	background: Rgba;
	card: Rgba;
	foreground: Rgba;
	border: Rgba;
	primary: Rgba;
	tertiary: Rgba;
	pinExecution: Rgba;
	pinString: Rgba;
	pinFloat: Rgba;
	pinInteger: Rgba;
}

export interface Viewport {
	x: number;
	y: number;
	zoom: number;
}

export type RendererStats = Record<string, unknown>;
export interface InteractionTargets {
	node: { index: number; x: number; y: number };
	reconnect: {
		edgeIndex: number;
		from: { x: number; y: number };
		to: { x: number; y: number };
		targetIndex: number;
		targetPin: number;
	};
}
export interface RendererHandle {
	setViewport(viewport: Viewport): void | Promise<void>;
	setMotion(enabled: boolean): void;
	setActivity(tick: number): void;
	moveNodes(indices: number[], dx: number, dy: number): void;
	stats(): RendererStats;
	prepareInteraction(): InteractionTargets | Promise<InteractionTargets>;
	getNodePosition(index: number): { x: number; y: number };
	getEdgeTarget(index: number): { targetIndex: number; targetPin: number };
	destroy(): void;
}
