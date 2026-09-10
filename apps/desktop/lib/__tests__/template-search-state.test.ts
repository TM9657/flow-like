import type { IMetadata } from "@flow-like/flow-like-ui";
import {
	MAX_OWNED_TEMPLATE_METADATA,
	type TemplateMetadataEntry,
	TemplateMetadataReadError,
} from "@flow-like/flow-like-ui/state/backend-state/template-read";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	invoke: vi.fn(),
	fetcher: vi.fn(),
	injectDataFunction: vi.fn(),
}));
vi.mock("@flow-like/flow-like-ui", () => ({
	injectDataFunction: mocks.injectDataFunction,
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../api", () => ({ fetcher: mocks.fetcher }));

import { TemplateState } from "../../components/tauri-provider/template-state";

function backend() {
	return {
		profile: { id: "profile" },
		auth: { user: { access_token: "test" } },
		queryClient: {},
		backgroundTaskHandler: vi.fn(),
		isOffline: vi.fn().mockResolvedValue(false),
	};
}
function entry(id: string, name = id): TemplateMetadataEntry {
	return ["app", id, { name } as IMetadata];
}

describe("desktop Scout template reads", () => {
	beforeEach(() => vi.resetAllMocks());

	test("preserves default public failure behavior while strict reads propagate errors", async () => {
		const failure = new Error("hub unavailable");
		mocks.fetcher.mockRejectedValue(failure);
		const state = new TemplateState(backend() as never);
		await expect(state.searchTemplates({ query: "invoice" })).resolves.toEqual(
			[],
		);
		await expect(
			state.searchTemplates({ query: "invoice" }, { strict: true }),
		).rejects.toBe(failure);
	});

	test("reports missing public-search profile only for strict reads", async () => {
		const state = new TemplateState({ profile: undefined } as never);
		await expect(state.searchTemplates({ query: "invoice" })).resolves.toEqual(
			[],
		);
		await expect(
			state.searchTemplates({ query: "invoice" }, { strict: true }),
		).rejects.toThrow("Profile not set");
	});

	test("merges owned metadata synchronously without downloading or caching templates", async () => {
		mocks.invoke.mockResolvedValue([entry("shared", "cached"), entry("local")]);
		mocks.fetcher.mockResolvedValue([
			entry("shared", "remote"),
			entry("hosted"),
		]);
		const host = backend();
		const onCoverage = vi.fn();
		const result = await new TemplateState(host as never).getTemplates(
			undefined,
			"de",
			{ strict: true, readOnly: true, onCoverage },
		);
		expect(result).toEqual([
			entry("hosted"),
			entry("local"),
			entry("shared", "remote"),
		]);
		expect(mocks.invoke.mock.calls).toEqual([
			[
				"get_templates",
				{
					appId: undefined,
					language: "de",
					metadataOnly: true,
					limit: MAX_OWNED_TEMPLATE_METADATA,
				},
			],
		]);
		expect(mocks.fetcher).toHaveBeenCalledTimes(1);
		expect(mocks.fetcher.mock.calls[0][1]).toBe(
			"user/templates?limit=100&offset=0&language=de",
		);
		expect(onCoverage).toHaveBeenCalledWith(
			expect.objectContaining({
				complete: false,
				warning: expect.stringContaining("first 100 memberships"),
			}),
		);
		expect(mocks.injectDataFunction).not.toHaveBeenCalled();
		expect(host.backgroundTaskHandler).not.toHaveBeenCalled();
	});

	test("retains local partial metadata when the owned remote request fails", async () => {
		mocks.invoke.mockResolvedValue([entry("cached")]);
		mocks.fetcher.mockRejectedValue(new Error("owned unavailable"));
		await expect(
			new TemplateState(backend() as never).getTemplates(undefined, undefined, {
				strict: true,
				readOnly: true,
			}),
		).rejects.toMatchObject({
			name: "TemplateMetadataReadError",
			message: "Owned remote template metadata: owned unavailable",
			partialTemplates: [entry("cached")],
		});
	});

	test("retains remote partial metadata when native listing fails", async () => {
		mocks.invoke.mockRejectedValue(new Error("local unavailable"));
		mocks.fetcher.mockResolvedValue([entry("hosted")]);
		const result = new TemplateState(backend() as never).getTemplates(
			undefined,
			undefined,
			{ strict: true, readOnly: true },
		);
		await expect(result).rejects.toBeInstanceOf(TemplateMetadataReadError);
		await expect(result).rejects.toMatchObject({
			partialTemplates: [entry("hosted")],
		});
	});

	test("bounds retained metadata and does not infer completion from an empty remote page", async () => {
		mocks.invoke.mockResolvedValue(
			Array.from({ length: MAX_OWNED_TEMPLATE_METADATA + 1 }, (_, index) =>
				entry(String(index)),
			),
		);
		mocks.fetcher.mockResolvedValue([]);
		const onCoverage = vi.fn();
		const result = await new TemplateState(backend() as never).getTemplates(
			undefined,
			undefined,
			{ strict: true, readOnly: true, onCoverage },
		);
		expect(result).toHaveLength(MAX_OWNED_TEMPLATE_METADATA);
		expect(onCoverage).toHaveBeenCalledWith(
			expect.objectContaining({
				complete: false,
				warning: expect.stringContaining("at most 1000"),
			}),
		);
		expect(mocks.fetcher).toHaveBeenCalledTimes(1);
	});
});
