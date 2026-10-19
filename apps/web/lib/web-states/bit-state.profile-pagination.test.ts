import type { IBit, IProfile } from "@flow-like/flow-like-ui";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { WebBackendRef } from "./api-utils";

const mocks = vi.hoisted(() => ({ apiGet: vi.fn() }));
vi.mock("@flow-like/flow-like-ui", () => ({}));
vi.mock("./api-state", () => ({ WebApiState: class {} }));
vi.mock("./api-utils", () => ({ apiGet: mocks.apiGet }));

import { WebBitState } from "./bit-state";

function backend(count: number): WebBackendRef {
	return {
		profile: {
			id: "profile-1",
			hub: "hub.example",
			bits: Array.from({ length: count }, (_, index) => `model-${index}`),
		} as IProfile,
	};
}

function bit(id: string, type = "File"): IBit {
	return { id, hub: "hub.example", type } as IBit;
}

describe("web profile Bit pagination", () => {
	beforeEach(() => vi.resetAllMocks());

	test("finds forecasting models after missing rows and deduplicates repeated private Bits", async () => {
		const privateModel = bit("private", "Forecasting");
		const lateModel = bit("model-200", "Forecasting");
		mocks.apiGet
			.mockResolvedValueOnce([bit("model-0"), privateModel])
			.mockResolvedValueOnce([privateModel])
			.mockResolvedValueOnce([lateModel, privateModel]);
		const result = await new WebBitState(backend(201)).getProfileBits();
		expect(result).toEqual([bit("model-0"), privateModel, lateModel]);
		expect(mocks.apiGet.mock.calls.map(([url]) => url)).toEqual([
			"profile/profile-1/bits?limit=100&offset=0&device_models=true",
			"profile/profile-1/bits?limit=100&offset=100&device_models=true",
			"profile/profile-1/bits?limit=100&offset=200&device_models=true",
		]);
	});

	test("propagates later page failures instead of returning an incomplete model list", async () => {
		mocks.apiGet
			.mockResolvedValueOnce([bit("model-0")])
			.mockRejectedValueOnce(new Error("Session expired"));
		await expect(
			new WebBitState(backend(101)).getProfileBits(),
		).rejects.toThrow("Session expired");
	});

	test("does not confuse malformed responses with an empty profile", async () => {
		mocks.apiGet.mockResolvedValueOnce({ error: "unavailable" });
		await expect(new WebBitState(backend(0)).getProfileBits()).rejects.toThrow(
			"invalid response",
		);
		mocks.apiGet.mockResolvedValueOnce([]);
		await expect(new WebBitState(backend(0)).getProfileBits()).resolves.toEqual(
			[],
		);
	});

	test("does not request a model list without an active profile", async () => {
		await expect(new WebBitState({}).getProfileBits()).resolves.toEqual([]);
		expect(mocks.apiGet).not.toHaveBeenCalled();
	});
});
