import { afterEach, beforeEach, describe, expect, spyOn, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { type Root, createRoot } from "react-dom/client";
import type { MicroWidgetInstanceComponent } from "../types";

let window: Window;
let root: Root;
let host: HTMLElement;
let client: import("@tanstack/react-query").QueryClient | undefined;
let restoreFrameSrc: () => void;
let restoreGlobals: () => void;
let restoreTimers: () => void;
let restoreBackend: (() => void) | undefined;
let readyTimeout: (() => void) | undefined;

beforeEach(() => {
	client = undefined;
	restoreBackend = undefined;
	readyTimeout = undefined;
	const scheduleTimeout = globalThis.setTimeout;
	const timerSpy = spyOn(globalThis, "setTimeout");
	timerSpy.mockImplementation(((...args: Parameters<typeof setTimeout>) => {
		const [callback, delay, ...callbackArgs] = args;
		if (delay === 10_000) readyTimeout = () => callback(...callbackArgs);
		return scheduleTimeout(...args);
	}) as typeof setTimeout);
	restoreTimers = () => timerSpy.mockRestore();
	window = new Window({ url: "https://local/use" });
	const globals = {
		document: window.document,
		HTMLElement: window.HTMLElement,
		Node: window.Node,
		navigator: window.navigator,
		getComputedStyle: window.getComputedStyle.bind(window),
		requestAnimationFrame: window.requestAnimationFrame.bind(window),
		cancelAnimationFrame: window.cancelAnimationFrame.bind(window),
		window,
		IS_REACT_ACT_ENVIRONMENT: true,
	};
	const descriptors = Object.keys(globals).map(
		(key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)] as const,
	);
	Object.assign(globalThis, globals);
	Object.assign(window, { __TAURI_INTERNALS__: {}, SyntaxError });
	restoreGlobals = () => {
		for (const [key, descriptor] of descriptors) {
			if (descriptor) Object.defineProperty(globalThis, key, descriptor);
			else Reflect.deleteProperty(globalThis, key);
		}
	};

	// Keep the real frame lifecycle while loading a blank document without network access.
	const framePrototype = window.HTMLIFrameElement.prototype;
	const src = Object.getOwnPropertyDescriptor(framePrototype, "src");
	if (!src) throw new Error("The iframe source descriptor is missing");
	Object.defineProperty(framePrototype, "src", {
		...src,
		get: () => "about:blank",
	});
	restoreFrameSrc = () => Object.defineProperty(framePrototype, "src", src);
	host = window.document.createElement("div") as unknown as HTMLElement;
	window.document.body.appendChild(host as never);
	root = createRoot(host);
});

afterEach(async () => {
	await act(() => root.unmount());
	client?.clear();
	restoreBackend?.();
	restoreTimers();
	restoreFrameSrc();
	await window.happyDOM.abort();
	restoreGlobals();
});

const component = (
	overrides: Partial<MicroWidgetInstanceComponent> = {},
): MicroWidgetInstanceComponent => ({
	id: "sales-chart",
	type: "microWidgetInstance",
	instanceId: "sales-chart",
	packageId: "com.example.sales",
	widgetId: "chart",
	packageVersion: "1.0.0",
	bundleHash: "old-bundle",
	props: { title: "Sales" },
	...overrides,
});

async function renderWidget(widget: MicroWidgetInstanceComponent) {
	const [
		{ A2UIMicroWidget },
		{ useBackendStore },
		{ QueryClient, QueryClientProvider },
		{ AppRouterContext },
	] = await Promise.all([
		import("./A2UIMicroWidget"),
		import("../../../state/backend-state"),
		import("@tanstack/react-query"),
		import("next/dist/shared/lib/app-router-context.shared-runtime"),
	]);
	if (!client) {
		client = new QueryClient();
		const previousBackend = useBackendStore.getState().backend;
		restoreBackend = () =>
			useBackendStore.setState({ backend: previousBackend });
		useBackendStore.getState().setBackend({
			userState: { getProfile: async () => null },
		} as never);
	}
	await act(async () => {
		root.render(
			createElement(
				AppRouterContext.Provider,
				{ value: {} as never },
				createElement(
					QueryClientProvider,
					{ client } as never,
					createElement(A2UIMicroWidget, {
						component: widget as never,
						componentId: "sales-chart",
						surfaceId: "page-1",
					} as never),
				),
			),
		);
	});
}

describe("micro widget bundle refresh", () => {
	test("retries a failed widget when a synced bundle replaces its old revision", async () => {
		await renderWidget(component());
		expect(host.querySelector("iframe") !== null).toBe(true);
		expect(readyTimeout).toBeDefined();
		await act(() => readyTimeout?.());
		expect(host.querySelector("iframe") === null).toBe(true);
		expect(host.textContent).toContain("did not become ready");

		await renderWidget(
			component({ packageVersion: "1.0.1", bundleHash: "synced-bundle" }),
		);
		const syncedFrame = host.querySelector("iframe");
		expect(syncedFrame).not.toBeNull();
		expect(syncedFrame?.getAttribute("src")).toContain("synced-bundle");
		expect(host.textContent).not.toContain("did not become ready");
	});

	test("keeps the live iframe for ordinary input updates", async () => {
		await renderWidget(component());
		const frame = host.querySelector("iframe");
		expect(frame).not.toBeNull();

		await renderWidget(component({ props: { title: "Updated sales" } }));
		expect(host.querySelector("iframe")).toBe(frame);
	});
});
