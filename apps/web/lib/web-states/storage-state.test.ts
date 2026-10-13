import {
	resetStableAssetUrls,
	stableAssetUrl,
} from "@flow-like/flow-like-ui/lib/stable-asset-url";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiPost: vi.fn() }));
vi.mock("@flow-like/flow-like-ui", async () => ({
	assertBulkUploadSucceeded: vi.fn(),
	requestPrefixesInBatches: (
		await import("@flow-like/flow-like-ui/lib/storage-batching")
	).requestPrefixesInBatches,
	runBulkUpload: vi.fn(),
	toUploadTasks: vi.fn(),
	uploadToSignedUrl: vi.fn(),
}));
vi.mock("./api-utils", () => ({
	apiPost: mocks.apiPost,
	apiDelete: vi.fn(),
	apiFetch: vi.fn(),
}));

import { WebStorageState } from "./storage-state";

describe("web storage downloads", () => {
	beforeEach(() => {
		mocks.apiPost.mockReset();
		resetStableAssetUrls();
	});

	test.each([
		["downloadStorageItems", "apps/app/data/download"],
		["downloadStorageItemsUser", "apps/app/data/user/download"],
	] as const)(
		"%s preserves new signatures for overwritten files",
		async (method, endpoint) => {
			const auth = { token: "test" };
			const state = new WebStorageState({ auth } as never);
			const expires = Math.floor(Date.now() / 1000) + 86_400;
			const base = "https://bucket.s3.amazonaws.com/docs/video.mp4";
			const first = `${base}?AWSAccessKeyId=key&Expires=${expires}&Signature=first`;
			const second = `${base}?AWSAccessKeyId=key&Expires=${expires + 60}&Signature=second`;
			expect(stableAssetUrl(first)).toBe(first);
			mocks.apiPost
				.mockResolvedValueOnce([{ prefix: "docs/video.mp4", url: first }])
				.mockResolvedValueOnce([{ prefix: "docs/video.mp4", url: second }]);
			expect(await state[method]("app", ["docs/video.mp4"])).toEqual([
				{ prefix: "docs/video.mp4", url: first },
			]);
			expect(await state[method]("app", ["docs/video.mp4"])).toEqual([
				{ prefix: "docs/video.mp4", url: second },
			]);
			expect(mocks.apiPost).toHaveBeenLastCalledWith(
				endpoint,
				{ prefixes: ["docs/video.mp4"] },
				auth,
			);
		},
	);
});

describe("web storage listing refresh", () => {
	beforeEach(() => {
		mocks.apiPost.mockReset();
		mocks.apiPost.mockResolvedValue([]);
	});

	test.each([
		["listStorageItems", "apps/app/data/list"],
		["listStorageItemsUser", "apps/app/data/user/list"],
	] as const)(
		"%s refreshes through the URL and preserves the body",
		async (method, endpoint) => {
			const auth = { token: "test" };
			const state = new WebStorageState({ auth } as never);
			for (const options of [undefined, { refresh: false }]) {
				await state[method]("app", "docs", options);
				expect(mocks.apiPost).toHaveBeenLastCalledWith(
					endpoint,
					{ prefix: "docs" },
					auth,
				);
			}
			await state[method]("app", "docs", { refresh: true });
			expect(mocks.apiPost).toHaveBeenLastCalledWith(
				`${endpoint}?refresh=true`,
				{ prefix: "docs" },
				auth,
			);
			mocks.apiPost.mockRejectedValueOnce(new Error("denied"));
			await expect(
				state[method]("app", "docs", { refresh: true }),
			).rejects.toThrow("denied");
		},
	);
});

describe("web storage rename", () => {
	beforeEach(() => {
		mocks.apiPost.mockReset();
		mocks.apiPost.mockResolvedValue(null);
	});

	test.each([
		["renameStorageItem", "apps/app/data/rename"],
		["renameStorageItemUser", "apps/app/data/user/rename"],
	] as const)(
		"%s preserves scope, paths, and item kind",
		async (method, endpoint) => {
			const auth = { token: "test" };
			const state = new WebStorageState({ auth } as never);
			for (const isDir of [false, true]) {
				await state[method]("app", "docs/old%23name", "new 100%.txt", isDir);
				expect(mocks.apiPost).toHaveBeenLastCalledWith(
					endpoint,
					{
						prefix: "docs/old%23name",
						new_name: "new 100%.txt",
						is_dir: isDir,
					},
					auth,
				);
			}
		},
	);

	test.each(["renameStorageItem", "renameStorageItemUser"] as const)(
		"%s propagates name conflicts to the browser",
		async (method) => {
			const state = new WebStorageState({ auth: {} } as never);
			mocks.apiPost.mockRejectedValueOnce(new Error("Name already exists"));
			await expect(state[method]("app", "old", "taken", true)).rejects.toThrow(
				"Name already exists",
			);
		},
	);
});
