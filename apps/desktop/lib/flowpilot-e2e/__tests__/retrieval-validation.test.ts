import { describe, expect, it } from "vitest";
import { validateRetrievalStructure } from "../retrieval-validation";
import type { FlowPilotAppCreationSnapshot } from "../types";
const helper = `function routeRecord(summary: string): (queue: string, minutes: int) {
 let queue = "general-desk"
 let minutes = 2880
 if (summary == "service interruption") { queue = "cobalt-response"; minutes = 17 }
 else if (summary == "refund") { queue = "amber-review"; minutes = 731 }
 return queue, minutes
}`;
function snapshot(
	source = `${helper}\neventsGeneric intake(summary: string) { //@n:intake-node
 const { queue, minutes } = routeRecord(summary)
}`,
): FlowPilotAppCreationSnapshot {
	return {
		appId: "target",
		appName: "Target",
		boards: [{ id: "board", name: "Intake", flowScript: source }],
		pages: [{ id: "page", name: "intake_console", boardId: "board" }],
		events: [
			{ id: "event", name: "Intake", boardId: "board", nodeId: "intake-node" },
		],
		tables: ["intake_tickets"],
		widgets: [],
	};
}
const schema = {
	fields: [
		{ name: "summary", data_type: "Utf8" },
		{ name: "queue", data_type: "Utf8" },
		{ name: "response_minutes", data_type: "Int64" },
	],
};
function status(
	value: FlowPilotAppCreationSnapshot,
	inputSchema: unknown = schema,
) {
	return Object.fromEntries(
		validateRetrievalStructure(value, inputSchema).map((check) => [
			check.code,
			check.status,
		]),
	);
}
describe("retrieval structural acceptance", () => {
	it("requires a real bound Event path to the helper and typed persisted fields", () => {
		expect(Object.values(status(snapshot()))).toEqual(["pass", "pass", "pass"]);
	});
	it("does not credit an unused helper or a function name inside a string", () => {
		for (const call of ["", 'const text = "routeRecord(summary)"']) {
			const result = status(
				snapshot(
					`${helper}\neventsGeneric intake(summary: string) { //@n:intake-node\n ${call}\n}`,
				),
			);
			expect(result["retrieval.structural.policy_helper"]).toBe("pass");
			expect(result["retrieval.structural.policy_reachable"]).toBe("fail");
		}
	});
	it("follows an actual intermediate helper call", () => {
		const value = snapshot(
			`${helper}\nfunction wrapper(summary: string): (queue: string, minutes: int) { return routeRecord(summary) }\neventsGeneric intake(summary: string) { //@n:intake-node\n const result = wrapper(summary)\n}`,
		);
		expect(status(value)["retrieval.structural.policy_reachable"]).toBe("pass");
	});
	it("rejects an Event on the wrong board or an unregistered Event anchor", () => {
		const wrongPage = snapshot();
		wrongPage.pages[0].boardId = "other";
		expect(status(wrongPage)["retrieval.structural.policy_reachable"]).toBe(
			"fail",
		);
		const wrongEvent = snapshot();
		wrongEvent.events[0].nodeId = "other";
		expect(status(wrongEvent)["retrieval.structural.policy_reachable"]).toBe(
			"fail",
		);
	});
	it("does not accept policy values found only in comments, a combined string or anchor-like numbers", () => {
		for (const body of [
			'// "cobalt-response" "amber-review" "general-desk" 17 731 2880\nreturn "nothing", 1',
			'const x = "cobalt-response amber-review general-desk 17 731 2880"\nreturn "nothing", 1',
		]) {
			const value = snapshot(
				`function routeRecord(summary: string): (queue: string, minutes: int) { ${body} }\neventsGeneric intake(summary: string) { //@n:intake-node\n const result = routeRecord(summary)\n}`,
			);
			expect(status(value)["retrieval.structural.policy_helper"]).toBe("fail");
		}
	});
	it("fails when the schema is unreadable or the deadline is a string", () => {
		expect(
			status(snapshot(), undefined)["retrieval.structural.table_fields"],
		).toBe("pass");
		expect(status(snapshot(), null)["retrieval.structural.table_fields"]).toBe(
			"fail",
		);
		const wrong = {
			fields: schema.fields.map((field) => ({ ...field, data_type: "Utf8" })),
		};
		expect(status(snapshot(), wrong)["retrieval.structural.table_fields"]).toBe(
			"fail",
		);
	});
});
