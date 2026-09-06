import { describe, expect, test } from "vitest";

import {
	composeAppCapabilityFragments,
	instantiateAppCapability,
	listAppCapabilities,
	lookupAppCapability,
} from "./capabilities";
import { compileAppSpec } from "./compiler";

const titleField = {
	name: "title",
	type: "string" as const,
	nullable: false,
};

function crudParameters(namespace = "tasks") {
	return {
		namespace,
		entity_name: "task",
		table_name: `${namespace.replace(/\./g, "_")}_records`,
		page_name: "Tasks",
		route: `/${namespace.replace(/\./g, "-")}`,
		fields: [titleField],
		sample_record: { title: "Draft" },
		sample_update: { title: "Ready" },
	};
}

function approvalParameters(namespace = "reviews") {
	return {
		namespace,
		request_name: "content review",
		requests_table_name: `${namespace}_requests`,
		audit_table_name: `${namespace}_audit`,
		page_name: "Reviews",
		route: `/${namespace}`,
		request_fields: [titleField],
		sample_request: { title: "Launch copy" },
		requester_id: "requester-fixture",
		approver_id: "approver-fixture",
	};
}

describe("app capability registry", () => {
	test("pins exact versions to stable content fingerprints", () => {
		expect(listAppCapabilities()).toMatchObject([
			{
				id: "crud.foundation",
				version: "1.0.0",
				fingerprint: "fp1:10dc499c28ecf74e204ea76cb0b300cc",
				certification: "scaffold_only",
			},
			{
				id: "approval.foundation",
				version: "1.0.0",
				fingerprint: "fp1:d277dd4b78b6c507674f536caad14891",
				certification: "scaffold_only",
			},
		]);
		expect(() => lookupAppCapability("crud.foundation", "1")).toThrow(
			"version must be exact",
		);
		expect(() => lookupAppCapability("crud.foundation", "2.0.0")).toThrow(
			"Unknown app capability",
		);
	});

	test("instantiates a compilable CRUD scaffold with meaningful runtime evidence", () => {
		const recipe = lookupAppCapability("crud.foundation", "1.0.0");
		const fragment = instantiateAppCapability(
			"crud.foundation",
			"1.0.0",
			crudParameters(),
			recipe.fingerprint,
		);
		const lifecycle = fragment.scenarios.find((scenario) =>
			scenario.id.endsWith("crud_lifecycle"),
		);
		expect(lifecycle).toBeDefined();
		for (const step of lifecycle?.steps ?? []) {
			const assertions = lifecycle?.assertions.filter(
				(assertion) => assertion.step_id === step.id,
			);
			expect(assertions?.some(({ kind }) => kind === "run_outcome")).toBe(true);
			expect(assertions?.some(({ kind }) => kind === "state")).toBe(true);
			expect(
				assertions?.some(
					(assertion) =>
						assertion.kind === "run_outcome" &&
						assertion.status_path === "/status",
				),
			).toBe(false);
		}
		const spec = composeAppCapabilityFragments({ name: "Task app" }, [
			fragment,
		]);
		expect(
			compileAppSpec(spec, { app_id: "app-fixture", build_id: "build-fixture" })
				.scenarios,
		).toHaveLength(2);
		expect(fragment.generation.exact_template_source).toBe(false);
	});

	test("instantiates and composes approval while rejecting stale fingerprints", () => {
		expect(() =>
			instantiateAppCapability(
				"approval.foundation",
				"1.0.0",
				approvalParameters(),
				"fp1:stale",
			),
		).toThrow("fingerprint mismatch");

		const approval = instantiateAppCapability(
			"approval.foundation",
			"1.0.0",
			approvalParameters(),
		);
		const crud = instantiateAppCapability(
			"crud.foundation",
			"1.0.0",
			crudParameters("tasks"),
		);
		const spec = composeAppCapabilityFragments({ name: "Operations" }, [
			crud,
			approval,
		]);
		expect(
			compileAppSpec(spec, { app_id: "app-fixture", build_id: "build-fixture" })
				.resources,
		).toHaveLength(11);
		expect(
			approval.scenarios
				.flatMap((scenario) => scenario.assertions)
				.some(
					(assertion) =>
						assertion.kind === "state" &&
						assertion.path === "/idempotent_replay",
				),
		).toBe(true);
	});

	test("validates typed sample parameters before emitting resources", () => {
		expect(() =>
			instantiateAppCapability("crud.foundation", "1.0.0", {
				...crudParameters(),
				sample_record: { title: 42 },
			}),
		).toThrow();
	});
});
