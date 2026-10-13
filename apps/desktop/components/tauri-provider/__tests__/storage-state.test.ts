import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	fetcher: vi.fn(),
	invoke: vi.fn(),
	dirname: vi.fn(),
	resolve: vi.fn(),
	mkdir: vi.fn(),
	open: vi.fn(),
	remove: vi.fn(),
	rename: vi.fn(),
	runBulkUpload: vi.fn(),
	toUploadTasks: vi.fn(),
}));
vi.mock("@flow-like/flow-like-ui", () => ({
	BulkUploadAbortError: class extends Error {},
	assertBulkUploadSucceeded: vi.fn(),
	requestPrefixesInBatches: vi.fn(),
	runBulkUpload: mocks.runBulkUpload,
	storageDisplayName: vi.fn(),
	toUploadTasks: mocks.toUploadTasks,
	uploadToSignedUrl: vi.fn(),
}));
vi.mock("@flow-like/flow-like-ui/lib/stable-asset-url", () => ({
	stabilizeSignedUrls: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/path", () => ({
	dirname: mocks.dirname,
	resolve: mocks.resolve,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({
	mkdir: mocks.mkdir,
	open: mocks.open,
	remove: mocks.remove,
	rename: mocks.rename,
}));
vi.mock("../../../lib/api", () => ({ fetcher: mocks.fetcher }));

import { StorageState } from "../storage-state";

const routes = [
	["listStorageItems", "apps/app/data/list", "storage_list"],
	["listStorageItemsUser", "apps/app/data/user/list", "storage_user_list"],
] as const;

describe("desktop storage listing refresh", () => {
	beforeEach(() => {
		vi.resetAllMocks();
		mocks.fetcher.mockResolvedValue([]);
		mocks.invoke.mockResolvedValue([]);
	});

	test.each(routes)(
		"%s refreshes remote storage through the URL",
		async (method, endpoint) => {
			const backend = {
				isOffline: vi.fn().mockResolvedValue(false),
				profile: {},
				auth: {},
				queryClient: {},
			};
			const state = new StorageState(backend as never);
			const request = {
				method: "POST",
				body: JSON.stringify({ prefix: "docs" }),
			};
			for (const options of [undefined, { refresh: false }]) {
				await state[method]("app", "docs", options);
				expect(mocks.fetcher).toHaveBeenLastCalledWith(
					backend.profile,
					endpoint,
					request,
					backend.auth,
				);
			}
			await state[method]("app", "docs", { refresh: true });
			expect(mocks.fetcher).toHaveBeenLastCalledWith(
				backend.profile,
				`${endpoint}?refresh=true`,
				request,
				backend.auth,
			);
			expect(mocks.invoke).not.toHaveBeenCalled();
			mocks.fetcher.mockRejectedValueOnce(new Error("denied"));
			await expect(
				state[method]("app", "docs", { refresh: true }),
			).rejects.toThrow("denied");
		},
	);

	test.each(routes)(
		"%s keeps offline listings local",
		async (method, _endpoint, command) => {
			const state = new StorageState({
				isOffline: vi.fn().mockResolvedValue(true),
			} as never);
			await state[method]("app", "docs", { refresh: true });
			expect(mocks.invoke).toHaveBeenCalledWith(command, {
				appId: "app",
				prefix: "docs",
			});
			expect(mocks.fetcher).not.toHaveBeenCalled();
		},
	);
});

describe("desktop storage rename", () => {
	const renameRoutes = [
		["renameStorageItem", "apps/app/data/rename", "storage_rename"],
		[
			"renameStorageItemUser",
			"apps/app/data/user/rename",
			"storage_user_rename",
		],
	] as const;

	beforeEach(() => {
		vi.resetAllMocks();
		mocks.fetcher.mockResolvedValue(undefined);
		mocks.invoke.mockResolvedValue(undefined);
	});

	test.each(renameRoutes)(
		"%s sends hub renames only to remote storage",
		async (method, endpoint) => {
			const backend = {
				isOffline: vi.fn().mockResolvedValue(false),
				profile: {},
				auth: {},
				queryClient: {},
			};
			const state = new StorageState(backend as never);
			for (const isDir of [false, true]) {
				await state[method]("app", "docs/old%20name", "new 100% name", isDir);
				expect(mocks.fetcher).toHaveBeenLastCalledWith(
					backend.profile,
					endpoint,
					{
						method: "POST",
						body: JSON.stringify({
							prefix: "docs/old%20name",
							new_name: "new 100% name",
							is_dir: isDir,
						}),
					},
					backend.auth,
				);
			}
			expect(mocks.invoke).not.toHaveBeenCalled();
		},
	);

	test.each(renameRoutes)(
		"%s sends offline renames to its scoped native command",
		async (method, _endpoint, command) => {
			const state = new StorageState({
				isOffline: vi.fn().mockResolvedValue(true),
			} as never);
			for (const isDir of [false, true]) {
				await state[method]("app", "docs/old%20name", "new 100% name", isDir);
				expect(mocks.invoke).toHaveBeenLastCalledWith(command, {
					appId: "app",
					prefix: "docs/old%20name",
					newName: "new 100% name",
					isDir,
				});
			}
			expect(mocks.fetcher).not.toHaveBeenCalled();
		},
	);

	test.each(renameRoutes)(
		"%s propagates remote errors without falling back to a local rename",
		async (method) => {
			const state = new StorageState({
				isOffline: vi.fn().mockResolvedValue(false),
				profile: {},
				auth: {},
				queryClient: {},
			} as never);
			mocks.fetcher.mockRejectedValueOnce(new Error("Name already exists"));
			await expect(
				state[method]("app", "old", "existing", true),
			).rejects.toThrow("Name already exists");
			expect(mocks.invoke).not.toHaveBeenCalled();
		},
	);

	test.each(renameRoutes)(
		"%s propagates native rename errors",
		async (method) => {
			const state = new StorageState({
				isOffline: vi.fn().mockResolvedValue(true),
			} as never);
			mocks.invoke.mockRejectedValueOnce(new Error("File is in use"));
			await expect(state[method]("app", "old", "new", false)).rejects.toThrow(
				"File is in use",
			);
			expect(mocks.fetcher).not.toHaveBeenCalled();
		},
	);

	test.each(renameRoutes)(
		"%s rejects an uninitialized remote backend",
		async (method) => {
			const state = new StorageState({
				isOffline: vi.fn().mockResolvedValue(false),
			} as never);
			await expect(state[method]("app", "old", "new", false)).rejects.toThrow(
				"not properly initialized for renaming",
			);
			expect(mocks.fetcher).not.toHaveBeenCalled();
			expect(mocks.invoke).not.toHaveBeenCalled();
		},
	);
});

describe("desktop storage replacement", () => {
	const target = "/storage/docs/report.docx";
	const original = new TextEncoder().encode("original document");
	let disk: Map<string, Uint8Array>;
	let temporaryPath: string;
	let controller: AbortController;
	let state: StorageState;
	let write: ReturnType<typeof vi.fn>;
	let close: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		vi.resetAllMocks();
		disk = new Map([[target, original]]);
		temporaryPath = "";
		controller = new AbortController();
		state = new StorageState({
			isOffline: vi.fn().mockResolvedValue(true),
		} as never);
		mocks.dirname.mockResolvedValue("/storage/docs");
		mocks.resolve.mockImplementation(async (...parts: string[]) =>
			parts.join("/"),
		);
		mocks.mkdir.mockResolvedValue(undefined);
		mocks.toUploadTasks.mockImplementation((prefix: string, files: File[]) =>
			files.map((file) => ({ path: `${prefix}/${file.name}`, file })),
		);
		mocks.runBulkUpload.mockImplementation(async (tasks, handlers, options) => {
			await handlers.send(
				`asset://localhost/${encodeURIComponent(target)}`,
				tasks[0],
				vi.fn(),
				options.signal,
			);
			return { uploaded: 1, failed: [], cancelled: false };
		});
		write = vi.fn(async (bytes: Uint8Array) => {
			const previous = disk.get(temporaryPath) ?? new Uint8Array();
			const next = new Uint8Array(previous.length + bytes.length);
			next.set(previous);
			next.set(bytes, previous.length);
			disk.set(temporaryPath, next);
			return bytes.length;
		});
		close = vi.fn().mockResolvedValue(undefined);
		mocks.open.mockImplementation(async (path: string) => {
			if (disk.has(path)) throw new Error("File already exists");
			temporaryPath = path;
			disk.set(path, new Uint8Array());
			return { write, close };
		});
		mocks.rename.mockImplementation(async (from: string, to: string) => {
			expect(disk.get(to)).toEqual(original);
			expect(close).toHaveBeenCalled();
			disk.set(to, disk.get(from) as Uint8Array);
			disk.delete(from);
		});
		mocks.remove.mockImplementation(async (path: string) => {
			disk.delete(path);
		});
	});

	function upload(file = new File(["updated"], "report.docx")) {
		return state.uploadStorageItems("app", "docs", [file], undefined, {
			signal: controller.signal,
		});
	}

	function expectOriginalKept() {
		expect(disk.get(target)).toEqual(original);
		expect(mocks.remove).not.toHaveBeenCalledWith(target);
		expect(disk.size).toBe(1);
	}

	test("replaces the old file only after the sibling write closes", async () => {
		await upload();
		expect(mocks.open).toHaveBeenCalledWith(
			expect.stringMatching(/^\/storage\/docs\/flow-like-upload-[\w-]+\.tmp$/),
			{ createNew: true, write: true },
		);
		expect(mocks.rename).toHaveBeenCalledWith(temporaryPath, target);
		expect(new TextDecoder().decode(disk.get(target))).toBe("updated");
		expect(disk.size).toBe(1);
		expect(mocks.remove).not.toHaveBeenCalled();
	});

	test("keeps the old file and removes only the temporary file after a write fails", async () => {
		write.mockRejectedValueOnce(new Error("disk full"));
		await expect(upload()).rejects.toThrow("disk full");
		expect(close).toHaveBeenCalled();
		expect(mocks.rename).not.toHaveBeenCalled();
		expect(mocks.remove).toHaveBeenCalledWith(temporaryPath);
		expectOriginalKept();
	});

	test("completes a short write before replacing the old file", async () => {
		write.mockImplementationOnce(async (bytes: Uint8Array) => {
			disk.set(temporaryPath, bytes.slice(0, 2));
			return 2;
		});
		await upload();
		expect(write).toHaveBeenCalledTimes(2);
		expect(new TextDecoder().decode(write.mock.calls[1][0])).toBe("dated");
		expect(new TextDecoder().decode(disk.get(target))).toBe("updated");
		expect(mocks.rename).toHaveBeenCalledWith(temporaryPath, target);
	});

	test.each([0, -1, Number.NaN, 8])(
		"keeps the old file when write reports invalid progress %s",
		async (written) => {
			write.mockResolvedValueOnce(written);
			await expect(upload()).rejects.toThrow("complete file");
			expect(mocks.rename).not.toHaveBeenCalled();
			expectOriginalKept();
		},
	);

	test("does not open a file for a cancelled upload", async () => {
		controller.abort();
		await expect(upload()).rejects.toThrow();
		expect(mocks.open).not.toHaveBeenCalled();
		expect(mocks.remove).not.toHaveBeenCalled();
		expectOriginalKept();
	});

	test("keeps the old file when cancellation arrives while reading buffered bytes", async () => {
		const file = new File(["updated"], "report.docx");
		vi.spyOn(file, "arrayBuffer").mockImplementation(async () => {
			controller.abort();
			return new TextEncoder().encode("updated").buffer;
		});
		await expect(upload(file)).rejects.toThrow();
		expect(write).not.toHaveBeenCalled();
		expect(mocks.rename).not.toHaveBeenCalled();
		expectOriginalKept();
	});

	test("keeps the old file when a streaming upload is cancelled after writing a chunk", async () => {
		write.mockImplementationOnce(async (bytes: Uint8Array) => {
			disk.set(temporaryPath, bytes);
			controller.abort();
			return bytes.length;
		});
		const file = new File([new Uint8Array(8 * 1024 * 1024)], "report.docx");
		await expect(upload(file)).rejects.toThrow();
		expect(write).toHaveBeenCalledTimes(1);
		expect(mocks.rename).not.toHaveBeenCalled();
		expectOriginalKept();
	});

	test("keeps the old file when closing the replacement fails", async () => {
		close.mockRejectedValueOnce(new Error("close failed"));
		await expect(upload()).rejects.toThrow("close failed");
		expect(mocks.rename).not.toHaveBeenCalled();
		expectOriginalKept();
	});

	test("keeps the old file when the final rename fails", async () => {
		mocks.rename.mockRejectedValueOnce(new Error("file locked"));
		await expect(upload()).rejects.toThrow("file locked");
		expect(mocks.remove).toHaveBeenCalledWith(temporaryPath);
		expectOriginalKept();
	});

	test("does not remove another file when exclusive temporary-file creation fails", async () => {
		mocks.open.mockRejectedValueOnce(new Error("File already exists"));
		await expect(upload()).rejects.toThrow("File already exists");
		expect(mocks.rename).not.toHaveBeenCalled();
		expect(mocks.remove).not.toHaveBeenCalled();
		expectOriginalKept();
	});
});
