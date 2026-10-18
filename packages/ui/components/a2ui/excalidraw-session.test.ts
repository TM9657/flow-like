import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ExcalidrawScene } from "./excalidraw-events";
import {
	type ExcalidrawSaveOptions,
	ExcalidrawSession,
	excalidrawContentKey,
	parseExcalidrawScene,
} from "./excalidraw-session";

function scene(revision = 0): ExcalidrawScene {
	return {
		type: "excalidraw",
		version: 2,
		source: "flow-like",
		elements: revision
			? [{ id: "rectangle", version: revision, x: revision }]
			: [],
		appState: { viewBackgroundColor: "#ffffff" },
		files: {},
	};
}

function pendingSave() {
	let resolve!: () => void;
	let reject!: (error: unknown) => void;
	const promise = new Promise<void>((resolvePromise, rejectPromise) => {
		resolve = resolvePromise;
		reject = rejectPromise;
	});
	return { promise, resolve, reject };
}

function setup(options: Partial<ExcalidrawSaveOptions> = {}) {
	const callbacks = {
		onDirty: vi.fn<(dirty: boolean) => void>(),
		onBusy: vi.fn<(busy: boolean) => void>(),
		onSave: vi
			.fn<
				(
					event: "save" | "autoSave",
					value: ExcalidrawScene,
					changeCount: number,
				) => Promise<void>
			>()
			.mockResolvedValue(undefined),
		onError: vi.fn<(event: "save" | "autoSave", error: unknown) => void>(),
	};
	const configuration: ExcalidrawSaveOptions = {
		readOnly: false,
		autoSave: true,
		autoSaveDelayMs: 1500,
		autoSaveMinChanges: 1,
		...options,
	};
	const session = new ExcalidrawSession(callbacks);
	session.configure(configuration);
	session.reset(scene());
	callbacks.onBusy.mockClear();
	return { session, configuration, ...callbacks };
}

beforeEach(() => {
	vi.useFakeTimers();
	vi.setSystemTime(new Date("2026-10-18T10:00:00Z"));
});

afterEach(() => {
	vi.clearAllTimers();
	vi.useRealTimers();
});

describe("Excalidraw idle autosave", () => {
	it("waits for an idle interval after the latest content change", async () => {
		const { session, onSave, onDirty, onBusy } = setup();
		session.change(scene(1));
		await vi.advanceTimersByTimeAsync(1000);
		session.change(scene(2));
		await vi.advanceTimersByTimeAsync(1499);
		expect(onSave).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(1);
		expect(onSave).toHaveBeenCalledExactlyOnceWith("autoSave", scene(2), 2);
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
		expect(onBusy.mock.calls).toEqual([[true], [false]]);
		await vi.advanceTimersByTimeAsync(30_000);
		expect(onSave).toHaveBeenCalledTimes(1);
	});

	it("waits for the minimum change count and resets that count after saving", async () => {
		const { session, onSave } = setup({ autoSaveMinChanges: 2 });
		session.change(scene(1));
		await vi.advanceTimersByTimeAsync(10_000);
		expect(onSave).not.toHaveBeenCalled();
		session.change(scene(2));
		await vi.advanceTimersByTimeAsync(1500);
		expect(onSave).toHaveBeenLastCalledWith("autoSave", scene(2), 2);
		session.change(scene(3));
		await vi.advanceTimersByTimeAsync(10_000);
		expect(onSave).toHaveBeenCalledTimes(1);
		session.change(scene(4));
		await vi.advanceTimersByTimeAsync(1500);
		expect(onSave).toHaveBeenLastCalledWith("autoSave", scene(4), 2);
	});

	it("ignores selection, viewport, theme, and tool changes without delaying a pending save", async () => {
		const { session, onSave, onDirty } = setup();
		const drawing = scene(1);
		session.change(drawing);
		await vi.advanceTimersByTimeAsync(1000);
		const viewportChange = {
			...drawing,
			appState: {
				...drawing.appState,
				selectedElementIds: { rectangle: true },
				scrollX: 200,
				scrollY: 100,
				zoom: { value: 2 },
				theme: "dark",
				activeTool: { type: "selection" },
			},
		};
		expect(session.change(viewportChange)).toBe(false);
		await vi.advanceTimersByTimeAsync(500);
		expect(onSave).toHaveBeenCalledExactlyOnceWith("autoSave", drawing, 1);
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
	});

	it("does not save an initial scene, identical changes, or content undone to its saved state", async () => {
		const { session, onSave, onDirty } = setup();
		await session.save("autoSave");
		expect(session.change(scene())).toBe(false);
		session.change(scene(1));
		await vi.advanceTimersByTimeAsync(1000);
		session.change(scene());
		await vi.advanceTimersByTimeAsync(10_000);
		expect(onSave).not.toHaveBeenCalled();
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
	});

	it("undoing an existing element to saved content clears dirty despite new internal revisions", async () => {
		const { session, onSave, onDirty } = setup();
		const saved = {
			...scene(),
			elements: [
				{
					id: "rectangle",
					type: "rectangle",
					x: 40,
					version: 2,
					versionNonce: 10,
					updated: 100,
				},
			],
		};
		session.reset(saved);
		session.change({
			...saved,
			elements: [
				{
					...saved.elements[0],
					x: 60,
					version: 3,
					versionNonce: 11,
					updated: 200,
				},
			],
		});
		session.change({
			...saved,
			elements: [
				{ ...saved.elements[0], version: 4, versionNonce: 12, updated: 300 },
			],
		});
		await vi.advanceTimersByTimeAsync(5000);
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
		expect(onSave).not.toHaveBeenCalled();
		const edited = {
			...saved,
			elements: [
				{
					...saved.elements[0],
					x: 80,
					version: 5,
					versionNonce: 13,
					updated: 400,
				},
			],
		};
		session.change(edited);
		await session.save();
		expect(onSave).toHaveBeenCalledWith("save", edited, 3);
	});

	it("cancels a pending autosave when disabled while allowing an explicit save", async () => {
		const { session, configuration, onSave } = setup();
		session.change(scene(1));
		session.configure({ ...configuration, autoSave: false });
		await vi.advanceTimersByTimeAsync(10_000);
		await session.save("autoSave");
		expect(onSave).not.toHaveBeenCalled();
		await session.save();
		expect(onSave).toHaveBeenCalledExactlyOnceWith("save", scene(1), 1);
	});

	it("read-only mode cancels autosave, rejects edits and manual saves, and preserves the draft", async () => {
		const { session, configuration, onSave } = setup();
		session.change(scene(1));
		session.configure({ ...configuration, readOnly: true });
		expect(session.change(scene(2))).toBe(false);
		await session.save();
		await vi.advanceTimersByTimeAsync(10_000);
		expect(onSave).not.toHaveBeenCalled();
		session.configure(configuration);
		await vi.advanceTimersByTimeAsync(0);
		expect(onSave).toHaveBeenCalledExactlyOnceWith("autoSave", scene(1), 1);
	});

	it("manual saving cancels the pending autosave timer", async () => {
		const { session, onSave } = setup();
		session.change(scene(1));
		await vi.advanceTimersByTimeAsync(500);
		await session.save();
		await vi.advanceTimersByTimeAsync(10_000);
		expect(onSave).toHaveBeenCalledExactlyOnceWith("save", scene(1), 1);
	});

	it("keeps failures dirty without automatic retries and permits an explicit retry", async () => {
		const { session, onSave, onError, onDirty, onBusy } = setup();
		const failure = new Error("Storage unavailable");
		onSave.mockRejectedValueOnce(failure);
		session.change(scene(1));
		await vi.advanceTimersByTimeAsync(1500);
		expect(onError).toHaveBeenCalledExactlyOnceWith("autoSave", failure);
		expect(onDirty.mock.calls).toEqual([[true]]);
		expect(onBusy.mock.calls).toEqual([[true], [false]]);
		await vi.advanceTimersByTimeAsync(60_000);
		expect(onSave).toHaveBeenCalledTimes(1);
		await session.save();
		expect(onSave).toHaveBeenLastCalledWith("save", scene(1), 1);
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
	});

	it("a later content edit retries a failed snapshot after a new idle interval", async () => {
		const { session, onSave } = setup();
		onSave.mockRejectedValueOnce(new Error("Try later"));
		session.change(scene(1));
		await vi.advanceTimersByTimeAsync(1500);
		session.change(scene(2));
		await vi.advanceTimersByTimeAsync(1499);
		expect(onSave).toHaveBeenCalledTimes(1);
		await vi.advanceTimersByTimeAsync(1);
		expect(onSave).toHaveBeenLastCalledWith("autoSave", scene(2), 2);
	});
});

describe("Excalidraw save concurrency and document lifetime", () => {
	it("keeps edits made during a save dirty and waits for their remaining idle interval", async () => {
		const { session, onSave, onDirty } = setup();
		const pending = pendingSave();
		onSave.mockReturnValueOnce(pending.promise);
		session.change(scene(1));
		const save = session.save();
		await vi.advanceTimersByTimeAsync(500);
		session.change(scene(2));
		await session.save();
		expect(onSave).toHaveBeenCalledTimes(1);
		await vi.advanceTimersByTimeAsync(500);
		pending.resolve();
		await save;
		expect(onDirty.mock.calls).toEqual([[true]]);
		await vi.advanceTimersByTimeAsync(999);
		expect(onSave).toHaveBeenCalledTimes(1);
		await vi.advanceTimersByTimeAsync(1);
		expect(onSave).toHaveBeenLastCalledWith("autoSave", scene(2), 1);
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
	});

	it("saves queued content as soon as an occupied save finishes if the idle interval already elapsed", async () => {
		const { session, onSave } = setup();
		const pending = pendingSave();
		onSave.mockReturnValueOnce(pending.promise);
		session.change(scene(1));
		const save = session.save();
		session.change(scene(2));
		await vi.advanceTimersByTimeAsync(5000);
		expect(onSave).toHaveBeenCalledTimes(1);
		pending.resolve();
		await save;
		await vi.advanceTimersByTimeAsync(0);
		expect(onSave).toHaveBeenLastCalledWith("autoSave", scene(2), 1);
	});

	it("retries the newest content when the occupied save fails, without losing its earlier changes", async () => {
		const { session, onSave, onError, onDirty } = setup();
		const pending = pendingSave();
		onSave.mockReturnValueOnce(pending.promise);
		session.change(scene(1));
		const save = session.save();
		session.change(scene(2));
		await vi.advanceTimersByTimeAsync(1500);
		const failure = new Error("Snapshot failed");
		pending.reject(failure);
		await save;
		expect(onError).toHaveBeenCalledExactlyOnceWith("save", failure);
		expect(onDirty.mock.calls).toEqual([[true]]);
		await vi.advanceTimersByTimeAsync(0);
		expect(onSave).toHaveBeenLastCalledWith("autoSave", scene(2), 2);
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
	});

	it("replacing a scene cancels pending autosave and establishes a new saved baseline", async () => {
		const { session, onSave, onDirty } = setup();
		session.change(scene(1));
		await vi.advanceTimersByTimeAsync(1000);
		session.reset(scene(10));
		await vi.advanceTimersByTimeAsync(5000);
		expect(onSave).not.toHaveBeenCalled();
		expect(onDirty.mock.calls).toEqual([[true], [false]]);
		session.change(scene(11));
		await vi.advanceTimersByTimeAsync(1500);
		expect(onSave).toHaveBeenCalledExactlyOnceWith("autoSave", scene(11), 1);
	});

	it.each(["resolve", "reject"] as const)(
		"ignores obsolete save %s after replacing the scene without clearing the new draft",
		async (outcome) => {
			const { session, onSave, onDirty, onBusy, onError } = setup();
			const previous = pendingSave();
			const current = pendingSave();
			onSave
				.mockReturnValueOnce(previous.promise)
				.mockReturnValueOnce(current.promise);
			session.change(scene(1));
			const previousSave = session.save();
			session.reset(scene(10));
			session.change(scene(11));
			const currentSave = session.save();
			onDirty.mockClear();
			onBusy.mockClear();
			if (outcome === "resolve") previous.resolve();
			else previous.reject(new Error("Previous document failed"));
			await previousSave;
			expect(onError).not.toHaveBeenCalled();
			expect(onDirty).not.toHaveBeenCalled();
			expect(onBusy).not.toHaveBeenCalled();
			await session.save();
			expect(onSave).toHaveBeenCalledTimes(2);
			current.resolve();
			await currentSave;
			expect(onDirty).toHaveBeenCalledExactlyOnceWith(false);
			expect(onBusy).toHaveBeenCalledExactlyOnceWith(false);
		},
	);

	it("disposal cancels pending autosave and prevents later edits or manual saves", async () => {
		const { session, onSave } = setup();
		session.change(scene(1));
		session.dispose();
		await vi.advanceTimersByTimeAsync(10_000);
		expect(session.change(scene(2))).toBe(false);
		await session.save();
		expect(onSave).not.toHaveBeenCalled();
	});

	it.each(["resolve", "reject"] as const)(
		"ignores an in-flight save %s after disposal",
		async (outcome) => {
			const { session, onSave, onDirty, onBusy, onError } = setup();
			const pending = pendingSave();
			onSave.mockReturnValueOnce(pending.promise);
			session.change(scene(1));
			const save = session.save();
			session.change(scene(2));
			session.dispose();
			onDirty.mockClear();
			onBusy.mockClear();
			if (outcome === "resolve") pending.resolve();
			else pending.reject(new Error("View closed"));
			await save;
			await vi.advanceTimersByTimeAsync(10_000);
			expect(onError).not.toHaveBeenCalled();
			expect(onDirty).not.toHaveBeenCalled();
			expect(onBusy).not.toHaveBeenCalled();
			expect(onSave).toHaveBeenCalledTimes(1);
		},
	);
});

describe("Excalidraw scene representation", () => {
	it("loads scene objects or JSON strings with images and supplies missing envelope fields", () => {
		const representation = {
			elements: [{ id: "image", fileId: "embedded-file" }],
			appState: { viewBackgroundColor: "#abcdef" },
			files: { "embedded-file": { dataURL: "data:image/png;base64,AA==" } },
		};
		for (const input of [representation, JSON.stringify(representation)]) {
			expect(parseExcalidrawScene(input)).toEqual({
				type: "excalidraw",
				version: 2,
				source: "flow-like",
				...representation,
			});
		}
	});

	it.each([
		"invalid JSON",
		[],
		{},
		{ elements: [null] },
		{ elements: [], type: "other" },
		{ elements: [], appState: [] },
		{ elements: [], files: { image: "invalid" } },
	])("rejects a malformed scene: %j", (input) => {
		expect(() => parseExcalidrawScene(input)).toThrow();
	});

	it("treats canvas background, grid settings, and embedded image updates as content changes", () => {
		const original = scene(1);
		for (const changes of [
			{ appState: { ...original.appState, viewBackgroundColor: "#000000" } },
			{ appState: { ...original.appState, gridModeEnabled: true } },
			{ appState: { ...original.appState, gridSize: 40, gridStep: 10 } },
			{ files: { image: { dataURL: "data:image/png;base64,AA==" } } },
		]) {
			expect(excalidrawContentKey({ ...original, ...changes })).not.toBe(
				excalidrawContentKey(original),
			);
		}
	});
});
