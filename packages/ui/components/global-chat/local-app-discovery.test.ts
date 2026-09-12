import { describe, expect, test } from "bun:test";
import {
	entersPrivateContext,
	isReturnedAppInventory,
	sealedResearchRefusal,
} from "./local-app-discovery";

const RUN = "run-1";
const state = (
	overrides: Partial<Parameters<typeof sealedResearchRefusal>[1]> = {},
) => ({
	appInventoryReturned: false,
	privateContextEntered: false,
	sealedResearchUsed: false,
	...overrides,
});
const code = (...args: Parameters<typeof sealedResearchRefusal>) =>
	sealedResearchRefusal(...args)?.code;

describe("isReturnedAppInventory", () => {
	test("a clean inventory opens the public fallback", () => {
		expect(
			isReturnedAppInventory({ status: "ok", complete: true, apps: [] }),
		).toBe(true);
	});

	test("a partial inventory still counts as discovery", () => {
		const partials = [
			{ status: "partial", complete: false, apps: [] },
			{ status: "partial", complete: false, truncated: true, apps: [] },
			{
				status: "partial",
				complete: false,
				missing_profile_app_count: 2,
				apps: [{ app_id: "a", events_status: "error" }],
			},
		];
		for (const inventory of partials) {
			expect(isReturnedAppInventory(inventory)).toBe(true);
		}
	});

	test("a failed or absent listing keeps the fallback shut", () => {
		expect(isReturnedAppInventory({ status: "error", message: "no" })).toBe(
			false,
		);
		expect(isReturnedAppInventory({ status: "timeout" })).toBe(false);
		expect(isReturnedAppInventory({ apps: [] })).toBe(false);
		expect(isReturnedAppInventory(undefined)).toBe(false);
		expect(isReturnedAppInventory(null)).toBe(false);
		expect(isReturnedAppInventory("ok")).toBe(false);
	});
});

describe("entersPrivateContext", () => {
	test("discovery and the researcher itself carry no private data", () => {
		expect(entersPrivateContext("list_apps")).toBe(false);
		expect(entersPrivateContext("research_agent")).toBe(false);
	});

	test("everything else is private, including unknown tools", () => {
		for (const tool of [
			"call_app_chat",
			"call_app_event",
			"data_studio_agent",
			"open_app_page",
			"_memory_search",
			"a_tool_added_next_year",
		]) {
			expect(entersPrivateContext(tool)).toBe(true);
		}
	});
});

describe("sealedResearchRefusal", () => {
	test("a plainly public request researches in the first wave", () => {
		expect(code(RUN, undefined)).toBeUndefined();
		expect(code(RUN, state())).toBeUndefined();
	});

	test("a run already holding private data must list apps first", () => {
		expect(code(RUN, state({ privateContextEntered: true }))).toBe(
			"local_app_discovery_required",
		);
		expect(
			code(
				RUN,
				state({ privateContextEntered: true, appInventoryReturned: true }),
			),
		).toBeUndefined();
	});

	test("the researcher is one-shot per run", () => {
		expect(code(RUN, state({ sealedResearchUsed: true }))).toBe(
			"sealed_research_already_used",
		);
	});

	test("a run with no id is refused so the one-shot cap stays enforceable", () => {
		expect(code(undefined, state())).toBe("sealed_research_unavailable");
		expect(code("  ", state())).toBe("sealed_research_unavailable");
	});
});
