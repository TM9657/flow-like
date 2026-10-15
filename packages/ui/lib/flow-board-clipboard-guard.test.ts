import { beforeEach, describe, expect, test } from "bun:test";
import { Window } from "happy-dom";

function installDom() {
	const window = new Window();
	Object.assign(window, { SyntaxError, TypeError });
	Object.assign(globalThis, {
		document: window.document,
		Element: window.Element,
		HTMLElement: window.HTMLElement,
		HTMLInputElement: window.HTMLInputElement,
		HTMLSelectElement: window.HTMLSelectElement,
		HTMLTextAreaElement: window.HTMLTextAreaElement,
		navigator: window.navigator,
		window,
	});
	return window;
}

installDom();

const {
	shouldIgnoreBoardClipboardEvent,
	handleCopy,
	handlePaste,
	duplicateSelectionCommand,
} = await import("./flow-board-utils");

let window = globalThis.window as unknown as Window;

// happy-dom's element classes are structurally distinct from lib.dom's, so the DOM this test
// builds only satisfies the guard's `instanceof` checks through the globals installed above.
function focus(element: unknown) {
	(element as HTMLElement).focus();
	Object.defineProperty(window.document, "activeElement", {
		configurable: true,
		get: () => element,
	});
}

beforeEach(() => {
	window = installDom() as unknown as Window;
});

describe("shouldIgnoreBoardClipboardEvent", () => {
	test("ignores clipboard events raised inside a Monaco EditContext host", () => {
		const editor = window.document.createElement("div");
		editor.className = "monaco-editor";
		const host = window.document.createElement("div");
		host.className = "native-edit-context";
		host.setAttribute("tabindex", "0");
		editor.appendChild(host);
		window.document.body.appendChild(editor);
		focus(host);

		expect(
			shouldIgnoreBoardClipboardEvent({
				target: host,
			} as unknown as ClipboardEvent),
		).toBe(true);
	});

	test("ignores clipboard events on an element that owns an EditContext", () => {
		const host = window.document.createElement("div");
		Object.defineProperty(host, "editContext", {
			configurable: true,
			value: {},
		});
		window.document.body.appendChild(host);
		focus(host);

		expect(
			shouldIgnoreBoardClipboardEvent({
				target: host,
			} as unknown as ClipboardEvent),
		).toBe(true);
	});

	test("still handles clipboard events raised on the board canvas", () => {
		const canvas = window.document.createElement("div");
		canvas.className = "react-flow__pane";
		window.document.body.appendChild(canvas);
		focus(canvas);

		expect(
			shouldIgnoreBoardClipboardEvent({
				target: canvas,
			} as unknown as ClipboardEvent),
		).toBe(false);
	});
});

function clipboardFixture() {
	const node = {
		id: "node",
		name: "literal",
		friendly_name: "Literal",
		description: "",
		category: "test",
		coordinates: [80, 40, 0],
		pins: {},
	};
	const board = {
		nodes: { node },
		comments: {},
		layers: {},
		variables: {},
		refs: {},
	} as unknown as import("./schema/flow/board").IBoard;
	return {
		board,
		nodes: [{ id: node.id, selected: true, type: "flowNode", data: { node } }],
	};
}
describe("board clipboard actions", () => {
	test("a rejected graph paste cannot fall through into creating a text comment", async () => {
		const calls: string[] = [];
		await expect(
			handlePaste(
				{
					target: window.document.body,
					clipboardData: {
						getData: () => JSON.stringify({ nodes: [], comments: [] }),
					},
					preventDefault() {},
					stopPropagation() {},
				} as unknown as ClipboardEvent,
				{ x: 10, y: 20 },
				"board",
				async (command) => {
					calls.push(command.command_type);
					throw new Error("backend rejected paste");
				},
			),
		).rejects.toThrow("backend rejected paste");
		expect(calls).toEqual(["CopyPaste"]);
	});
	test("copy writes the native ClipboardEvent synchronously without requesting clipboard permission", async () => {
		const f = clipboardFixture();
		const values = new Map<string, string>();
		let prevented = false;
		const result = handleCopy(f.nodes, f.board, { x: 10, y: 20 }, {
			target: window.document.body,
			clipboardData: {
				setData: (type: string, value: string) => values.set(type, value),
			},
			preventDefault: () => {
				prevented = true;
			},
			stopPropagation: () => {},
		} as unknown as ClipboardEvent);
		expect(prevented).toBe(true);
		expect(JSON.parse(values.get("text/plain") ?? "{}").nodes[0].id).toBe(
			"node",
		);
		expect(await result).toBe(true);
	});
	test("failed asynchronous clipboard writes reject before cut can remove anything", async () => {
		const f = clipboardFixture();
		Object.defineProperty(window.navigator, "clipboard", {
			configurable: true,
			value: {
				writeText: async () => {
					throw new Error("permission denied");
				},
			},
		});
		await expect(handleCopy(f.nodes, f.board)).rejects.toThrow(
			"permission denied",
		);
		expect(f.board.nodes.node.id).toBe("node");
	});
	test("duplicate creates a graph-copy command with an explicit offset and original ids unchanged", () => {
		const f = clipboardFixture();
		const command = duplicateSelectionCommand(f.nodes, f.board);
		expect(command).toMatchObject({
			command_type: "CopyPaste",
			old_mouse: [0, 0, 0],
			offset: [24, 24, 0],
			new_nodes: [],
		});
		expect(command?.original_nodes?.[0].id).toBe("node");
		expect(f.board.nodes.node.coordinates).toEqual([80, 40, 0]);
	});
});
