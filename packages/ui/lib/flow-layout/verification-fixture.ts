import type { INode } from "../schema/flow/node";
import { IVariableType } from "../schema/flow/node";
import { GraphBuilder } from "./test-fixtures";
import type { AutoLayoutInput } from "./types";

function namePin(
	node: INode,
	suffix: string,
	name: string,
	dataType = IVariableType.String,
) {
	const pin = node.pins[`${node.id}:${suffix}`];
	pin.name = name;
	pin.friendly_name = name;
	pin.data_type = dataType;
}

/** The Google verification example with Drive collapsed into its initial group. */
export function googleVerificationFixture(): AutoLayoutInput {
	const graph = new GraphBuilder();
	graph.exec("runGoogleVerification", { execIn: false, start: true });
	const provider = graph.pure("google-provider", { dataIns: 0 });
	provider.friendly_name = "Google";
	namePin(provider, "out-0", "Provider", IVariableType.Struct);
	const initialGroup = graph.exec("format-label-and-drive", {
		dataIns: 1,
		dataOuts: 2,
	});
	initialGroup.friendly_name = "Format String group";
	namePin(initialGroup, "in-0", "Provider", IVariableType.Struct);
	namePin(initialGroup, "out-0", "Formatted");
	namePin(initialGroup, "out-1", "passed", IVariableType.Boolean);
	graph.execLink("runGoogleVerification", initialGroup.id);
	graph.dataLink(provider.id, initialGroup.id);

	const services = [
		"Gmail",
		"Calendar",
		"Docs",
		"Sheets",
		"Slides",
		"Forms",
		"YouTube",
	];
	const results = [{ id: initialGroup.id, output: 1, label: "drivePassed" }];
	let previous = initialGroup.id;
	for (const service of services) {
		const node = graph.exec(`verify${service}`, {
			dataIns: service === "YouTube" ? 1 : 2,
			dataOuts: 1,
		});
		node.name = "control_call_function";
		node.friendly_name = `Call verify${service}`;
		namePin(node, "in-0", "provider", IVariableType.Struct);
		namePin(node, "out-0", "passed", IVariableType.Boolean);
		graph.execLink(previous, node.id);
		graph.dataLink(provider.id, node.id);
		if (service !== "YouTube") {
			namePin(node, "in-1", "label");
			graph.dataLink(initialGroup.id, node.id, 0, 1);
		}
		results.push({
			id: node.id,
			output: 0,
			label: `${service.toLowerCase()}Passed`,
		});
		previous = node.id;
	}

	const report = graph.pure("results-format", { dataIns: 9 });
	report.friendly_name = "Format String";
	namePin(report, "in-0", "Input");
	namePin(report, "out-0", "Formatted");
	for (const [index, result] of results.entries()) {
		namePin(report, `in-${index + 1}`, result.label, IVariableType.Boolean);
		graph.dataLink(result.id, report.id, result.output, index + 1);
	}
	const printResults = graph.exec("print-results", { dataIns: 2 });
	printResults.friendly_name = "Print Info";
	namePin(printResults, "in-0", "Message");
	namePin(printResults, "in-1", "On Screen?", IVariableType.Boolean);
	graph.execLink(previous, printResults.id);
	graph.dataLink(report.id, printResults.id);

	let conjunction = results[0];
	for (let index = 1; index < results.length; index++) {
		const and = graph.pure(`all-passed-${index}`, { dataIns: 2 });
		and.friendly_name = "And";
		namePin(and, "in-0", "Boolean A", IVariableType.Boolean);
		namePin(and, "in-1", "Boolean B", IVariableType.Boolean);
		namePin(and, "out-0", "Result", IVariableType.Boolean);
		graph.dataLink(conjunction.id, and.id, conjunction.output, 0);
		graph.dataLink(results[index].id, and.id, results[index].output, 1);
		conjunction = { id: and.id, output: 0, label: "allPassed" };
	}
	const branch = graph.exec("results-branch", {
		execOuts: ["true", "false"],
		dataIns: 1,
	});
	branch.friendly_name = "Branch";
	namePin(branch, "in-0", "Condition", IVariableType.Boolean);
	graph.execLink(printResults.id, branch.id);
	graph.dataLink(conjunction.id, branch.id);
	for (const [id, title, output] of [
		["verification-success", "Print Info", "true"],
		["verification-warning", "Log Warning", "false"],
	]) {
		const node = graph.exec(id, { dataIns: 2 });
		node.friendly_name = title;
		namePin(node, "in-0", "Message");
		namePin(node, "in-1", "On Screen?", IVariableType.Boolean);
		graph.execLink(branch.id, id, output);
	}

	const verificationNodes = new Set(graph.nodes.keys());
	graph.exec("readFormResponses", {
		execIn: false,
		start: true,
		coordinates: [0, 500, 0],
	});
	const formId = graph.pure("demo-form-id", { dataIns: 0 });
	formId.friendly_name = "Get demoFormId";
	const isEmpty = graph.pure("form-id-empty", { dataIns: 3 });
	isEmpty.friendly_name = "== (String)";
	namePin(isEmpty, "out-0", "Is Equal?", IVariableType.Boolean);
	graph.dataLink(formId.id, isEmpty.id);
	const formBranch = graph.exec("form-responses-branch", {
		execOuts: ["true", "false"],
		dataIns: 1,
	});
	formBranch.friendly_name = "Branch";
	namePin(formBranch, "in-0", "Condition", IVariableType.Boolean);
	graph.execLink("readFormResponses", formBranch.id);
	graph.dataLink(isEmpty.id, formBranch.id);
	const missingForm = graph.exec("missing-form-error", { dataIns: 2 });
	missingForm.friendly_name = "Log Error";
	graph.execLink(formBranch.id, missingForm.id, "true");
	const responses = graph.exec("list-form-responses", { dataOuts: 1 });
	responses.friendly_name = "List Form Responses group";
	graph.execLink(formBranch.id, responses.id, "false");
	const printResponses = graph.exec("print-responses", { dataIns: 2 });
	printResponses.friendly_name = "Print Info";
	graph.execLink(responses.id, printResponses.id);
	graph.dataLink(responses.id, printResponses.id);
	for (const node of graph.nodes.values()) {
		if (!verificationNodes.has(node.id)) node.coordinates = [0, 500, 0];
	}

	return graph.build();
}
