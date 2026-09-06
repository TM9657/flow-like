import { afterEach, expect, test } from "bun:test";
import { Window } from "happy-dom";
import { type ReactNode, act } from "react";
import type {
	OntologyQueryLanguagePreference,
	OntologyQueryProposal,
	OntologyQueryStatusEvent,
} from "../../../lib/ontology-query";

let cleanup: (() => Promise<void>) | undefined;

afterEach(async () => {
	await cleanup?.();
	cleanup = undefined;
});

async function setup() {
	const window = new Window();
	Object.assign(window, { SyntaxError, TypeError });
	Object.assign(globalThis, {
		document: window.document,
		Element: window.Element,
		Event: window.Event,
		HTMLElement: window.HTMLElement,
		HTMLButtonElement: window.HTMLButtonElement,
		HTMLInputElement: window.HTMLInputElement,
		HTMLSelectElement: window.HTMLSelectElement,
		HTMLTextAreaElement: window.HTMLTextAreaElement,
		InputEvent: window.InputEvent,
		KeyboardEvent: window.KeyboardEvent,
		MouseEvent: window.MouseEvent,
		Node: window.Node,
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
	const container = window.document.createElement("div");
	window.document.body.append(container);
	const root = createRoot(container as unknown as HTMLElement);
	cleanup = async () => {
		await act(async () => root.unmount());
		await window.happyDOM.close();
	};
	return {
		container,
		window,
		render: (children: ReactNode) =>
			act(async () => {
				root.render(children);
			}),
	};
}

function setInputValue(window: Window, input: HTMLInputElement, value: string) {
	const valueSetter = Object.getOwnPropertyDescriptor(
		window.HTMLInputElement.prototype,
		"value",
	)?.set;
	if (!valueSetter) throw new Error("Input value setter is unavailable");
	valueSetter.call(input, value);
	input.dispatchEvent(
		new window.InputEvent("input", {
			bubbles: true,
			data: value,
			inputType: "insertText",
		}),
	);
	input.dispatchEvent(new window.Event("change", { bubbles: true }));
}

function setSelectValue(
	window: Window,
	select: HTMLSelectElement,
	value: string,
) {
	const valueSetter = Object.getOwnPropertyDescriptor(
		window.HTMLSelectElement.prototype,
		"value",
	)?.set;
	if (!valueSetter) throw new Error("Select value setter is unavailable");
	valueSetter.call(select, value);
	select.dispatchEvent(new window.Event("change", { bubbles: true }));
}

function buttonWithText(container: HTMLElement, label: string) {
	return [...container.querySelectorAll("button")].find(
		(button) => button.textContent?.includes(label),
	);
}

test("asks FlowPilot with the natural-language prompt and selected language", async () => {
	const { container, render, window } = await setup();
	const { GraphQueryPanel } = await import("./graph-query-panel");
	const calls: Array<[string, OntologyQueryLanguagePreference]> = [];

	await render(
		<GraphQueryPanel
			onRunCypher={() => {}}
			onAskFlowPilot={(prompt, language) => {
				calls.push([prompt, language]);
			}}
			results={null}
		/>,
	);

	const prompt = container.querySelector<HTMLInputElement>(
		'[data-testid="ontology-natural-language-query"]',
	);
	const preference = container.querySelector<HTMLSelectElement>("select");
	if (!prompt || !preference) {
		throw new Error("Natural-language query controls were not rendered");
	}
	await act(async () => {
		setInputValue(window, prompt, "  Show customers with overdue invoices  ");
		setSelectValue(window, preference, "sql");
	});

	const ask = buttonWithText(container, "Ask FlowPilot");
	if (!ask) {
		throw new Error(
			`Ask FlowPilot button was not rendered: ${JSON.stringify(
				[...container.querySelectorAll("button")].map(
					(button) => button.textContent,
				),
			)}`,
		);
	}
	expect(ask.disabled).toBe(false);
	await act(async () => ask.click());

	expect(calls).toEqual([["Show customers with overdue invoices", "sql"]]);
});

test("syncs a generated SQL proposal into the editor and runs it with bound params", async () => {
	const { container, render } = await setup();
	const { GraphQueryPanel } = await import("./graph-query-panel");
	const runs: OntologyQueryProposal[] = [];
	const proposal: OntologyQueryProposal = {
		language: "sql",
		query: "SELECT name FROM customer WHERE balance > $minimum",
		params: { minimum: 250 },
		presentation: "table",
	};

	await render(
		<GraphQueryPanel
			onRunCypher={() => {}}
			onRunQuery={(next) => {
				runs.push(next);
			}}
			generatedProposal={proposal}
			results={null}
		/>,
	);

	const query = container.querySelector<HTMLTextAreaElement>("textarea");
	const language = container.querySelector<HTMLSelectElement>("select");
	expect(query?.value).toBe(proposal.query);
	expect(language?.value).toBe("sql");
	expect(container.textContent).toContain('Bound parameters: {"minimum":250}');

	const run = buttonWithText(container, "Run");
	if (!run) throw new Error("Run button was not rendered");
	expect(run.disabled).toBe(false);
	await act(async () => run.click());

	expect(runs).toEqual([proposal]);
});

test("shows FlowPilot progress and lets the user cancel the active request", async () => {
	const { container, render } = await setup();
	const { GraphQueryPanel } = await import("./graph-query-panel");
	let cancellations = 0;
	const status: OntologyQueryStatusEvent = {
		requestId: "query-1",
		phase: "generating",
		attempt: 2,
	};

	await render(
		<GraphQueryPanel
			onRunCypher={() => {}}
			onAskFlowPilot={() => {}}
			onCancelFlowPilot={() => {
				cancellations += 1;
			}}
			flowPilotStatus={status}
			results={null}
		/>,
	);

	const panel = container.firstElementChild;
	const prompt = container.querySelector<HTMLInputElement>(
		'[data-testid="ontology-natural-language-query"]',
	);
	expect(panel?.getAttribute("aria-busy")).toBe("true");
	expect(prompt?.disabled).toBe(true);
	expect(container.textContent).toContain(
		"FlowPilot is writing the query... Retrying once.",
	);

	const stop = buttonWithText(container, "Stop");
	if (!stop) throw new Error("Stop button was not rendered");
	await act(async () => stop.click());
	expect(cancellations).toBe(1);
});
