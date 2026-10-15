import { describe, expect, test } from "bun:test";
import type { Action, ActionBinding } from "../types";
import {
	type WidgetInstanceContextValue,
	applyExposedProps,
	applyRuntimeChildUpdates,
	resolveWidgetInstanceEventRoute,
} from "./A2UIWidgetInstance";

const action = (name: string): Action => ({ name, context: {} });

const INHERITED_ACTION_IDS = ["constructor", "toString", "__proto__"];

const workflowBinding: ActionBinding = {
	workflow: { flowId: "flow-id", inputMappings: {} },
};

describe("applyExposedProps", () => {
	test("preserves array length updates", () => {
		const original = [{ id: "text", component: { rows: [1, 2, 3] } }];
		const updated = applyExposedProps(
			original,
			[
				{
					id: "length",
					targetComponentId: "text",
					propertyPath: "rows.length",
					propType: "Json",
				},
			],
			{ length: 1 },
		);
		expect(updated[0].component.rows).toEqual([1]);
		expect(original[0].component.rows).toEqual([1, 2, 3]);
	});

	test("keeps prototype-named paths local to the copied component", () => {
		const original = [{ id: "text", component: { type: "text" } }];
		for (const propertyPath of [
			"__proto__.widgetPolluted",
			"constructor.prototype.widgetPolluted",
			"data.__proto__.widgetPolluted",
		]) {
			const updated = applyExposedProps(
				original,
				[
					{
						id: "value",
						targetComponentId: "text",
						propertyPath,
						propType: "Json",
					},
				],
				{ value: "local" },
			);
			expect(Object.hasOwn(Object.prototype, "widgetPolluted")).toBe(false);
			expect(original).toEqual([{ id: "text", component: { type: "text" } }]);
			let local: unknown = updated[0].component;
			for (const key of propertyPath.split(".")) {
				expect(Object.hasOwn(local as object, key)).toBe(true);
				local = (local as Record<string, unknown>)[key];
			}
			expect(local).toBe("local");
		}
	});

	test("preserves normal nested values and own prototype-named JSON properties", () => {
		const original = [
			{
				id: "text",
				component: JSON.parse(
					'{"type":"text","data":{"__proto__":{"before":1},"rows":[{"before":2}]}}',
				),
			},
		];
		const updated = applyExposedProps(
			original,
			[
				{
					id: "first",
					targetComponentId: "text",
					propertyPath: "data.__proto__.after",
					propType: "Json",
				},
				{
					id: "second",
					targetComponentId: "text",
					propertyPath: "data.rows.0.after",
					propType: "Json",
				},
			],
			{ first: 3, second: 4 },
		);
		expect(updated[0].component.data.__proto__).toEqual({
			before: 1,
			after: 3,
		});
		expect(updated[0].component.data.rows).toEqual([{ before: 2, after: 4 }]);
		expect(original[0].component.data.rows).toEqual([{ before: 2 }]);
	});
});

const widgetInstance = (
	overrides: Partial<WidgetInstanceContextValue> = {},
): WidgetInstanceContextValue => ({
	instanceId: "instance-id",
	widgetId: "widget-id",
	actionBindings: {},
	...overrides,
});

describe("resolveWidgetInstanceEventRoute", () => {
	test("prefers exact named handlers and preserves their order", () => {
		const route = resolveWidgetInstanceEventRoute(
			widgetInstance({
				eventHandlers: {
					submit: [action("workflow_event"), action("navigate_page")],
					"*": [action("wildcard")],
				},
				actionBindings: { submit: workflowBinding },
				actions: [action("legacy")],
			}),
			"submit",
		);

		expect(route).toEqual({
			kind: "actions",
			actions: [action("workflow_event"), action("navigate_page")],
		});
	});

	test("uses wildcard handlers before classic bindings", () => {
		expect(
			resolveWidgetInstanceEventRoute(
				widgetInstance({
					eventHandlers: { "*": [action("external_link")] },
					actionBindings: { refresh: workflowBinding },
				}),
				"refresh",
			),
		).toEqual({ kind: "actions", actions: [action("external_link")] });
	});

	test("treats an explicit empty named handler as handled", () => {
		expect(
			resolveWidgetInstanceEventRoute(
				widgetInstance({
					eventHandlers: {
						submit: [],
						"*": [action("wildcard")],
					},
					actionBindings: { submit: workflowBinding },
					actions: [action("legacy")],
				}),
				"submit",
			),
		).toEqual({ kind: "actions", actions: [] });
	});

	test("keeps a classic binding ahead of the legacy component action", () => {
		expect(
			resolveWidgetInstanceEventRoute(
				widgetInstance({
					actionBindings: { submit: workflowBinding },
					actions: [action("legacy")],
				}),
				"submit",
			),
		).toEqual({ kind: "binding", binding: workflowBinding });
	});

	test("falls back to only the first legacy component action", () => {
		expect(
			resolveWidgetInstanceEventRoute(
				widgetInstance({
					actions: [action("legacy"), action("previously-inert")],
				}),
				"submit",
			),
		).toEqual({ kind: "actions", actions: [action("legacy")] });
	});

	test("resolves an unbound action to the unbound route", () => {
		expect(resolveWidgetInstanceEventRoute(widgetInstance(), "submit")).toEqual(
			{ kind: "unbound" },
		);
	});

	test("inherited binding names resolve to no binding", () => {
		for (const actionId of INHERITED_ACTION_IDS) {
			expect(
				resolveWidgetInstanceEventRoute(widgetInstance(), actionId),
			).toEqual({ kind: "unbound" });
			expect(
				resolveWidgetInstanceEventRoute(
					widgetInstance({ actions: [action("legacy")] }),
					actionId,
				),
			).toEqual({ kind: "actions", actions: [action("legacy")] });
		}
	});

	test("still resolves own bindings named like prototype members", () => {
		// JSON.parse keeps `__proto__` as an own key, as a persisted page would.
		const binding = JSON.stringify(workflowBinding);
		const actionBindings = JSON.parse(
			`{"constructor":${binding},"__proto__":${binding}}`,
		) as Record<string, ActionBinding>;

		for (const actionId of ["constructor", "__proto__"]) {
			expect(
				resolveWidgetInstanceEventRoute(
					widgetInstance({ actionBindings }),
					actionId,
				),
			).toEqual({ kind: "binding", binding: workflowBinding });
		}
	});
});

describe("applyRuntimeChildUpdates", () => {
	test("replays ordered updates against an externally resolved widget child", () => {
		const original = [
			{
				id: "shared-field",
				component: { type: "textField", text: "Before" },
			},
		];
		const updated = applyRuntimeChildUpdates(original, {
			"shared-field": [
				{ type: "setText", text: "After" },
				{ type: "setVisibility", visible: false },
			],
		});

		expect(updated).not.toBe(original);
		expect(original[0].component).toEqual({
			type: "textField",
			text: "Before",
		});
		expect(updated[0].component).toMatchObject({
			type: "textField",
			text: "After",
			hidden: { literalBool: true },
		});
	});

	test("prefers an exact child id and preserves slash-containing ids", () => {
		const updated = applyRuntimeChildUpdates(
			[
				{ id: "prefix-field", component: { type: "text", text: "Prefix" } },
				{ id: "field", component: { type: "text", text: "Exact" } },
				{
					id: "section/field",
					component: { type: "text", text: "Nested" },
				},
			],
			{
				field: [{ type: "setText", text: "Exact updated" }],
				"section/field": [{ type: "setText", text: "Nested updated" }],
			},
		);

		expect(updated[0].component?.text).toBe("Prefix");
		expect(updated[1].component?.text).toBe("Exact updated");
		expect(updated[2].component?.text).toBe("Nested updated");
	});
});
