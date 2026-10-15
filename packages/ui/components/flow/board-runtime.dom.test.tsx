import { afterAll, afterEach, expect, test } from "bun:test";
import { ReactFlowProvider } from "@xyflow/react";
import { Window } from "happy-dom";
import { StrictMode, act, useLayoutEffect } from "react";
import {
	type BoardRuntime,
	BoardRuntimeProvider,
	LegacyBoardRuntimeScope,
	useBoardRuntime,
	useBoardRuntimeMode,
	useReactFlow,
	useStore,
	useViewport,
} from "./board-runtime";

const window = new Window({ url: "https://localhost" });
Object.assign(window, { SyntaxError, TypeError, Error });
const globals = {
	window,
	document: window.document,
	navigator: window.navigator,
	HTMLElement: window.HTMLElement,
	Element: window.Element,
	Node: window.Node,
	IS_REACT_ACT_ENVIRONMENT: true,
};
const descriptors = Object.keys(globals).map(
	(key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)] as const,
);
Object.assign(globalThis, globals);
const { createRoot } = await import("react-dom/client");
const roots: ReturnType<typeof createRoot>[] = [];

afterEach(async () => {
	await act(async () => {
		for (const root of roots.splice(0)) root.unmount();
	});
	window.document.body.replaceChildren();
});
afterAll(() => {
	for (const [key, descriptor] of descriptors) {
		if (descriptor) Object.defineProperty(globalThis, key, descriptor);
		else Reflect.deleteProperty(globalThis, key);
	}
	window.happyDOM.abort();
});

function container() {
	const element = window.document.createElement("div");
	window.document.body.append(element);
	const root = createRoot(element as unknown as HTMLElement);
	roots.push(root);
	return { element, root };
}

test("native hooks work without RF context and graph selectors skip camera updates under StrictMode", async () => {
	const { element, root } = container();
	let runtime: BoardRuntime | undefined;
	let graphReads = 0;
	function Setup() {
		const value = useBoardRuntime();
		useLayoutEffect(() => {
			runtime = value;
			value?.configure({
				nodes: [
					{
						id: "native",
						position: { x: 0, y: 0 },
						data: { label: "Initial" },
					},
				],
			});
		}, [value]);
		return null;
	}
	function Graph() {
		const label = useStore((state) => {
			graphReads++;
			return state.nodeLookup.get("native")?.data.label;
		});
		return <span data-graph>{String(label)}</span>;
	}
	function Camera() {
		const viewport = useViewport();
		return <span data-camera>{viewport.x}</span>;
	}
	await act(async () =>
		root.render(
			<StrictMode>
				<BoardRuntimeProvider>
					<Setup />
					<Graph />
					<Camera />
				</BoardRuntimeProvider>
			</StrictMode>,
		),
	);
	expect(runtime).toBeDefined();
	expect(element.querySelector("[data-graph]")?.textContent).toBe("Initial");
	const before = graphReads;
	await act(async () => {
		await runtime?.setViewport({ x: 45, y: 20, zoom: 0.5 }, { notify: false });
	});
	expect(element.querySelector("[data-camera]")?.textContent).toBe("45");
	expect(graphReads).toBe(before);
	await act(async () =>
		runtime?.flow.setNodes((nodes) =>
			nodes.map((node) => ({ ...node, data: { label: "Updated" } })),
		),
	);
	expect(element.querySelector("[data-graph]")?.textContent).toBe("Updated");
});

test("independent legacy preview scope cannot observe or edit the outer native graph", async () => {
	const { element, root } = container();
	let runtime: BoardRuntime | undefined;
	function Setup() {
		const value = useBoardRuntime();
		useLayoutEffect(() => {
			runtime = value;
			value?.configure({
				nodes: [{ id: "board", position: { x: 0, y: 0 }, data: {} }],
			});
		}, [value]);
		return null;
	}
	function Preview() {
		const flow = useReactFlow();
		const value = useBoardRuntime();
		const mode = useBoardRuntimeMode();
		return (
			<span
				data-preview
			>{`${flow.getNodes()[0]?.id}:${String(value)}:${String(mode)}`}</span>
		);
	}
	await act(async () =>
		root.render(
			<BoardRuntimeProvider>
				<Setup />
				<LegacyBoardRuntimeScope>
					<ReactFlowProvider
						initialNodes={[
							{ id: "preview", position: { x: 0, y: 0 }, data: {} },
						]}
					>
						<Preview />
					</ReactFlowProvider>
				</LegacyBoardRuntimeScope>
			</BoardRuntimeProvider>,
		),
	);
	expect(element.querySelector("[data-preview]")?.textContent).toBe(
		"preview:undefined:undefined",
	);
	expect(runtime?.flow.getNodes().map((node) => node.id)).toEqual(["board"]);
});
