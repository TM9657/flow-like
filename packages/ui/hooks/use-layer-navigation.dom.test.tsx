import { afterEach, describe, expect, mock, test } from "bun:test";
import type { UseQueryResult } from "@tanstack/react-query";
import type { ReactFlowInstance } from "@xyflow/react";
import { Window } from "happy-dom";
import { act, createElement, useState } from "react";
import { type IBoard, type ILayer, ILayerType } from "../lib/schema/flow/board";
import { useLayerNavigation } from "./use-layer-navigation";

const window = new Window({ url: "https://localhost" });
const frames = new Map<number, FrameRequestCallback>();
let frameId = 0;
Object.assign(globalThis, {
	window,
	document: window.document,
	navigator: window.navigator,
	HTMLElement: window.HTMLElement,
	IS_REACT_ACT_ENVIRONMENT: true,
	requestAnimationFrame: (callback: FrameRequestCallback) => {
		const id = ++frameId;
		frames.set(id, callback);
		return id;
	},
	cancelAnimationFrame: (id: number) => frames.delete(id),
});

const { createRoot } = await import("react-dom/client");
const roots: ReturnType<typeof createRoot>[] = [];

afterEach(() => {
	act(() => {
		for (const root of roots.splice(0)) root.unmount();
	});
	frames.clear();
});

function layer(id: string, parentId: string | null, type: ILayerType): ILayer {
	return {
		id,
		name: id,
		parent_id: parentId,
		type,
		pins: {},
	} as ILayer;
}

const layers = {
	module: layer("module", null, ILayerType.Module),
	other: layer("other", null, ILayerType.Module),
	child: layer("child", "module", ILayerType.Collapsed),
	localFn: layer("localFn", "module", ILayerType.Function),
	sharedFn: layer("sharedFn", null, ILayerType.Function),
};

function mount(initialPath = "module", saveViewport = mock(async () => {})) {
	let navigation!: ReturnType<typeof useLayerNavigation>;
	let path: string | undefined;
	let changeKey!: (key: string) => void;
	const fitView = mock(async () => true);
	const release = mock(() => {});

	function Harness() {
		const [layerPath, setLayerPath] = useState<string | undefined>(initialPath);
		const [key, setKey] = useState("left");
		const [, setCurrentLayer] = useState<string | undefined>();
		path = layerPath;
		changeKey = setKey;
		navigation = useLayerNavigation({
			navigationKey: key,
			board: {
				data: { layers, nodes: {}, comments: {} },
			} as unknown as UseQueryResult<IBoard>,
			layerPath,
			setLayerPath,
			setCurrentLayer,
			saveViewport,
			holdViewport: () => release,
			fitView: fitView as ReactFlowInstance["fitView"],
			getNodes: () => [],
		});
		return null;
	}

	const container = window.document.createElement("div");
	window.document.body.appendChild(container);
	const root = createRoot(container as unknown as HTMLElement);
	roots.push(root);
	act(() => root.render(createElement(Harness)));
	return {
		get navigation() {
			return navigation;
		},
		get path() {
			return path;
		},
		fitView,
		release,
		switchTab(key: string, target: string, copyNavigationKey?: string) {
			act(() => {
				changeKey(key);
				navigation.navigateToLayer(target, {
					navigationKey: key,
					copyNavigationKey,
				});
			});
		},
	};
}

describe("useLayerNavigation", () => {
	test("back remains in the module while an earlier viewport save is pending", async () => {
		let finishSave!: () => void;
		const pendingSave = new Promise<void>((resolve) => {
			finishSave = resolve;
		});
		const view = mount(
			"module",
			mock(() => pendingSave),
		);
		let entering!: Promise<void>;
		act(() => {
			entering = view.navigation.pushLayer(layers.child);
		});
		expect(view.path).toBe("module/child");
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("module");
		await act(async () => {
			finishSave();
			await entering;
		});
		expect(view.path).toBe("module");
	});

	test("cached node callbacks read the current module as their caller", async () => {
		const view = mount("other");
		const cachedPush = view.navigation.pushLayer;
		act(() => view.navigation.navigateToLayer("module"));
		await act(async () => {
			await cachedPush(layers.sharedFn);
		});
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("module");
	});

	test("entering and leaving within the same tick still returns to the caller", async () => {
		const view = mount();
		await act(async () => {
			const entering = view.navigation.pushLayer(layers.sharedFn);
			view.navigation.popLayer();
			await entering;
		});
		expect(view.path).toBe("module");
	});

	test("a restored short function path resolves its owning module", () => {
		const view = mount("other");
		act(() => view.navigation.navigateToLayer("localFn"));
		expect(view.path).toBe("module/localFn");
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("module");
	});

	test("cached layer objects use the current ownership from the board", async () => {
		const view = mount();
		const staleFunction = { ...layers.localFn, parent_id: null };
		await act(async () => {
			await view.navigation.pushLayer(staleFunction);
		});
		expect(view.path).toBe("module/localFn");
	});

	test("each tab remembers its own caller for the same function", async () => {
		const view = mount();
		await act(async () => {
			await view.navigation.pushLayer(layers.sharedFn);
		});
		view.switchTab("right", "other");
		await act(async () => {
			await view.navigation.pushLayer(layers.sharedFn);
		});
		view.switchTab("left", "sharedFn");
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("module");
		view.switchTab("right", "sharedFn");
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("other");
	});

	test("splitting a tab copies its caller without consuming the source trail", async () => {
		const view = mount();
		await act(async () => {
			await view.navigation.pushLayer(layers.sharedFn);
		});
		view.switchTab("split", "sharedFn", "left");
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("module");
		view.switchTab("left", "sharedFn");
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("module");
	});

	test("a reused split key starts with the new source caller", async () => {
		const view = mount();
		await act(async () => {
			await view.navigation.pushLayer(layers.sharedFn);
		});
		view.switchTab("split", "sharedFn", "left");
		view.switchTab("left", "other");
		await act(async () => {
			await view.navigation.pushLayer(layers.sharedFn);
		});
		view.switchTab("split", "sharedFn", "left");
		act(() => view.navigation.popLayer());
		expect(view.path).toBe("other");
	});

	test("a closed tab's caller cannot be inherited by a newly opened tab", async () => {
		const view = mount();
		await act(async () => {
			await view.navigation.pushLayer(layers.sharedFn);
		});
		view.switchTab("right", "other");
		view.navigation.forgetNavigation("left");
		view.switchTab("left", "sharedFn");
		act(() => view.navigation.popLayer());
		expect(view.path).toBeUndefined();
	});

	test("a direct jump cancels a pending focus before it moves the new viewport", () => {
		const view = mount();
		act(() => view.navigation.focusNode("sharedFn"));
		expect(frames.size).toBe(1);
		act(() => view.navigation.navigateToLayer("other"));
		expect(frames.size).toBe(0);
		expect(view.fitView).not.toHaveBeenCalled();
		expect(view.release).toHaveBeenCalledTimes(1);
		expect(view.path).toBe("other");
	});
});
