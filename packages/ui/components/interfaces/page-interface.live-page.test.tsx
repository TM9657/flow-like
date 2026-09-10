import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { type ReactNode, act, useEffect } from "react";
import { type Root, createRoot } from "react-dom/client";
import type { IEvent } from "../../lib/schema/flow/event";
import type {
	IPage,
	IPageBootstrap,
} from "../../state/backend-state/page-state";

const noop = () => {};
const persistence = {
	getAll: async () => ({}),
	set: async () => {},
	clearPage: async () => {},
};
mock.module("../../lib/idb-storage", () => ({
	appGlobalState: persistence,
	pageLocalState: persistence,
}));
const translate = (_key: string, fallback: string) => fallback;
mock.module("@flow-like/locales", () => ({
	useTranslation: () => ({ t: translate }),
	i18n: { t: translate },
}));
const router = { push: noop, replace: noop };
mock.module("next/navigation", () => ({
	useRouter: () => router,
	usePathname: () => "/developer/flowpilot-e2e",
	useSearchParams: () => new URLSearchParams("host=ignored"),
}));
mock.module("react-oidc-context", () => ({ useAuth: () => null }));
mock.module("../../hooks/use-asset-source", () => ({
	useAssetSource: () => ({ src: undefined }),
}));
mock.module("../../lib/use-runtime-tailwind", () => ({
	useRuntimeTailwindStyles: noop,
}));
const executeEvent = mock(async () => undefined);
const backend = { eventState: { executeEvent } };
mock.module("../../state/backend-state", () => ({
	useBackend: () => backend,
	useSignedIn: () => false,
	useBackendReady: () => true,
}));
mock.module("../../state/execution-service-context", () => ({
	useExecutionServiceOptional: () => null,
}));
const childrenOnly = ({ children }: { children: ReactNode }) => children;
const dialogs = { openDialog: noop, closeDialog: noop };
mock.module("../a2ui/RouteDialogProvider", () => ({
	RouteDialogProvider: childrenOnly,
	useRouteDialog: () => dialogs,
	useRouteDialogSafe: () => dialogs,
}));
mock.module("../a2ui/layout/A2UIWidgetInstance", () => ({
	useWidgetInstance: () => undefined,
	resolveWidgetInstanceEventRoute: noop,
}));
const elementStorage = {
	storeElementValue: noop,
	restoreSurfaceValues: async () => ({}),
};
mock.module("../a2ui/hooks/use-element-storage", () => ({
	useElementStorage: () => elementStorage,
}));
mock.module("../a2ui/elements-request-handler", () => ({
	handleElementsRequestMessage: () => false,
}));
mock.module("../a2ui/widget-query-handler", () => ({
	handleWidgetQueryMessage: () => false,
}));
mock.module("../scoped-custom-css", () => ({ ScopedCustomCss: () => null }));
mock.module("./page-loading-skeleton", () => ({
	PageLoadingSkeleton: () => null,
}));

// Keep the fixture's renderers real while excluding unrelated maps, charts and media imports.
const { A2UIColumn } = await import("../a2ui/layout/Column");
const { A2UITextField } = await import("../a2ui/interactive/TextField");
const { A2UIButton } = await import("../a2ui/interactive/Button");
const { A2UIText } = await import("../a2ui/display/Text");
const renderers = {
	column: A2UIColumn,
	textField: A2UITextField,
	button: A2UIButton,
	text: A2UIText,
};
mock.module("../a2ui/ComponentRegistry", () => ({
	getComponentRenderer: (type: keyof typeof renderers) => renderers[type],
}));

const { PageInterface } = await import("./page-interface");
const { findLivePage } = await import("../a2ui/live-page-registry");
const {
	IntakeRuntimePageHost,
	mountIntakeRuntimePage,
	unmountIntakeRuntimePage,
	describeIntakeRuntimeMount,
} = await import(
	"../../../../apps/desktop/lib/flowpilot-e2e/intake-runtime-host"
);

let root: Root | undefined;
let window: Window | undefined;
let restoreGlobals: (() => void) | undefined;
afterEach(async () => {
	await act(async () => unmountIntakeRuntimePage());
	await act(() => root?.unmount());
	root = undefined;
	await window?.happyDOM.abort();
	window = undefined;
	restoreGlobals?.();
	restoreGlobals = undefined;
	executeEvent.mockClear();
});
afterAll(() => mock.restore());

const page: IPage = {
	id: "intake-page",
	boardId: "intake-board",
	name: "Intake",
	route: "/intake",
	layoutType: "freeform",
	content: [],
	createdAt: "2026-09-09",
	updatedAt: "2026-09-09",
	components: [
		{
			id: "root",
			component: {
				type: "column",
				children: {
					explicitList: ["summary_input", "submit_ticket", "queue_result"],
				},
			},
		},
		{
			id: "summary_input",
			component: {
				type: "textField",
				label: { literalString: "Summary" },
				value: { path: "/summary" },
			},
		},
		{
			id: "submit_ticket",
			component: {
				type: "button",
				label: { literalString: "Submit ticket" },
				eventHandlers: {
					click: [
						{
							name: "workflow_event",
							pageAction: {
								actionId: "pa1_submit",
								manifestRevision: "execution-v1",
							},
						},
					],
				},
			},
		},
		{
			id: "queue_result",
			component: {
				type: "text",
				content: { literalString: "Awaiting submission" },
			},
		},
	],
};

function createEnvironment() {
	window = new Window({ url: "https://example.test/developer/flowpilot-e2e" });
	// Bun does not populate this Happy DOM realm constructor used by selector parsing.
	Object.assign(window, { SyntaxError });
	const globals = {
		window,
		document: window.document,
		HTMLElement: window.HTMLElement,
		Node: window.Node,
		navigator: window.navigator,
		IS_REACT_ACT_ENVIRONMENT: true,
	};
	const previous = Object.fromEntries(
		Object.keys(globals).map((key) => [
			key,
			Object.getOwnPropertyDescriptor(globalThis, key),
		]),
	);
	Object.assign(globalThis, globals);
	restoreGlobals = () => {
		for (const [key, descriptor] of Object.entries(previous)) {
			if (descriptor) Object.defineProperty(globalThis, key, descriptor);
			else Reflect.deleteProperty(globalThis, key);
		}
	};
	const host = window.document.createElement("div");
	window.document.body.append(host);
	root = createRoot(host as unknown as HTMLElement);
	return host;
}

function bootstrap(executionRevision = "execution-v1"): IPageBootstrap {
	const event = {
		id: "page-event",
		board_id: page.boardId,
		default_page_id: page.id,
		active: true,
	} as IEvent;
	return {
		event,
		page,
		executionRevision,
		canonicalRoute: "/intake",
	};
}

async function mount(executionRevision: string | undefined) {
	const host = createEnvironment();
	await act(async () => {
		root?.render(
			<PageInterface
				appId="intake-app"
				event={bootstrap().event}
				page={page}
				pageExecutionRevision={executionRevision}
				route="/intake"
				queryParams={{}}
			/>,
		);
	});
	return host;
}

describe("PageInterface live Page integration", () => {
	test("mounts the real renderer and bridge with a connected input/button/result surface", async () => {
		const host = await mount("execution-v1");
		const handle = findLivePage("intake-app", {
			eventId: "page-event",
			pageId: page.id,
		});
		expect(handle).toBeDefined();
		expect(handle?.getContainer?.()?.isConnected).toBe(true);
		expect(handle?.getSurface()?.id).toBe(page.id);
		expect(host.querySelector("input")).not.toBeNull();
		expect(host.querySelector("button")?.textContent).toBe("Submit ticket");
		expect(
			host.querySelector('[data-a2ui-element-ref="intake-page/queue_result"]')
				?.textContent,
		).toBe("Awaiting submission");
		expect(executeEvent).not.toHaveBeenCalled();
		await act(() =>
			handle?.setElementValue("summary_input", "Service interruption"),
		);
		expect(handle?.getElementValues()["intake-page/summary_input"]).toBe(
			"Service interruption",
		);
		expect(host.querySelector("input")?.value).toBe("Service interruption");
		await act(() => root?.unmount());
		root = undefined;
		expect(
			findLivePage("intake-app", { eventId: "page-event", pageId: page.id }),
		).toBeUndefined();
	});

	test("does not register a governed Page before execution authorization exists", async () => {
		const host = await mount(undefined);
		expect(
			findLivePage("intake-app", { eventId: "page-event", pageId: page.id }),
		).toBeUndefined();
		expect(host.textContent).toContain(
			"could not load its execution authorization",
		);
	});
});

describe("persistent intake runtime host", () => {
	test("a delayed generation callback mounts into the replacement ancestor and survives another remount", async () => {
		const host = createEnvironment();
		let releaseGeneration: (value: IPageBootstrap) => void = () => {};
		const generated = new Promise<IPageBootstrap>((resolve) => {
			releaseGeneration = resolve;
		});
		let generationRun: Promise<void> | undefined;
		function GenerationAncestor() {
			useEffect(() => {
				generationRun ??= generated.then((result) =>
					mountIntakeRuntimePage("intake-app", result),
				);
			}, []);
			return <IntakeRuntimePageHost />;
		}
		await act(async () => {
			root?.render(<GenerationAncestor key="original" />);
		});
		expect(generationRun).toBeDefined();
		await act(async () => {
			root?.render(<GenerationAncestor key="replacement" />);
		});
		await act(async () => {
			releaseGeneration(bootstrap());
			await generationRun;
		});
		const target = { eventId: "page-event", pageId: page.id };
		const firstHandle = findLivePage("intake-app", target);
		expect(firstHandle?.getContainer?.()?.isConnected).toBe(true);
		expect(host.querySelector("button")?.textContent).toBe("Submit ticket");
		expect(describeIntakeRuntimeMount()).toMatchObject({
			requestedAppId: "intake-app",
			requestedPageId: page.id,
			hostConnected: true,
			connectedHostCount: 1,
			renderedElementCount: 4,
		});
		await act(async () => {
			root?.render(<GenerationAncestor key="second-replacement" />);
		});
		const replacementHandle = findLivePage("intake-app", target);
		expect(replacementHandle).not.toBe(firstHandle);
		expect(firstHandle?.getContainer?.()?.isConnected).not.toBe(true);
		expect(replacementHandle?.getContainer?.()?.isConnected).toBe(true);
		expect(describeIntakeRuntimeMount()).toMatchObject({
			hostConnected: true,
			connectedHostCount: 1,
		});
		await act(async () => unmountIntakeRuntimePage());
		expect(findLivePage("intake-app", target)).toBeUndefined();
		expect(host.querySelector("[data-flowpilot-intake-runtime]")).toBeNull();
		expect(describeIntakeRuntimeMount()).toMatchObject({
			hostConnected: false,
			connectedHostCount: 0,
			renderedElementCount: 0,
		});
	});

	test("waits for a requested Page to commit when no ancestor is mounted yet", async () => {
		createEnvironment();
		let committed = false;
		const pending = mountIntakeRuntimePage("intake-app", bootstrap()).then(
			() => {
				committed = true;
			},
		);
		await Promise.resolve();
		expect(committed).toBe(false);
		expect(describeIntakeRuntimeMount()).toMatchObject({
			requestedPageId: page.id,
			hostConnected: false,
		});
		await act(async () => {
			root?.render(<IntakeRuntimePageHost />);
		});
		await pending;
		expect(committed).toBe(true);
		expect(
			findLivePage("intake-app", { pageId: page.id })?.getContainer?.()
				?.isConnected,
		).toBe(true);
	});
});
