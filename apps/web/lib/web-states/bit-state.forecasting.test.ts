import { describe, expect, it, vi } from "vitest";
import {
	type IBit,
	IBitTypes,
} from "@flow-like/flow-like-ui/lib/schema/bit/bit";
import type { WebBackendRef } from "./api-utils";

vi.mock("@flow-like/flow-like-ui", async () => ({
	IBitTypes: (await import("@flow-like/flow-like-ui/lib/schema/bit/bit"))
		.IBitTypes,
}));
vi.mock("./api-state", () => ({ WebApiState: class {} }));
vi.mock("./api-utils", () => ({
	apiGet: vi.fn(),
	apiPost: vi.fn(),
	apiPut: vi.fn(),
	apiDelete: vi.fn(),
}));

import { WebBitState } from "./bit-state";

describe("web forecasting checkpoint packs", () => {
	const root = {
		id: "private-root",
		hub: "hub.example",
		type: IBitTypes.Forecasting,
		size: 100,
		parameters: {
			model_type: "chronos2",
			assets: [
				{ bit: "config", relative_path: "config.json" },
				{
					bit: "https://other.example:8443:weights",
					relative_path: "model-00001-of-00001.safetensors",
				},
			],
		},
	} as IBit;

	it("resolves published assets directly for private roots and reports their download size", async () => {
		const state = new WebBitState({} as WebBackendRef);
		const read = vi.fn(async (id: string, hub?: string) => ({
			...root,
			type: IBitTypes.File,
			id,
			hub: hub ?? "",
			size: id === "config" ? 5 : 1000,
		}));
		state.getBit = read;
		const pack = await state.getPackFromBit(root);
		expect(pack.bits.map((bit) => bit.id)).toEqual([
			"private-root",
			"config",
			"weights",
		]);
		expect(read.mock.calls).toEqual([
			["config", "hub.example"],
			["weights", "https://other.example:8443"],
		]);
		expect(await state.getPackSize(pack.bits)).toBe(1105);
		expect(await state.getBitSize(root)).toBe(1105);
	});

	it("does not report an incomplete pack when a required asset is missing", async () => {
		const state = new WebBitState({} as WebBackendRef);
		state.getBit = async () => {
			throw new Error("Asset not found");
		};
		await expect(state.getPackFromBit(root)).rejects.toThrow("Asset not found");
	});
});
