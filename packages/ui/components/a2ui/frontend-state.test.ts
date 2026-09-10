import { describe, expect, spyOn, test } from "bun:test";
import {
	EMPTY_FRONTEND_STATE_RECORD,
	type FrontendStatePersistence,
	createFrontendStateStore,
	getFrontendStateStore,
} from "./frontend-state";

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((resolvePromise) => {
		resolve = resolvePromise;
	});
	return { promise, resolve };
}

function createPersistence(): FrontendStatePersistence {
	return {
		global: {
			getAll: async () => ({}),
			set: async () => {},
		},
		page: {
			getAll: async () => ({}),
			set: async () => {},
			clearPage: async () => {},
		},
	};
}

describe("frontend state store", () => {
	test("returns one shared store per app and isolates different apps", () => {
		expect(getFrontendStateStore("shared-app")).toBe(
			getFrontendStateStore("shared-app"),
		);
		expect(getFrontendStateStore("shared-app")).not.toBe(
			getFrontendStateStore("other-app"),
		);
		expect(getFrontendStateStore(undefined)).toBe(
			getFrontendStateStore(undefined),
		);
		const first = createFrontendStateStore("first", createPersistence());
		const second = createFrontendStateStore("second", createPersistence());
		first.setGlobalState("theme", "dark");
		first.setPageState("page", "count", 1);
		expect(second.getSnapshot()).toEqual({ globalState: {}, pageStates: {} });
	});

	test("notifies every subscriber and preserves earlier snapshots", () => {
		const store = createFrontendStateStore(undefined);
		let firstUpdates = 0;
		let secondUpdates = 0;
		const unsubscribe = store.subscribe(() => firstUpdates++);
		store.subscribe(() => secondUpdates++);
		const original = store.getSnapshot();
		expect(original.globalState).toBe(EMPTY_FRONTEND_STATE_RECORD);
		expect(store.getSnapshot()).toBe(original);
		store.setPageState("page", "count", 1);
		const first = store.getSnapshot();
		unsubscribe();
		store.setPageState("page", "count", 2);
		store.setGlobalState("theme", "dark");
		expect(firstUpdates).toBe(1);
		expect(secondUpdates).toBe(3);
		expect(original).toEqual({ globalState: {}, pageStates: {} });
		expect(first.pageStates.page).toEqual({ count: 1 });
		expect(Object.isFrozen(first)).toBe(true);
		expect(Object.isFrozen(first.pageStates)).toBe(true);
		expect(Object.isFrozen(first.pageStates.page)).toBe(true);
	});

	test("accepts Rust and camelCase page messages and keeps pages separate", () => {
		const store = createFrontendStateStore(undefined);
		expect(
			store.handleMessage({
				type: "setPageState",
				page_id: "first",
				key: "count",
				value: 0,
			}),
		).toBe(true);
		store.handleMessage({
			type: "setPageState",
			pageId: "second",
			key: "enabled",
			value: false,
		});
		store.handleMessage({
			type: "setGlobalState",
			key: "selection",
			value: null,
		});
		expect(store.getSnapshot()).toEqual({
			globalState: { selection: null },
			pageStates: { first: { count: 0 }, second: { enabled: false } },
		});
		store.handleMessage({ type: "clearPageState", page_id: "first" });
		expect(store.getSnapshot().pageStates).toEqual({
			first: {},
			second: { enabled: false },
		});
		store.handleMessage({ type: "clearPageState", pageId: "second" });
		expect(store.getSnapshot().pageStates.second).toEqual({});
	});

	test("consumes invalid state messages without changing state", () => {
		const store = createFrontendStateStore(undefined);
		const initial = store.getSnapshot();
		for (const unsafe of [
			"",
			"__proto__",
			"constructor",
			"prototype",
			null,
			42,
		]) {
			for (const message of [
				{ type: "setGlobalState", key: unsafe, value: true },
				{ type: "setPageState", page_id: unsafe, key: "valid", value: true },
				{ type: "setPageState", pageId: "valid", key: unsafe, value: true },
				{ type: "clearPageState", pageId: unsafe },
			]) {
				expect(store.handleMessage(message)).toBe(true);
			}
		}
		expect(
			store.handleMessage({ type: "setGlobalState", key: "missing" }),
		).toBe(true);
		expect(
			store.handleMessage({
				type: "setPageState",
				pageId: "page",
				key: "missing",
			}),
		).toBe(true);
		for (const message of [null, [], "invalid", { type: "surfaceUpdate" }]) {
			expect(store.handleMessage(message)).toBe(false);
		}
		expect(store.getSnapshot()).toBe(initial);
	});

	test("supports keys inherited by ordinary objects without exposing prototypes", () => {
		const store = createFrontendStateStore(undefined);
		expect(store.getSnapshot().pageStates.toString).toBeUndefined();
		store.setPageState("toString", "hasOwnProperty", false);
		store.setGlobalState("toString", 0);
		expect(store.getSnapshot()).toEqual({
			globalState: { toString: 0 },
			pageStates: { toString: { hasOwnProperty: false } },
		});
	});

	test("loads global state once and each page once across concurrent consumers", async () => {
		const storage = createPersistence();
		const globalReads: string[] = [];
		const pageReads: string[] = [];
		storage.global.getAll = async (appId) => {
			globalReads.push(appId);
			return { theme: "dark" };
		};
		storage.page.getAll = async (appId, pageId) => {
			pageReads.push(`${appId}/${pageId}`);
			return { page: pageId };
		};
		const store = createFrontendStateStore("app", storage);
		await Promise.all([
			store.ensureLoaded("first"),
			store.ensureLoaded("first"),
			store.ensureLoaded("second"),
		]);
		await store.ensureLoaded("first");
		expect(globalReads).toEqual(["app"]);
		expect(pageReads).toEqual(["app/first", "app/second"]);
		expect(store.getSnapshot()).toEqual({
			globalState: { theme: "dark" },
			pageStates: { first: { page: "first" }, second: { page: "second" } },
		});
	});

	test("merges persisted state without overwriting writes before or during hydration", async () => {
		const storage = createPersistence();
		const globalRead = deferred<Record<string, unknown>>();
		const pageRead = deferred<Record<string, unknown>>();
		storage.global.getAll = () => globalRead.promise;
		storage.page.getAll = () => pageRead.promise;
		const store = createFrontendStateStore("app", storage);
		store.setGlobalState("before", 0);
		store.setPageState("page", "before", false);
		const loaded = store.ensureLoaded("page");
		store.setGlobalState("during", null);
		store.setPageState("page", "during", "current");
		globalRead.resolve({ before: 9, during: "stale", persisted: "global" });
		pageRead.resolve({ before: true, during: "stale", persisted: "page" });
		await loaded;
		expect(store.getSnapshot()).toEqual({
			globalState: { before: 0, during: null, persisted: "global" },
			pageStates: {
				page: { before: false, during: "current", persisted: "page" },
			},
		});
	});

	test("a clear during hydration discards old rows and preserves subsequent writes", async () => {
		const storage = createPersistence();
		const pageRead = deferred<Record<string, unknown>>();
		storage.page.getAll = () => pageRead.promise;
		const store = createFrontendStateStore("app", storage);
		const loaded = store.ensureLoaded("page");
		store.clearPageState("page");
		store.setPageState("page", "new", 2);
		pageRead.resolve({ stale: 1, new: 1 });
		await loaded;
		expect(store.getSnapshot().pageStates.page).toEqual({ new: 2 });
	});

	test("a clear before hydration cannot resurrect persisted state", async () => {
		const storage = createPersistence();
		storage.page.getAll = async () => ({ stale: true });
		const store = createFrontendStateStore("app", storage);
		store.clearPageState("page");
		await store.ensureLoaded("page");
		expect(store.getSnapshot().pageStates.page).toEqual({});
	});

	test("drops unsafe persisted keys during hydration", async () => {
		const storage = createPersistence();
		const persisted = JSON.parse(
			'{"__proto__":{"polluted":true},"constructor":1,"prototype":2,"":3,"valid":0}',
		);
		storage.global.getAll = async () => persisted;
		storage.page.getAll = async () => persisted;
		const store = createFrontendStateStore("app", storage);
		await store.ensureLoaded("page");
		expect(store.getSnapshot()).toEqual({
			globalState: { valid: 0 },
			pageStates: { page: { valid: 0 } },
		});
	});

	test("retries failed hydration while preserving writes and cleared pages", async () => {
		const storage = createPersistence();
		const error = spyOn(console, "error").mockImplementation(() => {});
		let globalReads = 0;
		const pageReads = new Map<string, number>();
		storage.global.getAll = async () => {
			if (++globalReads === 1) throw new Error("Storage unavailable");
			return { pending: "stale", persisted: "global" };
		};
		storage.page.getAll = async (_appId, pageId) => {
			const reads = (pageReads.get(pageId) ?? 0) + 1;
			pageReads.set(pageId, reads);
			if (reads === 1) throw new Error("Storage unavailable");
			return { pending: "stale", persisted: "page" };
		};
		try {
			const store = createFrontendStateStore("app", storage);
			await Promise.all([
				store.ensureLoaded("edited"),
				store.ensureLoaded("cleared"),
			]);
			store.setGlobalState("pending", false);
			store.setPageState("edited", "pending", 0);
			store.clearPageState("cleared");
			store.setPageState("cleared", "pending", null);
			await Promise.all([
				store.ensureLoaded("edited"),
				store.ensureLoaded("cleared"),
			]);
			expect(globalReads).toBe(2);
			expect(pageReads.get("edited")).toBe(2);
			expect(pageReads.get("cleared")).toBe(2);
			expect(error).toHaveBeenCalledTimes(3);
			expect(store.getSnapshot()).toEqual({
				globalState: { pending: false, persisted: "global" },
				pageStates: {
					edited: { pending: 0, persisted: "page" },
					cleared: { pending: null },
				},
			});
		} finally {
			error.mockRestore();
		}
	});

	test("serializes persistence in message order, including clears", async () => {
		const storage = createPersistence();
		const firstStarted = deferred<void>();
		const firstRelease = deferred<void>();
		const clearStarted = deferred<void>();
		const clearRelease = deferred<void>();
		const lastStarted = deferred<void>();
		const operations: unknown[][] = [];
		storage.page.set = async (appId, pageId, key, value) => {
			operations.push(["set", appId, pageId, key, value]);
			if (value === 1) {
				firstStarted.resolve();
				await firstRelease.promise;
			} else {
				lastStarted.resolve();
			}
		};
		storage.page.clearPage = async (appId, pageId) => {
			operations.push(["clear", appId, pageId]);
			clearStarted.resolve();
			await clearRelease.promise;
		};
		storage.global.set = async (appId, key, value) => {
			operations.push(["global", appId, key, value]);
		};
		const store = createFrontendStateStore("app", storage);
		store.setPageState("page", "count", 1);
		store.clearPageState("page");
		store.setGlobalState("enabled", false);
		store.setPageState("page", "count", 2);
		await firstStarted.promise;
		expect(operations).toEqual([["set", "app", "page", "count", 1]]);
		firstRelease.resolve();
		await clearStarted.promise;
		expect(operations).toHaveLength(2);
		clearRelease.resolve();
		await lastStarted.promise;
		expect(operations).toEqual([
			["set", "app", "page", "count", 1],
			["clear", "app", "page"],
			["global", "app", "enabled", false],
			["set", "app", "page", "count", 2],
		]);
	});

	test("continues persisting after a failed write", async () => {
		const storage = createPersistence();
		const lastWrite = deferred<void>();
		const error = spyOn(console, "error").mockImplementation(() => {});
		let writes = 0;
		storage.global.set = async () => {
			writes++;
			if (writes === 1) throw new Error("Storage unavailable");
			lastWrite.resolve();
		};
		try {
			const store = createFrontendStateStore("app", storage);
			store.setGlobalState("count", 1);
			store.setGlobalState("count", 2);
			await lastWrite.promise;
			expect(writes).toBe(2);
			expect(store.getSnapshot().globalState.count).toBe(2);
			expect(error).toHaveBeenCalledTimes(1);
		} finally {
			error.mockRestore();
		}
	});

	test("anonymous stores keep state in memory without accessing persistence", async () => {
		const storage = createPersistence();
		const globalRead = spyOn(storage.global, "getAll");
		const pageRead = spyOn(storage.page, "getAll");
		const globalWrite = spyOn(storage.global, "set");
		const pageWrite = spyOn(storage.page, "set");
		const pageClear = spyOn(storage.page, "clearPage");
		const store = createFrontendStateStore(undefined, storage);
		store.setGlobalState("theme", "dark");
		store.setPageState("page", "count", 1);
		store.clearPageState("page");
		await store.ensureLoaded("page");
		for (const operation of [
			globalRead,
			pageRead,
			globalWrite,
			pageWrite,
			pageClear,
		]) {
			expect(operation).not.toHaveBeenCalled();
		}
		expect(store.getSnapshot()).toEqual({
			globalState: { theme: "dark" },
			pageStates: { page: {} },
		});
	});
});
