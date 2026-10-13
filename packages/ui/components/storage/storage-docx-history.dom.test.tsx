import { act } from "react";
import { type Root, createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useStorageDocxHistory } from "./storage-docx-history";

const route = "/library/config/storage?id=app-a";
const nextState = {
	__NA: true,
	__PRIVATE_NEXTJS_INTERNALS_TREE: ["", { children: ["library", {}] }],
};

let root: Root;
let container: HTMLDivElement;
let api: ReturnType<typeof useStorageDocxHistory>;
let entries: { state: unknown; url: string }[];
let cursor: number;
let canClose: ReturnType<typeof vi.fn<() => boolean>>;
let onClose: ReturnType<typeof vi.fn<() => void>>;
let nativeReplace: History["replaceState"];

function visit(index: number) {
	cursor = index;
	const entry = entries[cursor];
	nativeReplace(entry.state, "", entry.url);
	window.dispatchEvent(new PopStateEvent("popstate", { state: entry.state }));
}

function Harness() {
	api = useStorageDocxHistory(canClose, onClose);
	return <div>Document editor</div>;
}

beforeEach(async () => {
	Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
	nativeReplace = window.history.replaceState.bind(window.history);
	entries = [
		{ state: { __NA: true }, url: "/library/config/flows?id=app-a" },
		{ state: structuredClone(nextState), url: route },
	];
	cursor = 1;
	nativeReplace(entries[cursor].state, "", route);
	vi.spyOn(window.history, "state", "get").mockImplementation(
		() => entries[cursor].state,
	);
	vi.spyOn(window.history, "replaceState").mockImplementation(
		(state, _title, url) => {
			entries[cursor] = {
				state: structuredClone(state),
				url: String(url ?? entries[cursor].url),
			};
			nativeReplace(state, "", url);
		},
	);
	vi.spyOn(window.history, "pushState").mockImplementation(
		(state, _title, url) => {
			entries.splice(cursor + 1);
			entries.push({
				state: structuredClone(state),
				url: String(url ?? entries[cursor].url),
			});
			cursor++;
			nativeReplace(state, "", url);
		},
	);
	vi.spyOn(window.history, "back").mockImplementation(() => {
		queueMicrotask(() => {
			if (cursor > 0) visit(cursor - 1);
		});
	});
	canClose = vi.fn().mockReturnValue(true);
	onClose = vi.fn();
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
	await act(async () => {
		root.render(<Harness />);
	});
});

afterEach(async () => {
	await act(async () => {
		root.unmount();
	});
	container.remove();
	vi.restoreAllMocks();
});

it("preserves Next state and keeps the document open when browser Back is declined", async () => {
	api.openHistory();
	expect(window.history.state).toMatchObject(nextState);
	expect(cursor).toBe(2);
	canClose.mockReturnValue(false);
	const routed = vi.fn();
	window.addEventListener("popstate", routed);
	await act(async () => {
		window.history.back();
	});
	expect(canClose).toHaveBeenCalledOnce();
	expect(onClose).not.toHaveBeenCalled();
	expect(routed).not.toHaveBeenCalled();
	expect(cursor).toBe(2);
	expect(window.location.pathname + window.location.search).toBe(route);
	expect(window.history.state).toMatchObject(nextState);
	window.removeEventListener("popstate", routed);
});

it("closes on confirmed browser Back, restores original state, and leaves the next Back available", async () => {
	api.openHistory();
	await act(async () => {
		window.history.back();
	});
	expect(onClose).toHaveBeenCalledOnce();
	expect(window.history.state).toEqual(nextState);
	expect(cursor).toBe(1);
	await act(async () => {
		window.history.back();
	});
	expect(window.location.pathname).toBe("/library/config/flows");
	expect(canClose).toHaveBeenCalledOnce();
});

it("consumes the modal entry before explicit close continues without prompting again", async () => {
	api.openHistory();
	let closed = false;
	await act(async () => {
		closed = await api.closeHistory();
	});
	expect(closed).toBe(true);
	expect(cursor).toBe(1);
	expect(window.history.state).toEqual(nextState);
	expect(canClose).not.toHaveBeenCalled();
	expect(onClose).not.toHaveBeenCalled();
	api.openHistory();
	expect(cursor).toBe(2);
	expect(entries).toHaveLength(3);
});

it("does not consume unrelated history traversals, even to the same URL", async () => {
	api.openHistory();
	entries[0] = { state: structuredClone(nextState), url: route };
	const routed = vi.fn();
	window.addEventListener("popstate", routed);
	visit(0);
	expect(canClose).not.toHaveBeenCalled();
	expect(onClose).not.toHaveBeenCalled();
	expect(routed).toHaveBeenCalledOnce();
	expect(window.history.state).toEqual(nextState);
	expect(await api.closeHistory()).toBe(true);
	expect(window.history.back).not.toHaveBeenCalled();
	window.removeEventListener("popstate", routed);
});

it("cleans up an owned entry on unmount without an extra Back during pending close", async () => {
	api.openHistory();
	const back = vi.mocked(window.history.back);
	back.mockImplementation(() => {});
	const closing = api.closeHistory();
	await act(async () => {
		root.unmount();
	});
	expect(await closing).toBe(false);
	expect(back).toHaveBeenCalledOnce();
	visit(1);
	expect(window.history.state).toEqual(nextState);
});
