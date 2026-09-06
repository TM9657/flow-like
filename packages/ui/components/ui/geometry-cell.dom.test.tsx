import { afterEach, expect, test } from "bun:test";
import { Window } from "happy-dom";
import { act } from "react";

let cleanup: (() => Promise<void>) | undefined;
afterEach(async () => {
	await cleanup?.();
	cleanup = undefined;
});

async function setup(onClick?: () => void) {
	const window = new Window();
	Object.assign(window, { SyntaxError, TypeError });
	Object.assign(globalThis, {
		document: window.document,
		Element: window.Element,
		Event: window.Event,
		CustomEvent: window.CustomEvent,
		HTMLElement: window.HTMLElement,
		HTMLInputElement: window.HTMLInputElement,
		HTMLButtonElement: window.HTMLButtonElement,
		KeyboardEvent: window.KeyboardEvent,
		MouseEvent: window.MouseEvent,
		Node: window.Node,
		NodeFilter: window.NodeFilter,
		navigator: window.navigator,
		window,
		Document: window.Document,
		DocumentFragment: window.DocumentFragment,
		Text: window.Text,
		MutationObserver: window.MutationObserver,
		ResizeObserver: window.ResizeObserver,
		getComputedStyle: window.getComputedStyle.bind(window),
		requestAnimationFrame: window.requestAnimationFrame.bind(window),
		cancelAnimationFrame: window.cancelAnimationFrame.bind(window),
		IS_REACT_ACT_ENVIRONMENT: true,
	});
	const { createRoot } = await import("react-dom/client");
	const { GeometryCell } = await import("./geometry-cell");
	const container = window.document.createElement("div");
	window.document.body.append(container);
	const root = createRoot(container as unknown as HTMLElement);
	cleanup = async () => {
		await act(async () => root.unmount());
		await window.happyDOM.close();
	};
	await act(async () =>
		root.render(
			<GeometryCell
				value={{ type: "Point", coordinates: [13, 52] }}
				metadata={{ "ARROW:extension:name": "geoarrow.wkb" }}
				onClick={onClick}
			/>,
		),
	);
	return { window, container };
}

test("closing a Geometry dialog returns keyboard focus to its accessible opener", async () => {
	const { window, container } = await setup();
	const trigger = container.querySelector("button");
	expect(trigger).not.toBeNull();
	expect(trigger?.getAttribute("aria-haspopup")).toBe("dialog");
	expect(trigger?.getAttribute("aria-expanded")).toBe("false");
	await act(async () => {
		trigger?.focus();
		trigger?.click();
	});
	const dialog = window.document.querySelector('[role="dialog"]');
	expect(dialog).not.toBeNull();
	expect(trigger?.getAttribute("aria-controls")).toBe(dialog?.id);
	expect(trigger?.getAttribute("aria-expanded")).toBe("true");
	await act(async () => {
		window.document.dispatchEvent(
			new window.KeyboardEvent("keydown", {
				key: "Escape",
				bubbles: true,
			}),
		);
		await new Promise((resolve) => setTimeout(resolve, 10));
	});
	await act(async () => {
		await new Promise((resolve) => setTimeout(resolve, 10));
	});
	expect(window.document.querySelector('[role="dialog"]')).toBeNull();
	expect(trigger?.getAttribute("aria-expanded")).toBe("false");
	expect(window.document.activeElement === trigger).toBe(true);
});

test("a host click handler retains ownership of Geometry inspection", async () => {
	let clicks = 0;
	const { window, container } = await setup(() => {
		clicks++;
	});
	const trigger = container.querySelector("button");
	await act(async () => trigger?.click());
	expect(clicks).toBe(1);
	expect(window.document.querySelector('[role="dialog"]')).toBeNull();
	expect(trigger?.hasAttribute("aria-haspopup")).toBe(false);
});
