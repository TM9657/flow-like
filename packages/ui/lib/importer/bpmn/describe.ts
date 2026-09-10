import type {
	BpmnEventDefinition,
	BpmnFlowNode,
	BpmnProcess,
} from "../bpmn-model";

/**
 * How an imported element describes itself on the board. Layer names and node
 * comments are the only place BPMN wording survives the paste, so everything
 * the file said about an element is folded into them.
 */

export function label(node: BpmnFlowNode): string {
	return node.name?.trim() || humanKind(node.kind);
}

export function humanKind(kind: string): string {
	return kind
		.replace(/([A-Z])/g, " $1")
		.replace(/^./, (c) => c.toUpperCase())
		.trim();
}

export function describeProcess(process: BpmnProcess): string {
	const lines = [`BPMN process ${process.id}`];
	if (process.participantName) lines.push(`Pool: ${process.participantName}`);
	if (process.documentation) lines.push(process.documentation);
	return lines.join("\n");
}

export function describeNode(node: BpmnFlowNode, extra: string[] = []): string {
	const lines = [`BPMN ${humanKind(node.kind)} (${node.id})`, ...extra];
	if (node.documentation) lines.push(node.documentation);
	for (const def of node.eventDefinitions) lines.push(describeEventDef(def));
	for (const [key, value] of Object.entries(node.vendorAttrs)) {
		lines.push(`${key} = ${value}`);
	}
	for (const ext of node.extensions) {
		const attrs = Object.entries(ext.attrs)
			.map(([k, v]) => `${k}=${v}`)
			.join(" ");
		const children = ext.element.children
			.map((child) => {
				const inner = Object.entries(child.attrs)
					.map(([k, v]) => `${k}=${v}`)
					.join(" ");
				const text = child.text.trim();
				return `${child.prefix ? `${child.prefix}:` : ""}${child.name}${inner ? ` ${inner}` : ""}${text ? ` ${text}` : ""}`;
			})
			.join("; ");
		const text = ext.text ? ` ${ext.text}` : "";
		lines.push(
			`${ext.prefix ? `${ext.prefix}:` : ""}${ext.name}${attrs ? ` ${attrs}` : ""}${children ? ` { ${children} }` : ""}${text}`,
		);
	}
	if (node.ioMapping) {
		for (const io of node.ioMapping.inputs) {
			lines.push(`input ${io.target} = ${io.source}`);
		}
		for (const io of node.ioMapping.outputs) {
			lines.push(`output ${io.target} = ${io.source}`);
		}
	}
	if (node.script?.body) {
		lines.push(
			`script (${node.script.format ?? "unknown"}):\n${node.script.body}`,
		);
	}
	return lines.join("\n");
}

export function describeEventDef(def: BpmnEventDefinition): string {
	switch (def.kind) {
		case "timer": {
			const t = def.timer ?? {};
			return `timer: ${t.timeDuration ? `duration ${t.timeDuration}` : t.timeCycle ? `cycle ${t.timeCycle}` : t.timeDate ? `date ${t.timeDate}` : "unspecified"}`;
		}
		case "message":
			return `message: ${def.refName ?? def.ref ?? "unnamed"}`;
		case "signal":
			return `signal: ${def.refName ?? def.ref ?? "unnamed"}`;
		case "error":
			return `error: ${def.refName ?? def.ref ?? "any"}${def.code ? ` (code ${def.code})` : ""}`;
		case "escalation":
			return `escalation: ${def.refName ?? def.ref ?? "any"}${def.code ? ` (code ${def.code})` : ""}`;
		case "conditional":
			return `condition: ${def.condition ?? "unspecified"}`;
		case "link":
			return `link: ${def.linkName ?? "unnamed"}`;
		case "compensate":
			return `compensation${def.ref ? ` for ${def.ref}` : ""}`;
		default:
			return def.kind;
	}
}
