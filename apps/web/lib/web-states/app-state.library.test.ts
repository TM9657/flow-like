import { QueryClient } from "@tanstack/react-query";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiGet: vi.fn() }));

vi.mock("@flow-like/flow-like-ui", () => ({
	IExecutionStage: {},
	ILogLevel: {},
	isAzureBlobStorageUrl: vi.fn(),
}));
vi.mock("./api-utils", () => ({
	apiDelete: vi.fn(),
	apiGet: mocks.apiGet,
	apiPatch: vi.fn(),
	apiPost: vi.fn(),
	apiPut: vi.fn(),
}));

import { WebAppState } from "./app-state";

const auth = { isAuthenticated: true, user: { access_token: "token" } };
const listings = [
	["getApps", "apps"],
	["getMyGroups", "user/groups"],
] as const;

describe("web library listings", () => {
	beforeEach(() => vi.resetAllMocks());

	test.each(listings)(
		"%s preserves request failures for retry",
		async (method, path) => {
			const failure = new Error("session is not ready");
			mocks.apiGet.mockRejectedValue(failure);
			const state = new WebAppState({ auth } as never);

			await expect(state[method]()).rejects.toBe(failure);
			expect(mocks.apiGet).toHaveBeenCalledWith(path, auth);
		},
	);

	test.each(listings)(
		"%s accepts an empty list confirmed by the server",
		async (method, path) => {
			mocks.apiGet.mockResolvedValue([]);
			const state = new WebAppState({ auth } as never);

			await expect(state[method]()).resolves.toEqual([]);
			expect(mocks.apiGet).toHaveBeenCalledWith(path, auth);
		},
	);

	test.each(listings)(
		"%s keeps the last successful query result when refresh fails",
		async (method) => {
			const client = new QueryClient();
			const previous =
				method === "getApps"
					? [[{ id: "saved-app" }, { name: "Saved app" }]]
					: [{ id: "saved-group", name: "Saved group" }];
			const failure = new Error("hub unavailable");
			client.setQueryData([method], previous);
			mocks.apiGet.mockRejectedValue(failure);
			const state = new WebAppState({ auth } as never);

			try {
				await expect(
					client.fetchQuery<unknown>({
						queryKey: [method],
						queryFn: () => state[method](),
						retry: false,
					}),
				).rejects.toBe(failure);
				expect(client.getQueryData([method])).toEqual(previous);
			} finally {
				client.clear();
			}
		},
	);
});
