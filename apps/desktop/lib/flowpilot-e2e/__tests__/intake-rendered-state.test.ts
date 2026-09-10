import type { Surface } from "@flow-like/flow-like-ui/components/a2ui/types";
import { Window } from "happy-dom";
import { afterEach, describe, expect, test } from "vitest";
import { readIntakeRenderedQueue } from "../intake-rendered-state";

const windows: Window[] = [];
afterEach(async () => {
	for (const window of windows.splice(0)) await window.happyDOM.abort();
});

function fixture() {
	const window = new Window();
	windows.push(window);
	const container = window.document.createElement("div");
	const element = window.document.createElement("span");
	element.setAttribute("data-a2ui-element-ref", "page/queue_result");
	element.textContent = "Assigned queue: cobalt-response";
	element.getClientRects = () =>
		[{ width: 220, height: 24 }] as unknown as ReturnType<
			typeof element.getClientRects
		>;
	container.appendChild(element);
	window.document.body.appendChild(container);
	const surface: Surface = {
		id: "page",
		rootComponentId: "root",
		components: {
			root: {
				id: "root",
				component: {
					id: "root",
					type: "column",
					children: { explicitList: ["queue_result"] },
				},
			},
			queue_result: {
				id: "queue_result",
				component: {
					id: "queue_result",
					type: "text",
					content: { literalString: "stale semantic result" },
				},
			},
		},
	};
	const handle = {
		pageId: "page",
		getSurface: () => surface,
		getContainer: () => container as unknown as HTMLElement,
		resolveBoundValue: (value: unknown) => {
			if (value && typeof value === "object" && "literalBool" in value)
				return value.literalBool;
			return value;
		},
	};
	return { window, container, element, surface, handle };
}

describe("intake rendered queue evidence", () => {
	test("reads the actual visible prefixed text instead of semantic surface values", () => {
		const { handle } = fixture();
		expect(readIntakeRenderedQueue(handle)).toBe(
			"Assigned queue: cobalt-response",
		);
	});

	test("does not substitute hidden textContent when rendered innerText is empty", () => {
		const { element, handle } = fixture();
		Object.defineProperty(element, "innerText", { value: "" });
		expect(element.textContent).toContain("cobalt-response");
		expect(readIntakeRenderedQueue(handle)).toBe("");
	});

	test("rejects a result that is absent from the root tree even if matching DOM exists", () => {
		const { surface, handle } = fixture();
		surface.components.root.component.children = { explicitList: [] };
		expect(() => readIntakeRenderedQueue(handle)).toThrow("unreachable");
	});

	test("rejects a hidden result or hidden surface ancestor", () => {
		for (const id of ["queue_result", "root"]) {
			const { surface, handle } = fixture();
			surface.components[id].component.hidden = { literalBool: true };
			expect(() => readIntakeRenderedQueue(handle)).toThrow("hidden");
		}
	});

	test("rejects missing, detached, duplicate and another Page's DOM elements", () => {
		for (const mutate of [
			({ element }: ReturnType<typeof fixture>) => element.remove(),
			({ container }: ReturnType<typeof fixture>) => container.remove(),
			({ container, element }: ReturnType<typeof fixture>) =>
				container.appendChild(element.cloneNode(true)),
			({ element }: ReturnType<typeof fixture>) =>
				element.setAttribute("data-a2ui-element-ref", "other/queue_result"),
			({ window, element }: ReturnType<typeof fixture>) =>
				window.document.body.appendChild(element),
			({ surface }: ReturnType<typeof fixture>) => {
				surface.id = "other";
			},
		]) {
			const context = fixture();
			mutate(context);
			expect(() => readIntakeRenderedQueue(context.handle)).toThrow();
		}
	});

	test("rejects DOM hidden by CSS or lacking layout geometry", () => {
		for (const css of [
			"display: none",
			"visibility: hidden",
			"opacity: 0",
			"content-visibility: hidden",
		]) {
			const { container, handle } = fixture();
			container.setAttribute("style", css);
			expect(() => readIntakeRenderedQueue(handle)).toThrow("hidden");
		}
		const { element, handle } = fixture();
		element.getClientRects = () =>
			[] as unknown as ReturnType<typeof element.getClientRects>;
		expect(() => readIntakeRenderedQueue(handle)).toThrow("geometry");
	});
});
