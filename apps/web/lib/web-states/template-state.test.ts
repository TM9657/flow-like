import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	apiGet: vi.fn(),
	apiPut: vi.fn(),
	apiDelete: vi.fn(),
}));
vi.mock("./api-utils", () => mocks);

import { WebTemplateState } from "./template-state";

describe("web Scout template reads", () => {
	beforeEach(() => vi.resetAllMocks());

	test("preserves default search failures as empty while strict search reports the failure", async () => {
		const error = new Error("public unavailable");
		mocks.apiGet.mockRejectedValue(error);
		const state = new WebTemplateState({ auth: {} } as never);
		await expect(state.searchTemplates({ query: "invoice" })).resolves.toEqual(
			[],
		);
		await expect(
			state.searchTemplates(
				{ query: "invoice", offset: 7, limit: 4 },
				{ strict: true },
			),
		).rejects.toBe(error);
		expect(mocks.apiGet).toHaveBeenLastCalledWith(
			"apps/templates/search?query=invoice&limit=4&offset=7",
			{},
		);
	});

	test("uses one bounded owned membership page and never treats an empty page as exhaustive", async () => {
		mocks.apiGet.mockResolvedValue([]);
		const onCoverage = vi.fn();
		await expect(
			new WebTemplateState({ auth: {} } as never).getTemplates(
				undefined,
				"de",
				{ strict: true, readOnly: true, onCoverage },
			),
		).resolves.toEqual([]);
		expect(mocks.apiGet).toHaveBeenCalledTimes(1);
		expect(mocks.apiGet).toHaveBeenCalledWith(
			"user/templates?language=de&limit=100&offset=0",
			{},
		);
		expect(onCoverage).toHaveBeenCalledWith(
			expect.objectContaining({
				complete: false,
				warning: expect.stringContaining("first 100 memberships"),
			}),
		);
		expect(mocks.apiPut).not.toHaveBeenCalled();
		expect(mocks.apiDelete).not.toHaveBeenCalled();
	});

	test("retains owned listing failures for Scout while ordinary listings keep their existing fallback", async () => {
		mocks.apiGet.mockRejectedValue(new Error("owned unavailable"));
		const state = new WebTemplateState({ auth: {} } as never);
		await expect(state.getTemplates()).resolves.toEqual([]);
		await expect(
			state.getTemplates(undefined, undefined, {
				strict: true,
				readOnly: true,
			}),
		).rejects.toMatchObject({
			name: "TemplateMetadataReadError",
			message: "Owned remote template metadata: owned unavailable",
			partialTemplates: [],
		});
	});
});
