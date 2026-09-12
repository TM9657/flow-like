import { afterAll, beforeAll, describe, expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act, createElement } from "react";

const window = new Window({ url: "https://localhost" });
Object.assign(window, { SyntaxError, TypeError, Error });

function installDomGlobals() {
	Object.assign(globalThis, {
		window,
		document: window.document,
		navigator: window.navigator,
		localStorage: window.localStorage,
		HTMLElement: window.HTMLElement,
		Element: window.Element,
		Node: window.Node,
		MutationObserver: window.MutationObserver,
		HTMLButtonElement: window.HTMLButtonElement,
		SVGElement: window.SVGElement,
		Event: window.Event,
		CustomEvent: window.CustomEvent,
		MouseEvent: window.MouseEvent,
		PointerEvent: window.PointerEvent,
		Blob: window.Blob,
		getComputedStyle: window.getComputedStyle.bind(window),
		requestAnimationFrame: (cb: FrameRequestCallback) =>
			setTimeout(() => cb(0), 0),
		cancelAnimationFrame: (id: number) => clearTimeout(id),
		ResizeObserver: class {
			observe() {}
			unobserve() {}
			disconnect() {}
		},
	});
}
installDomGlobals();

// @ts-expect-error — react-dom checks this flag before touching the DOM.
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

const translate = (_key: string, fallback: string) => fallback;
mock.module("@flow-like/locales", () => ({
	useTranslation: () => ({ t: translate }),
}));
mock.module("next-themes", () => ({
	useTheme: () => ({ resolvedTheme: "dark" }),
}));
mock.module("sonner", () => ({
	toast: { success: () => {}, error: () => {} },
}));

let storedSheet = "";
const setAppStylesheet = mock(async (_appId: string, css: string) => {
	storedSheet = css;
});
/** One stable object, the way the real backend context hands it out. */
const backend = {
	appState: {
		getAppStylesheet: async () => storedSheet,
		setAppStylesheet,
	},
};
mock.module("../../../state/backend-state", () => ({
	useBackend: () => backend,
}));

/** Monaco loads its editor from a CDN; the studio only needs a text surface here. */
mock.module("@monaco-editor/react", () => ({
	default: () => createElement("div"),
}));
mock.module("../../ui/monaco-code-editor", () => ({
	MonacoCodeEditor: ({
		value,
		onChange,
	}: {
		value: string;
		onChange: (next: string) => void;
	}) =>
		createElement("textarea", {
			"data-testid": "sheet",
			value,
			onChange: (event: { target: { value: string } }) =>
				onChange(event.target.value),
		}),
}));

mock.module("./appearance-rail", () => ({
	AppearanceRail: ({
		onChange,
		state,
	}: {
		onChange: (next: unknown) => void;
		state: { effects: Record<string, boolean> };
	}) =>
		createElement("button", {
			id: "appearance-fx-aurora",
			type: "button",
			onClick: () =>
				onChange({ ...state, effects: { ...state.effects, aurora: true } }),
		}),
}));

const { createRoot } = await import("react-dom/client");
const { AppearanceStudio } = await import("./appearance-studio");
const { APPEARANCE_END, APPEARANCE_START } = await import(
	"../../../lib/appearance/appearance-theme"
);

beforeAll(() => installDomGlobals());

const roots: ReturnType<typeof createRoot>[] = [];

async function render(): Promise<HTMLElement> {
	const container = window.document.createElement(
		"div",
	) as unknown as HTMLElement;
	window.document.body.appendChild(container as never);
	const root = createRoot(container);
	roots.push(root);
	act(() => {
		root.render(createElement(AppearanceStudio, { appId: "app-1" }));
	});
	// The sheet arrives from the backend asynchronously; flush until the editor is up.
	for (let pass = 0; pass < 25; pass += 1) {
		if (container.querySelector("[data-testid=sheet]")) break;
		await new Promise((resolve) => setTimeout(resolve, 0));
		act(() => {});
	}
	return container;
}

afterAll(() => {
	act(() => {
		for (const root of roots) root.unmount();
	});
});

const sheetOf = (container: HTMLElement) =>
	(container.querySelector("[data-testid=sheet]") as HTMLTextAreaElement).value;

describe("AppearanceStudio", () => {
	test("seeds a managed sheet and previews it on the app surfaces", async () => {
		storedSheet = "";
		const container = await render();

		const sheet = sheetOf(container);
		expect(sheet).toContain(APPEARANCE_START);
		expect(sheet).toContain(APPEARANCE_END);
		expect(sheet).toContain("--primary:");

		const preview = container.querySelector('[data-appearance-preview="1"]');
		expect(preview).not.toBeNull();
		expect(preview?.textContent).toContain("Operations");
		// The preview is styled by the sheet itself, scoped the way the runtime scopes it.
		expect(container.querySelector("style")?.textContent).toContain(
			'[data-appearance-preview="1"]',
		);
	});

	test("a control writes into the managed block", async () => {
		storedSheet = "";
		const container = await render();
		expect(sheetOf(container)).not.toContain("@fx aurora");

		const aurora = container.querySelector(
			"#appearance-fx-aurora",
		) as HTMLElement;
		expect(aurora).not.toBeNull();
		act(() => {
			aurora.click();
		});

		expect(sheetOf(container)).toContain("@fx aurora");
		expect(sheetOf(container)).toContain("@keyframes fl-appearance-aurora");
	});

	test("an existing sheet without markers is kept below the block", async () => {
		storedSheet = ".mine {\n  color: red;\n}";
		const container = await render();
		const sheet = sheetOf(container);
		expect(sheet.indexOf(".mine")).toBeGreaterThan(
			sheet.indexOf(APPEARANCE_END),
		);
	});
});
