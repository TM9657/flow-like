import { afterEach, describe, expect, spyOn, test } from "bun:test";
import { Window } from "happy-dom";
import { act } from "react";
import type { Root } from "react-dom/client";
import { applyElementUpdate } from "./apply-a2ui-message";
import type { SurfaceComponent } from "./types";

let root: Root | undefined;
const cleanup: (() => void)[] = [];

afterEach(async () => {
	await act(() => root?.unmount());
	root = undefined;
	for (const restore of cleanup.reverse()) restore();
	cleanup.length = 0;
});

function installDom(): Window {
	const window = new Window({ url: "https://local/use" });
	const globals = {
		window,
		document: window.document,
		navigator: window.navigator,
		CustomEvent: window.CustomEvent,
		IS_REACT_ACT_ENVIRONMENT: true,
	};
	const descriptors = Object.keys(globals).map(
		(key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)] as const,
	);
	Object.assign(globalThis, globals);
	cleanup.push(() => {
		for (const [key, descriptor] of descriptors) {
			if (descriptor) Object.defineProperty(globalThis, key, descriptor);
			else Reflect.deleteProperty(globalThis, key);
		}
	});
	return window;
}

const field = (id: string, value: string): SurfaceComponent =>
	({
		id,
		component: { type: "textField", value: { literalString: value } },
	}) as unknown as SurfaceComponent;

function setValue(
	components: Record<string, SurfaceComponent>,
	id: string,
	value: string,
): Record<string, SurfaceComponent> {
	return {
		...components,
		[id]: applyElementUpdate(components[id], { type: "setValue", value }),
	};
}

const sentValue = (elements: Record<string, unknown>, elementId: string) =>
	(elements[elementId] as { component: { value: unknown } }).component.value;

describe("flow writes and the run payload", () => {
	test("an action after a flow write sends the value the screen shows", async () => {
		const window = installDom();
		const [
			{ createRoot },
			{ ActionProvider, useCollectEventElements, useOnAction },
			{ useBackendStore },
			uiState,
			{ AppRouterContext },
			{ PathnameContext },
		] = await Promise.all([
			import("react-dom/client"),
			import("./ActionHandler"),
			import("../../state/backend-state"),
			import("../../db/ui-state-db"),
			import("next/dist/shared/lib/app-router-context.shared-runtime"),
			import("next/dist/shared/lib/hooks-client-context.shared-runtime"),
		]);

		const persisted: [string, unknown][] = [];
		const spies = [
			spyOn(uiState.uiElementValues, "getAll").mockResolvedValue({
				"page/kcal": {
					id: "restored",
					appId: "app",
					elementId: "page/kcal",
					value: "208",
					updatedAt: Date.now(),
				},
			}),
			spyOn(uiState.uiElementValues, "set").mockImplementation(
				async (_appId, elementId, value) => {
					persisted.push([elementId, value]);
				},
			),
			spyOn(uiState, "pruneElementValues").mockResolvedValue(),
		];
		cleanup.push(() => {
			for (const spy of spies) spy.mockRestore();
		});

		const previousBackend = useBackendStore.getState().backend;
		cleanup.push(() => useBackendStore.setState({ backend: previousBackend }));
		useBackendStore
			.getState()
			.setBackend({ boardState: {}, eventState: {} } as never);

		const probe: {
			onAction?: ReturnType<typeof useOnAction>;
			collect?: ReturnType<typeof useCollectEventElements>;
		} = {};
		function Probe() {
			probe.onAction = useOnAction();
			probe.collect = useCollectEventElements();
			return null;
		}

		const appId = `flow-values-${crypto.randomUUID()}`;
		const render = (components: Record<string, SurfaceComponent>) =>
			act(async () => {
				root?.render(
					<AppRouterContext.Provider value={{} as never}>
						<PathnameContext.Provider value="/use">
							<ActionProvider
								appId={appId}
								surfaceId="page"
								isPreviewMode
								components={components}
							>
								<Probe />
							</ActionProvider>
						</PathnameContext.Provider>
					</AppRouterContext.Provider>,
				);
			});
		const collect = () => {
			if (!probe.collect) throw new Error("ActionProvider probe did not mount");
			return probe.collect();
		};
		const settle = (ms = 0) =>
			act(() => new Promise((resolve) => setTimeout(resolve, ms)));
		const edit = (value: string) =>
			act(() => {
				probe.onAction?.({
					type: "userAction",
					name: "change",
					surfaceId: "page",
					sourceComponentId: "kcal",
					timestamp: Date.now(),
					context: { value },
				});
			});

		const host = window.document.createElement("div");
		window.document.body.appendChild(host);
		root = createRoot(host as unknown as HTMLElement);

		let surface: Record<string, SurfaceComponent> = {
			kcal: field("kcal", "210"),
		};
		await render(surface);
		await settle();
		expect(sentValue(await collect(), "page/kcal")).toEqual({
			literalString: "208",
		});

		await edit("211");
		surface = setValue(surface, "kcal", "405");
		await render(surface);
		expect(sentValue(await collect(), "page/kcal")).toEqual({
			literalString: "405",
		});

		await settle(350);
		expect(persisted).toEqual([["page/kcal", "405"]]);

		await edit("410");
		await render({ ...surface });
		expect(sentValue(await collect(), "page/kcal")).toEqual({
			literalString: "410",
		});
	});
});
