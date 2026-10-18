import { afterEach, describe, expect, spyOn, test } from "bun:test";
import { Window } from "happy-dom";
import { act } from "react";
import { type Root, createRoot } from "react-dom/client";
import type { IIntercomEvent } from "../../lib/schema/events/intercom-event";
import type { ILogMetadata } from "../../lib/schema/flow/log-metadata";
import type { Action, ActionBinding, EventHandlers } from "./types";

let root: Root | undefined;
const cleanup: (() => void)[] = [];

afterEach(async () => {
	await act(() => root?.unmount());
	root = undefined;
	for (const restore of cleanup.reverse()) restore();
	cleanup.length = 0;
});

const pageAction = { actionId: "save-file", manifestRevision: "revision-1" };
const strict = { requireSuccess: true };
const workflow = (governed: boolean): Action => ({
	name: "workflow_event",
	context: { nodeId: "save-node" },
	...(governed ? { pageAction } : {}),
});
const widget: Action = { name: "widget_event", context: { actionId: "save" } };

interface HarnessOptions {
	governed?: boolean;
	running?: boolean;
	missingApp?: boolean;
	missingBoard?: boolean;
	missingEvent?: boolean;
	binding?: ActionBinding | null;
	eventHandlers?: EventHandlers;
	legacyActions?: Action[];
	interceptNavigation?: boolean;
}

async function setup(options: HarnessOptions = {}) {
	const window = new Window({ url: "https://local/use" });
	const globals = {
		window,
		document: window.document,
		navigator: window.navigator,
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
	const [
		{ ActionProvider, useExecuteAction, useComponentEventTrigger },
		{ WidgetInstanceProvider },
		{ useBackendStore },
		{ appGlobalState, pageLocalState },
		uiState,
		{ AppRouterContext },
		{ PathnameContext },
		{ toast },
	] = await Promise.all([
		import("./ActionHandler"),
		import("./layout/A2UIWidgetInstance"),
		import("../../state/backend-state"),
		import("../../lib/idb-storage"),
		import("../../db/ui-state-db"),
		import("next/dist/shared/lib/app-router-context.shared-runtime"),
		import("next/dist/shared/lib/hooks-client-context.shared-runtime"),
		import("sonner"),
	]);
	const spies = [
		spyOn(appGlobalState, "getAll").mockResolvedValue({}),
		spyOn(pageLocalState, "getAll").mockResolvedValue({}),
		spyOn(uiState.uiElementValues, "getAll").mockResolvedValue({}),
		spyOn(uiState, "pruneElementValues").mockResolvedValue(),
		spyOn(console, "log").mockImplementation(() => {}),
		spyOn(console, "warn").mockImplementation(() => {}),
		spyOn(console, "error").mockImplementation(() => {}),
		spyOn(toast, "error").mockImplementation(() => ""),
		spyOn(toast, "warning").mockImplementation(() => ""),
		spyOn(toast, "info").mockImplementation(() => ""),
	];
	cleanup.push(() => {
		for (const spy of spies) spy.mockRestore();
	});
	const previousBackend = useBackendStore.getState().backend;
	cleanup.push(() => useBackendStore.setState({ backend: previousBackend }));

	let outcome:
		| "ok"
		| "started"
		| "completed"
		| "success"
		| "succeeded"
		| "declined"
		| "metadata-error"
		| "fatal"
		| "stream-error"
		| "failed-completion"
		| "cancelled-completion"
		| "unknown-completion"
		| "terminal-log-error"
		| "navigate"
		| "reject" = "ok";
	const calls: string[] = [];
	const payloads: { id: string; payload: Record<string, unknown> }[] = [];
	const navigations: string[] = [];
	const interceptions: string[] = [];
	const run = async (
		kind: string,
		_appId: string,
		_targetId: string,
		payload: { id: string; payload: Record<string, unknown> },
		_stream: boolean,
		onStarted?: (id: string) => void,
		onEvents?: (events: IIntercomEvent[]) => void,
	) => {
		calls.push(kind);
		payloads.push(payload);
		if (outcome === "reject") throw new Error("Storage write refused.");
		if (outcome === "declined") return undefined;
		onStarted?.("run-save");
		if (outcome === "navigate") {
			onEvents?.([
				{
					event_type: "a2ui",
					payload: { type: "navigateTo", route: "/next", replace: false },
				} as IIntercomEvent,
			]);
		}
		if (outcome === "started") return undefined;
		if (
			outcome === "completed" ||
			outcome === "success" ||
			outcome === "succeeded"
		) {
			onEvents?.([
				{
					event_type:
						outcome === "success" ? "execution_complete" : "completed",
					payload: { status: ` ${outcome.toUpperCase()} ` },
				} as IIntercomEvent,
			]);
			return undefined;
		}
		if (outcome === "unknown-completion" || outcome === "terminal-log-error") {
			onEvents?.([
				{
					event_type: "completed",
					payload:
						outcome === "unknown-completion"
							? {}
							: { status: "completed", log_level: 3 },
				} as IIntercomEvent,
			]);
			return undefined;
		}
		if (outcome === "stream-error") {
			onEvents?.([
				{
					event_type: "error",
					payload: { message: "Upload failed during the run." },
				} as IIntercomEvent,
			]);
		}
		if (outcome === "failed-completion" || outcome === "cancelled-completion") {
			onEvents?.([
				{
					event_type:
						outcome === "failed-completion"
							? "completed"
							: "execution_complete",
					payload: {
						status: outcome === "failed-completion" ? "failed" : "cancelled",
					},
				} as IIntercomEvent,
			]);
			return undefined;
		}
		return {
			run_id: "run-save",
			log_level: outcome === "metadata-error" ? 3 : outcome === "fatal" ? 4 : 0,
		} as ILogMetadata;
	};
	useBackendStore.getState().setBackend({
		boardState: {
			getBoard: async () => ({ page_ids: [], nodes: {}, layers: {} }),
			executeBoard: run.bind(null, "board"),
		},
		eventState: {
			executeEvent: run.bind(null, "event"),
		},
	} as never);

	const controls: Record<
		string,
		ReturnType<typeof useExecuteAction> & {
			trigger: ReturnType<typeof useComponentEventTrigger>;
		}
	> = {};
	function Probe({ id }: { id: string }) {
		controls[id] = {
			...useExecuteAction(),
			trigger: useComponentEventTrigger("file"),
		};
		return null;
	}
	const governed = options.governed ?? false;
	const binding: ActionBinding | null =
		options.binding === undefined
			? {
					workflow: { flowId: "save-node", inputMappings: {} },
					...(governed ? { pageAction } : {}),
				}
			: options.binding;
	const host = window.document.createElement("div");
	window.document.body.appendChild(host);
	root = createRoot(host as unknown as HTMLElement);
	await act(async () => {
		root?.render(
			<AppRouterContext.Provider
				value={
					{
						push: (url: string) => navigations.push(url),
						replace: (url: string) => navigations.push(url),
					} as never
				}
			>
				<PathnameContext.Provider value="/use">
					<ActionProvider
						appId={options.missingApp ? undefined : "app-save"}
						boardId={options.missingBoard ? undefined : "board-save"}
						eventId={options.missingEvent ? undefined : "event-save"}
						surfaceId="page-save"
						governedPage={governed}
						isPreviewMode={options.running ?? true}
						components={{}}
						onNavigationMessage={
							options.interceptNavigation
								? (message) => {
										interceptions.push(message.type);
									}
								: undefined
						}
					>
						<Probe id="page" />
						<WidgetInstanceProvider
							instanceId="widget-save"
							widgetId="file-widget"
							componentId="widget-save"
							actionBindings={binding ? { save: binding } : {}}
							eventHandlers={options.eventHandlers}
							actions={options.legacyActions}
						>
							<Probe id="widget" />
						</WidgetInstanceProvider>
					</ActionProvider>
				</PathnameContext.Provider>
			</AppRouterContext.Provider>,
		);
	});
	return {
		controls,
		calls,
		payloads,
		navigations,
		interceptions,
		setOutcome: (value: typeof outcome) => {
			outcome = value;
		},
	};
}

describe("workflow acknowledgement required by file saves", () => {
	for (const governed of [false, true]) {
		test(`${governed ? "governed" : "direct"} component events carry their trusted semantic identity`, async () => {
			const harness = await setup({ governed });
			await act(async () => {
				await harness.controls.page.trigger(
					"save",
					{ eventHandlers: { save: [workflow(governed)] } },
					{
						actionId: "untrusted-payload-name",
						url: "https://files.example/edit",
					},
					strict,
				);
				await harness.controls.page.executeAction(
					workflow(governed),
					"file",
					{ actionId: "ordinary-page-field" },
					strict,
				);
			});
			expect(harness.payloads[0].payload).toMatchObject({
				_action_id: "save",
				_widget_instance_id: "",
				_triggering_component_id: "file",
				_action_context: {
					actionId: "untrusted-payload-name",
					url: "https://files.example/edit",
				},
			});
			expect(harness.payloads[1].payload._action_id).toBeUndefined();
		});
	}

	for (const route of ["binding", "named"] as const) {
		test(`component events retain the authored widget ${route} route and its event identity`, async () => {
			const harness = await setup({
				...(route === "named"
					? { eventHandlers: { save: [workflow(false)] } }
					: {}),
			});
			await act(async () => {
				await harness.controls.widget.trigger(
					"capture",
					{ eventHandlers: { capture: [widget] } },
					{ actionId: "capture", url: "https://files.example/edit" },
					strict,
				);
			});
			expect(harness.payloads).toHaveLength(1);
			expect(harness.payloads[0]).toMatchObject({
				id: "save-node",
				payload: {
					_action_id: "save",
					_widget_instance_id: "widget-save",
					_action_context: {
						actionId: "save",
						url: "https://files.example/edit",
					},
				},
			});
		});
	}

	for (const governed of [false, true]) {
		for (const probe of ["page", "widget"] as const) {
			test(`${governed ? "governed Event" : "direct Board"} ${probe} rejects unsuccessful runs and keeps legacy calls non-throwing`, async () => {
				const harness = await setup({ governed });
				const action = probe === "page" ? workflow(governed) : widget;
				await act(async () => {
					for (const outcome of [
						"ok",
						"completed",
						"success",
						"succeeded",
					] as const) {
						harness.setOutcome(outcome);
						await expect(
							harness.controls[probe].executeAction(action, "file", {}, strict),
						).resolves.toBeUndefined();
					}
					for (const [outcome, message] of [
						["declined", "did not start"],
						["started", "did not confirm completion"],
						["metadata-error", "workflow failed"],
						["fatal", "workflow failed"],
						["stream-error", "Upload failed during the run"],
						["failed-completion", "status: failed"],
						["cancelled-completion", "status: cancelled"],
						["unknown-completion", "status: unknown"],
						["terminal-log-error", "workflow failed"],
						["reject", "Storage write refused"],
					] as const) {
						harness.setOutcome(outcome);
						await expect(
							harness.controls[probe].executeAction(action, "file", {}, strict),
						).rejects.toThrow(message);
						await expect(
							harness.controls[probe].executeAction(action, "file"),
						).resolves.toBeUndefined();
					}
				});
				expect(new Set(harness.calls)).toEqual(
					new Set([governed ? "event" : "board"]),
				);
			});
		}
	}

	test("save handlers are required and stop after the first failed workflow", async () => {
		const harness = await setup();
		await act(async () => {
			await expect(
				harness.controls.page.trigger("save", {}, {}, strict),
			).rejects.toThrow("no configured workflow");
			await expect(
				harness.controls.page.trigger("save", {}),
			).resolves.toBeUndefined();
			harness.setOutcome("metadata-error");
			await expect(
				harness.controls.page.trigger(
					"save",
					{
						eventHandlers: { save: [workflow(false), workflow(false)] },
					},
					{},
					strict,
				),
			).rejects.toThrow("workflow failed");
		});
		expect(harness.calls).toEqual(["board"]);
	});

	test("a strict save rejects missing actions and non-workflow action names before dispatch", async () => {
		const harness = await setup();
		await act(async () => {
			await expect(
				harness.controls.page.executeAction(undefined, "file", {}, strict),
			).rejects.toThrow("No workflow");
			for (const name of [
				"navigate_page",
				"external_link",
				"submit_feedback",
				"custom_save",
			]) {
				await expect(
					harness.controls.page.executeAction(
						{ name, context: {} },
						"file",
						{},
						strict,
					),
				).rejects.toThrow("requires a workflow");
			}
		});
		expect(harness.calls).toEqual([]);
	});

	test.each([
		["builder mode", { running: false }, workflow(false), "page"],
		["missing app", { missingApp: true }, workflow(false), "page"],
		["missing board", { missingBoard: true }, widget, "widget"],
		["missing handler", {}, { name: "workflow_event", context: {} }, "page"],
		[
			"pending Page action",
			{ governed: true },
			{ name: "workflow_event", context: {}, pendingPageAction: true },
			"page",
		],
		["unauthorized Page action", { governed: true }, workflow(false), "page"],
		[
			"missing Page Event",
			{ governed: true, missingEvent: true },
			workflow(true),
			"page",
		],
		[
			"missing widget action id",
			{},
			{ name: "widget_event", context: {} },
			"widget",
		],
		["unbound widget action", { binding: null }, widget, "widget"],
		[
			"non-workflow widget binding",
			{ binding: { command: { commandName: "navigate", args: {} } } },
			widget,
			"widget",
		],
		[
			"non-workflow delegated action",
			{ eventHandlers: { save: [{ name: "external_link", context: {} }] } },
			widget,
			"widget",
		],
		[
			"disabled widget handler",
			{ eventHandlers: { save: [] } },
			widget,
			"widget",
		],
		[
			"pending widget action",
			{
				binding: {
					workflow: { flowId: "save-node", inputMappings: {} },
					pendingPageAction: true,
				},
			},
			widget,
			"widget",
		],
		[
			"unauthorized widget action",
			{
				governed: true,
				binding: { workflow: { flowId: "save-node", inputMappings: {} } },
			},
			widget,
			"widget",
		],
		[
			"missing widget Event",
			{ governed: true, missingEvent: true },
			widget,
			"widget",
		],
		[
			"missing widget handler",
			{ binding: { workflow: { flowId: "", inputMappings: {} } } },
			widget,
			"widget",
		],
	] as [string, HarnessOptions, Action, string][])(
		"%s rejects without executing",
		async (_label, options, action, probe) => {
			const harness = await setup(options);
			await act(async () => {
				await expect(
					harness.controls[probe].executeAction(action, "file", {}, strict),
				).rejects.toThrow();
			});
			expect(harness.calls).toEqual([]);
		},
	);

	for (const routing of ["named", "legacy"] as const) {
		test(`${routing} widget delegation preserves required success`, async () => {
			const harness = await setup({
				binding: null,
				...(routing === "named"
					? { eventHandlers: { save: [workflow(false)] } }
					: { legacyActions: [workflow(false)] }),
			});
			harness.setOutcome("declined");
			await act(async () => {
				await expect(
					harness.controls.widget.executeAction(widget, "file", {}, strict),
				).rejects.toThrow("did not start");
			});
			expect(harness.calls).toEqual(["board"]);
		});
	}
});

describe("unsaved document navigation protection", () => {
	test("other surfaces do not block navigation and unregistering releases the active surface", async () => {
		const harness = await setup();
		const { registerA2UINavigationGuard } = await import("./navigation-guard");
		cleanup.push(
			registerA2UINavigationGuard("another-app", "page-save", () => false),
			registerA2UINavigationGuard("app-save", "another-page", () => false),
		);
		const navigate: Action = {
			name: "navigate_page",
			context: { route: "/next" },
		};
		await act(async () => {
			await harness.controls.page.executeAction(navigate);
		});
		expect(harness.navigations).toHaveLength(1);
		const unregister = registerA2UINavigationGuard(
			"app-save",
			"page-save",
			() => false,
		);
		cleanup.push(unregister);
		await act(async () => {
			await harness.controls.page.executeAction(navigate);
		});
		expect(harness.navigations).toHaveLength(1);
		unregister();
		await act(async () => {
			await harness.controls.page.executeAction(navigate);
		});
		expect(harness.navigations).toHaveLength(2);
	});

	test.each([false, true])(
		"guards action and workflow navigation before intercepting (interceptor handles: %s)",
		async (interceptNavigation) => {
			const harness = await setup({ interceptNavigation });
			const navigationGuard = await import("./navigation-guard");
			const guard = spyOn(
				navigationGuard,
				"canLeaveA2UISurface",
			).mockReturnValue(false);
			cleanup.push(() => guard.mockRestore());
			const navigate: Action = {
				name: "navigate_page",
				context: { route: "/next" },
			};
			harness.setOutcome("navigate");
			await act(async () => {
				await harness.controls.page.executeAction(navigate);
				await harness.controls.page.executeAction(workflow(false));
			});
			expect(harness.navigations).toEqual([]);
			expect(harness.interceptions).toEqual([]);
			expect(guard.mock.calls).toEqual([
				["app-save", "page-save"],
				["app-save", "page-save"],
			]);
			guard.mockReturnValue(true);
			await act(async () => {
				await harness.controls.page.executeAction(navigate);
				await harness.controls.page.executeAction(workflow(false));
			});
			expect(harness.interceptions).toEqual(
				interceptNavigation ? ["navigateTo", "navigateTo"] : [],
			);
			expect(harness.navigations).toHaveLength(interceptNavigation ? 0 : 2);
		},
	);
});
