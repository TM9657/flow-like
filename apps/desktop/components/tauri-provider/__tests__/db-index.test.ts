import { IIndexType } from "@flow-like/flow-like-ui/state/backend-state/db-state";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	invoke: vi.fn(),
	fetcher: vi.fn(),
	apiPost: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../../../lib/api", () => ({ fetcher: mocks.fetcher }));
vi.mock("../../../../web/lib/web-states/api-utils", () => ({
	apiPost: mocks.apiPost,
	apiGet: vi.fn(),
	apiPut: vi.fn(),
	apiDelete: vi.fn(),
}));

import { WebDatabaseState } from "../../../../web/lib/web-states/database-state";
import { DatabaseState } from "../db-state";

const INDEX_TYPES = [
	[IIndexType.FullText, "FullText"],
	[IIndexType.BTree, "BTree"],
	[IIndexType.Bitmap, "Bitmap"],
	[IIndexType.LabelList, "LabelList"],
	[IIndexType.Auto, "Auto"],
	[IIndexType.Vector, "Vector"],
	[IIndexType.Fm, "Fm"],
	[IIndexType.IvfFlat, "IvfFlat"],
	[IIndexType.IvfPq, "IvfPq"],
	[IIndexType.IvfSq, "IvfSq"],
	[IIndexType.IvfRq, "IvfRq"],
	[IIndexType.IvfHnswFlat, "IvfHnswFlat"],
	[IIndexType.IvfHnswPq, "IvfHnswPq"],
	[IIndexType.IvfHnswSq, "IvfHnswSq"],
	[IIndexType.NGram, "NGram"],
	[IIndexType.ZoneMap, "ZoneMap"],
	[IIndexType.BloomFilter, "BloomFilter"],
	[IIndexType.RTree, "RTree"],
] as const;

beforeEach(() => {
	vi.clearAllMocks();
});

describe("database index requests", () => {
	test.each(INDEX_TYPES)(
		"sends enum %s as %s to the desktop command",
		async (indexType, name) => {
			const database = new DatabaseState({
				isOffline: async () => true,
			} as never);

			await database.buildIndex(
				"app-1",
				"documents",
				"embedding",
				indexType,
				true,
				true,
			);

			expect(mocks.invoke).toHaveBeenCalledWith("build_index", {
				appId: "app-1",
				tableName: "documents",
				column: "embedding",
				indexType: name,
				optimize: true,
				userScoped: true,
			});
			expect(mocks.fetcher).not.toHaveBeenCalled();
		},
	);

	test.each(INDEX_TYPES)(
		"sends enum %s as %s through both HTTP backends",
		async (indexType, name) => {
			const auth = { user: { access_token: "test-token" } };
			const profile = { id: "profile-1" };
			const desktop = new DatabaseState({
				isOffline: async () => false,
				auth,
				profile,
			} as never);
			const web = new WebDatabaseState({ auth } as never);

			await desktop.buildIndex(
				"app-1",
				"my documents",
				"embedding",
				indexType,
				undefined,
				true,
			);
			await web.buildIndex(
				"app-1",
				"my documents",
				"embedding",
				indexType,
				undefined,
				true,
			);

			const body = { column: "embedding", index_type: name, optimize: false };
			const path = "apps/app-1/db/my%20documents/index?scope=user";
			expect(mocks.fetcher).toHaveBeenCalledWith(
				profile,
				path,
				{ method: "POST", body: JSON.stringify(body) },
				auth,
			);
			expect(mocks.apiPost).toHaveBeenCalledWith(path, body, auth);
			expect(mocks.invoke).not.toHaveBeenCalled();
		},
	);
});
