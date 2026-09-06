import { describe, expect, it } from "vitest";

import { compileAppSpec, validateAppSpec } from "./compiler";
import { appBuildContractGuide, MAX_APP_SPEC_BYTES } from "./contract";
import { exampleAppSpec } from "./test-fixtures";

describe("AppSpec compiler", () => {
	it("ships a valid, behaviorally testable contract example", () => {
		expect(validateAppSpec(appBuildContractGuide().example).ok).toBe(true);
	});

	it("canonicalizes ordering while keeping stable host reservations", () => {
		const original = exampleAppSpec();
		const reordered = {
			...original,
			resources: [...original.resources].reverse(),
		};
		const first = compileAppSpec(original, {
			app_id: "app_1",
			build_id: "build_1",
		});
		const second = compileAppSpec(reordered, {
			app_id: "app_1",
			build_id: "build_1",
		});

		expect(second.spec_fingerprint).toBe(first.spec_fingerprint);
		expect(second.resources.map((resource) => resource.physical_id)).toEqual(
			first.resources.map((resource) => resource.physical_id),
		);
		expect(first.waves).toEqual([["issue_flow"], ["submit_issue"]]);
		expect(
			compileAppSpec(original, { app_id: "app_1", build_id: "build_2" })
				.resources[0]?.physical_id,
		).not.toBe(first.resources[0]?.physical_id);
	});

	it("rejects unknown fields and incomplete lifecycle pairs", () => {
		const spec = exampleAppSpec();
		const unknownField = { ...spec, invented: true };
		expect(validateAppSpec(unknownField).ok).toBe(false);

		const lifecycle = {
			...spec,
			resources: [
				spec.resources[0],
				{
					key: "issue_page",
					kind: "page",
					depends_on: ["issue_flow"],
					requirement_ids: ["capture_issue"],
					config: {
						name: "Issues",
						route: "/issues",
						board: "issue_flow",
						instruction: "Render the issue form.",
						on_interval_entry: "refresh",
					},
				},
			],
		};
		const validation = validateAppSpec(lifecycle);
		expect(validation.ok).toBe(false);
		if (!validation.ok) {
			expect(
				validation.issues.some((issue) => issue.code === "invalid_schema"),
			).toBe(true);
		}
	});

	it("rejects contracts too large for a durable checkpoint", () => {
		const spec = exampleAppSpec();
		const validation = validateAppSpec({
			...spec,
			resources: Array.from({ length: 14 }, (_, index) => ({
				key: `board_${index}`,
				kind: "board",
				depends_on: [],
				requirement_ids: ["capture_issue"],
				config: {
					name: `Board ${index}`,
					instruction: "x".repeat(40_000),
				},
			})),
			scenarios: [],
		});
		expect(validation.ok).toBe(false);
		if (!validation.ok) {
			expect(validation.issues).toEqual(
				expect.arrayContaining([
					expect.objectContaining({
						code: "invalid_schema",
						message: expect.stringContaining(String(MAX_APP_SPEC_BYTES)),
					}),
				]),
			);
		}
	});

	it("models a page route and lifecycle hooks as explicit board dependencies", () => {
		const spec = exampleAppSpec();
		const pageSpec = {
			...spec,
			resources: [
				spec.resources[0],
				{
					key: "issue_page",
					kind: "page",
					depends_on: ["issue_flow"],
					requirement_ids: ["capture_issue"],
					config: {
						name: "Issues",
						route: "/issues",
						board: "issue_flow",
						instruction: "Render the issue form.",
						on_load_entry: "load",
						on_interval_entry: "refresh",
						interval_seconds: 60,
					},
				},
				{
					key: "issue_page_event",
					kind: "event",
					depends_on: ["issue_page"],
					requirement_ids: ["capture_issue"],
					config: {
						name: "Issues",
						event_type: "page",
						page: "issue_page",
						route: "/issues",
					},
				},
			],
			scenarios: [],
		};
		const validation = validateAppSpec(pageSpec);
		expect(validation.ok).toBe(true);
		const compiled = compileAppSpec(pageSpec, {
			app_id: "app_1",
			build_id: "build_1",
		});
		expect(compiled.waves).toEqual([
			["issue_flow"],
			["issue_page"],
			["issue_page_event"],
		]);
	});

	it("rejects dependency cycles and missing symbolic dependencies", () => {
		const spec = exampleAppSpec();
		const resources = spec.resources.map((resource) =>
			resource.key === "issue_flow"
				? { ...resource, depends_on: ["submit_issue"] }
				: { ...resource, depends_on: [] },
		);
		const validation = validateAppSpec({ ...spec, resources });
		expect(validation.ok).toBe(false);
		if (!validation.ok) {
			expect(validation.issues.map((issue) => issue.code)).toContain(
				"missing_symbolic_dependency",
			);
		}

		const cyclic = validateAppSpec({
			...spec,
			resources: spec.resources.map((resource) =>
				resource.key === "issue_flow"
					? { ...resource, depends_on: ["submit_issue"] }
					: resource,
			),
		});
		expect(cyclic.ok).toBe(false);
		if (!cyclic.ok) {
			expect(cyclic.issues.map((issue) => issue.code)).toContain(
				"dependency_cycle",
			);
		}
	});

	it("detects page and Event route collisions independent of resource order", () => {
		const spec = exampleAppSpec();
		const resources = [
			{
				key: "a_event",
				kind: "event",
				depends_on: ["z_page_two"],
				requirement_ids: ["capture_issue"],
				config: {
					name: "Wrong route",
					event_type: "page",
					page: "z_page_two",
					route: "/one",
				},
			},
			{
				key: "issue_flow",
				kind: "board",
				depends_on: [],
				requirement_ids: ["capture_issue"],
				config: { name: "Flow", instruction: "Return a page model." },
			},
			{
				key: "z_page_one",
				kind: "page",
				depends_on: ["issue_flow"],
				requirement_ids: ["capture_issue"],
				config: {
					name: "One",
					route: "/one",
					board: "issue_flow",
					instruction: "Render one.",
				},
			},
			{
				key: "z_page_two",
				kind: "page",
				depends_on: ["issue_flow"],
				requirement_ids: ["capture_issue"],
				config: {
					name: "Two",
					route: "/two",
					board: "issue_flow",
					instruction: "Render two.",
				},
			},
		];
		const validation = validateAppSpec({ ...spec, resources });
		expect(validation.ok).toBe(false);
		if (!validation.ok) {
			expect(validation.issues.map((issue) => issue.code)).toContain(
				"duplicate_route",
			);
		}
	});
});
