import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";
import { renderToString } from "react-dom/server";
import {
	setBoardRendererPreference,
	useBoardRenderer,
} from "./use-board-renderer";

let browser: Window;
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
const { createRoot } = await import("react-dom/client");
const roots: ReturnType<typeof createRoot>[] = [];

beforeEach(() => {
	browser = new Window({ url: "https://localhost" });
	Object.assign(globalThis, {
		window: browser,
		document: browser.document,
		navigator: browser.navigator,
		HTMLElement: browser.HTMLElement,
	});
});

afterEach(() => {
	act(() => {
		for (const root of roots.splice(0)) root.unmount();
	});
});

function mount() {
	let current!: ReturnType<typeof useBoardRenderer>;
	function Probe() {
		current = useBoardRenderer();
		return null;
	}
	const root = createRoot(
		browser.document.createElement("div") as unknown as HTMLElement,
	);
	roots.push(root);
	act(() => root.render(createElement(Probe)));
	return () => current;
}

describe("board renderer preference", () => {
	test("defaults to WASM without persisting an implicit choice", () => {
		const current = mount();
		expect(current().renderer).toBe("egui");
		expect(current().ready).toBe(true);
		expect(browser.localStorage.getItem("flow-board-renderer")).toBeNull();
	});

	test("preserves an explicit React Flow opt-out and defaults invalid values to WASM", () => {
		browser.localStorage.setItem("flow-board-renderer", "react-flow");
		expect(mount()().renderer).toBe("react-flow");
		act(() => {
			browser.dispatchEvent(
				new browser.StorageEvent("storage", {
					key: "flow-board-renderer",
					newValue: "unknown",
					storageArea: browser.localStorage,
				}),
			);
		});
		expect(mount()().renderer).toBe("egui");
	});

	test("explicit settings update every mounted consumer and persist the raw value", () => {
		const first = mount();
		const second = mount();
		act(() => first().setRenderer("egui"));
		expect(first().renderer).toBe("egui");
		expect(second().renderer).toBe("egui");
		expect(browser.localStorage.getItem("flow-board-renderer")).toBe("egui");
		act(() => setBoardRendererPreference("react-flow"));
		expect(first().renderer).toBe("react-flow");
		expect(second().renderer).toBe("react-flow");
	});

	test("the setter can initialize a fixture before any hook mounts", () => {
		setBoardRendererPreference("egui");
		expect(mount()().renderer).toBe("egui");
	});

	test("other-tab changes and storage clearing update consumers, but session storage does not", () => {
		const current = mount();
		const change = (
			key: string | null,
			newValue: string | null,
			session = false,
		) =>
			act(() => {
				browser.dispatchEvent(
					new browser.StorageEvent("storage", {
						key: key ?? undefined,
						newValue: newValue ?? undefined,
						storageArea: session
							? browser.sessionStorage
							: browser.localStorage,
					}),
				);
			});
		change("unrelated", "react-flow");
		change("flow-board-renderer", "react-flow", true);
		expect(current().renderer).toBe("egui");
		change("flow-board-renderer", "react-flow");
		expect(current().renderer).toBe("react-flow");
		change(null, null);
		expect(current().renderer).toBe("egui");
	});

	test("blocked storage keeps explicit choices available for this session", () => {
		Object.defineProperty(browser, "localStorage", {
			configurable: true,
			get: () => {
				throw new Error("Storage denied");
			},
		});
		const current = mount();
		expect(current().renderer).toBe("egui");
		act(() => current().setRenderer("react-flow"));
		expect(current().renderer).toBe("react-flow");
		expect(mount()().renderer).toBe("react-flow");
	});

	test("server rendering always uses the safe default, independent of client memory", () => {
		setBoardRendererPreference("egui");
		function ServerProbe() {
			const { renderer, ready } = useBoardRenderer();
			return createElement("span", null, `${renderer}:${ready}`);
		}
		expect(renderToString(createElement(ServerProbe))).toBe(
			"<span>react-flow:false</span>",
		);
		expect(mount()().renderer).toBe("egui");
	});
});
