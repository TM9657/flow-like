// @vitest-environment happy-dom

import { act } from "react";
import { type Root, createRoot } from "react-dom/client";
import { toast } from "sonner";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { ApiResponseError } from "../../../lib/api-error";
import type { BoardCommand } from "../../../lib/schema/flow/copilot";
import type { IApplyFlowScriptResponse } from "../../../state/backend-state/board-state";
import { FlowScriptPanel } from "./flowscript-panel";

const { backend } = vi.hoisted(() => ({
	backend: {
		boardState: { getFlowScript: vi.fn() },
		userState: { lookupUser: vi.fn() },
	},
}));

vi.mock("@flow-like/locales", () => ({
	useTranslation: () => ({ t: translate }),
}));
vi.mock("../../../state/backend-state", () => ({
	useBackend: () => backend,
}));
vi.mock("../../../hooks/use-developer-mode", () => ({
	useDeveloperMode: () => ({ developerMode: false }),
}));
vi.mock("@monaco-editor/react", () => ({
	default: ({
		value,
		onChange,
	}: { value: string; onChange: (value: string) => void }) => (
		<textarea
			aria-label="FlowScript editor"
			value={value}
			onChange={(event) => onChange(event.currentTarget.value)}
		/>
	),
}));
vi.mock("../../ui", async () => ({
	...(await import("../../ui/alert-dialog")),
	...(await import("../../ui/badge")),
	...(await import("../../ui/button")),
	...(await import("../../ui/tooltip")),
}));
vi.mock("./flowscript-comment-widgets", () => ({
	FlowScriptCommentOverlay: () => null,
}));

function translate(key: string, fallback: string | Record<string, unknown>) {
	if (typeof fallback === "string") return fallback;
	const template =
		(fallback.count === 1
			? (fallback.defaultValue_one ?? fallback.defaultValue)
			: (fallback.defaultValue_other ?? fallback.defaultValue)) ?? key;
	return String(template).replace(/\{\{(\w+)\}\}/g, (_, name: string) =>
		String(fallback[name] ?? ""),
	);
}

let root: Root;
let container: HTMLDivElement;
const apply =
	vi.fn<
		(
			text: string,
			options?: { allowDeletions?: boolean },
		) => Promise<IApplyFlowScriptResponse | undefined>
	>();

beforeEach(async () => {
	vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
	backend.boardState.getFlowScript.mockReset().mockResolvedValue("original");
	apply.mockReset();
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
	await act(async () => {
		root.render(
			<FlowScriptPanel
				appId="app"
				boardId="board"
				onApplyFlowScript={apply}
				onClose={() => {}}
			/>,
		);
	});
	await edit("edited draft");
});

afterEach(async () => {
	vi.useRealTimers();
	await act(async () => root.unmount());
	container.remove();
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

function editor() {
	const element = container.querySelector("textarea");
	if (!element) throw new Error("FlowScript editor not found");
	return element;
}

async function edit(text: string) {
	await act(async () => {
		const element = editor();
		Object.getOwnPropertyDescriptor(
			HTMLTextAreaElement.prototype,
			"value",
		)?.set?.call(element, text);
		element.dispatchEvent(new Event("input", { bubbles: true }));
	});
}

function button(label: string) {
	const element = [...document.querySelectorAll("button")].find(
		(button) => button.textContent?.trim() === label,
	);
	if (!element) throw new Error(`Button not found: ${label}`);
	return element;
}

async function click(label: string) {
	await act(async () => button(label).click());
}

function reviewDialog() {
	const element = document.querySelector('[role="alertdialog"]');
	if (!element) throw new Error("Apply review dialog not found");
	return element;
}

function blockedApply(
	commands: Record<string, unknown>[],
): IApplyFlowScriptResponse {
	return {
		commands: [],
		board_commands: commands as unknown as BoardCommand[],
		diagnostics: [
			"FlowScript edit would delete or relocate existing board items",
		],
	};
}

test("a blocked apply separates deleted items from each moved item before confirmation", async () => {
	apply
		.mockResolvedValueOnce(
			blockedApply([
				{
					command_type: "RemoveNode",
					node_id: "delete-first",
					summary: "Remove Logger",
				},
				{
					command_type: "RemoveNode",
					node_id: "delete-second",
					summary: "Remove Logger",
				},
				{
					command_type: "MoveToLayer",
					ids: ["move-first", "move-second", "move-third"],
					target_layer: "destination-module",
					summary: "Move 3 nodes to their written module",
				},
			]),
		)
		.mockResolvedValueOnce({
			commands: [],
			board_commands: [],
			diagnostics: [],
		});

	await click("Apply to board");
	const dialog = reviewDialog();
	expect(dialog.textContent).toContain("Review deletions and moves");
	expect(dialog.textContent).toContain("Deletes (2)");
	expect(dialog.textContent).toContain("Moves (3)");
	const rows = [...dialog.querySelectorAll("li")];
	expect(rows).toHaveLength(5);
	for (const id of ["delete-first", "delete-second"]) {
		const row = rows.find((row) => row.textContent?.includes(id));
		expect(row?.textContent).toContain("Remove Logger");
	}
	for (const id of ["move-first", "move-second", "move-third"]) {
		const row = rows.find((row) => row.textContent?.includes(id));
		expect(row?.textContent).toContain("destination-module");
	}
	expect(apply).toHaveBeenCalledTimes(1);
	expect(apply).toHaveBeenLastCalledWith(
		"edited draft",
		expect.objectContaining({ allowDeletions: false }),
	);

	await click("Apply changes");
	expect(apply).toHaveBeenCalledTimes(2);
	expect(apply).toHaveBeenLastCalledWith(
		"edited draft",
		expect.objectContaining({ allowDeletions: true }),
	);
});

test("a move-only review shows zero deletes and cancellation keeps the draft", async () => {
	apply.mockResolvedValueOnce(
		blockedApply([
			{
				command_type: "MoveToLayer",
				ids: ["event-node", "logger-node"],
				target_layer: null,
			},
		]),
	);

	await click("Apply to board");
	const dialog = reviewDialog();
	expect(dialog.textContent).toContain("Deletes (0)");
	expect(dialog.textContent).toContain("Moves (2)");
	const rows = [...dialog.querySelectorAll("li")];
	expect(rows).toHaveLength(2);
	for (const row of rows) expect(row.textContent).toContain("Board root");
	expect(button("Apply with moves").disabled).toBe(false);

	await click("Keep everything");
	expect(document.querySelector('[role="alertdialog"]')).toBeNull();
	expect(apply).toHaveBeenCalledTimes(1);
	expect(apply).toHaveBeenLastCalledWith(
		"edited draft",
		expect.objectContaining({ allowDeletions: false }),
	);
	expect(editor().value).toBe("edited draft");
});

test("a deletion-only review shows zero moves and keeps the deletion action", async () => {
	apply.mockResolvedValueOnce(
		blockedApply([
			{
				command_type: "RemoveNode",
				node_id: "removed-node",
				summary: "Remove obsolete node",
			},
			{
				command_type: "DeleteVariable",
				variable_id: "removed-variable",
				summary: "Remove obsolete value",
			},
			{
				command_type: "RemoveLayer",
				layer_id: "removed-layer",
				summary: "Remove obsolete layer",
			},
			{
				command_type: "DeleteComment",
				comment_id: "removed-comment",
				summary: "Remove obsolete comment",
			},
		]),
	);

	await click("Apply to board");
	const dialog = reviewDialog();
	expect(dialog.textContent).toContain("Deletes (4)");
	expect(dialog.textContent).toContain("Moves (0)");
	expect(dialog.textContent).toContain("No items will be moved.");
	expect(dialog.querySelectorAll("li")).toHaveLength(4);
	for (const id of [
		"removed-node",
		"removed-variable",
		"removed-layer",
		"removed-comment",
	]) {
		expect(dialog.textContent).toContain(id);
	}
	expect(button("Apply with deletions").disabled).toBe(false);
});

test("a large move batch lists all 1678 items and counts items rather than commands", async () => {
	const ids = Array.from({ length: 1678 }, (_, index) => `moved-item-${index}`);
	apply.mockResolvedValueOnce(
		blockedApply([
			{
				command_type: "MoveToLayer",
				ids,
				target_layer: null,
				summary: "Move 1678 nodes to the board root",
			},
		]),
	);

	await click("Apply to board");
	const dialog = reviewDialog();
	expect(dialog.textContent).toContain("Deletes (0)");
	expect(dialog.textContent).toContain("Moves (1678)");
	const rows = [...dialog.querySelectorAll("li")];
	expect(rows).toHaveLength(ids.length);
	for (const [index, row] of rows.entries()) {
		expect(row.textContent).toContain(ids[index]);
	}
});

test("a failed apply after deletion confirmation keeps the draft and shows a persistent error", async () => {
	apply
		.mockResolvedValueOnce({
			commands: [],
			board_commands: [],
			corrections: [],
			diagnostics: ["FlowScript edit would delete 1116 items"],
		})
		.mockRejectedValueOnce(new TypeError("Failed to fetch"));
	await click("Apply to board");
	expect(document.querySelector('[role="alertdialog"]')?.textContent).toContain(
		"1116 items",
	);
	await click("Apply with deletions");
	expect(apply).toHaveBeenLastCalledWith(
		"edited draft",
		expect.objectContaining({ allowDeletions: true }),
	);
	expect(container.querySelector('[role="alert"]')?.textContent).toContain(
		"Failed to fetch",
	);
	expect(container.textContent).toContain("Apply failed. See details above.");
	expect(container.textContent).toContain("Your draft is preserved.");
	expect(container.textContent).not.toContain("nothing was applied");
	expect(editor().value).toBe("edited draft");
	expect(button("Apply to board").disabled).toBe(false);

	backend.boardState.getFlowScript.mockResolvedValueOnce("applied draft");
	apply.mockResolvedValueOnce({
		commands: [],
		board_commands: [],
		corrections: ["fixed"],
		diagnostics: [],
	});
	await click("Apply to board");
	expect(container.querySelector('[role="alert"]')).toBeNull();
	expect(container.textContent).toContain("In sync with board");
	expect(editor().value).toBe("applied draft");
});

test("a missing apply result is shown as a failure and reset clears it", async () => {
	apply.mockResolvedValueOnce(undefined);
	await click("Apply to board");
	expect(container.querySelector('[role="alert"]')?.textContent).toContain(
		"Apply returned no result.",
	);
	expect(editor().value).toBe("edited draft");
	await click("Reset");
	expect(container.querySelector('[role="alert"]')).toBeNull();
	expect(editor().value).toBe("original");
});

test("a long HTTP 400 apply error remains readable and copyable until dismissed", async () => {
	const error = new ApiResponseError({
		status: 400,
		code: "BAD_REQUEST",
		errorId: "apply-error-reference",
		message: [
			"Failed to apply FlowScript:",
			...Array.from(
				{ length: 100 },
				(_, index) =>
					`  Removing node-${index} leaves dependent-${index} unrun.`,
			),
		].join("\n"),
	});
	apply.mockRejectedValueOnce(error);
	const writeText = vi
		.spyOn(navigator.clipboard, "writeText")
		.mockResolvedValue(undefined);

	await click("Apply to board");
	expect(container.querySelector('[role="alert"] li')?.textContent).toBe(
		error.message,
	);
	expect(editor().value).toBe("edited draft");

	vi.useFakeTimers();
	await act(async () => vi.advanceTimersByTimeAsync(10_000));
	await edit("draft after failed apply");
	expect(container.querySelector('[role="alert"] li')?.textContent).toBe(
		error.message,
	);
	await click("Copy details");
	expect(writeText).toHaveBeenCalledExactlyOnceWith(error.message);
	expect(container.querySelector('[role="alert"]')).not.toBeNull();

	await act(async () => {
		container
			.querySelector<HTMLButtonElement>('[aria-label="Dismiss"]')
			?.click();
	});
	expect(container.querySelector('[role="alert"]')).toBeNull();
	expect(editor().value).toBe("draft after failed apply");
});

test("copy details includes every validation diagnostic", async () => {
	const diagnostics = [
		"Unknown node type\n  at Logger",
		"Missing input: value",
	];
	apply.mockResolvedValueOnce({
		commands: [],
		diagnostics,
	});
	const writeText = vi
		.spyOn(navigator.clipboard, "writeText")
		.mockResolvedValue(undefined);

	await click("Apply to board");
	await click("Copy details");
	expect(writeText).toHaveBeenCalledExactlyOnceWith(diagnostics.join("\n"));
	expect(container.querySelectorAll('[role="alert"] li')).toHaveLength(2);
});

test("clipboard failure keeps the apply error available for manual copying", async () => {
	apply.mockRejectedValueOnce(new Error("Apply request failed"));
	vi.spyOn(navigator.clipboard, "writeText").mockRejectedValueOnce(
		new Error("Clipboard access denied"),
	);
	const showError = vi.spyOn(toast, "error");

	await click("Apply to board");
	await click("Copy details");
	expect(showError).toHaveBeenCalledWith(
		"Couldn't copy details. Select and copy the text in the panel.",
	);
	expect(container.querySelector('[role="alert"] li')?.textContent).toBe(
		"Apply request failed",
	);
});
