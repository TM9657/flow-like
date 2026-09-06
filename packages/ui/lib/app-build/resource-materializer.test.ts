import { expect, test } from "vitest";
import { compileAppSpec } from "./compiler";
import { appResourceInstruction } from "./resource-materializer";
import { exampleAppSpec } from "./test-fixtures";

test("repair briefs retain the full contract and exact symbols alongside host diagnostics", () => {
	const plan = compileAppSpec(exampleAppSpec(), {
		app_id: "app",
		build_id: "build",
	});
	const resource = plan.resources.find((item) => item.kind === "board")!;
	const instruction = appResourceInstruction(plan, resource, [
		"Missing field title in persisted insert payload.",
	]);
	expect(instruction).toContain("Validate the payload and persist the issue.");
	expect(instruction).toContain(resource.physical_id);
	expect(instruction).toContain("capture_issue");
	expect(instruction).toContain(
		"Missing field title in persisted insert payload.",
	);
	expect(instruction).toContain(
		"do not authorize extra actions or reduced scope",
	);
});
