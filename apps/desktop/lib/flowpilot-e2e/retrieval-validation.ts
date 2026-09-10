import { extractFlowScriptWorkspaceSymbols } from "@flow-like/flow-like-ui/lib/flowpilot/workspace-symbols";
import type {
	FlowPilotAppCreationSnapshot,
	FlowPilotE2ECheck,
	FlowPilotE2ERunReport,
} from "./types";

interface Token {
	kind: "string" | "word" | "number" | "punct";
	value: string;
}
/** Lexical evidence only. The existing native compiler checks syntax and catalog semantics. */
function tokens(source: string): Token[] {
	const result: Token[] = [];
	for (let i = 0; i < source.length; ) {
		const ch = source[i];
		if (/\s/.test(ch)) {
			i++;
			continue;
		}
		if (source.startsWith("//", i)) {
			const end = source.indexOf("\n", i);
			i = end < 0 ? source.length : end;
			continue;
		}
		if (source.startsWith("/*", i)) {
			const end = source.indexOf("*/", i + 2);
			i = end < 0 ? source.length : end + 2;
			continue;
		}
		if (ch === '"' || ch === "'" || ch === "`") {
			const quote = ch;
			let value = "";
			i++;
			while (i < source.length && source[i] !== quote) {
				if (source[i] === "\\") {
					value += source.slice(i, i + 2);
					i += 2;
				} else {
					value += source[i];
					i++;
				}
			}
			i++;
			// A template expression cannot be used as a policy literal or as a static call edge.
			if (!value.includes("${")) result.push({ kind: "string", value });
			continue;
		}
		const word = source.slice(i).match(/^[\p{L}_$][\p{L}\p{N}_$]*/u)?.[0];
		if (word) {
			result.push({ kind: "word", value: word });
			i += word.length;
			continue;
		}
		const number = source.slice(i).match(/^\d+(?:\.\d+)?/)?.[0];
		if (number) {
			result.push({ kind: "number", value: number });
			i += number.length;
			continue;
		}
		result.push({ kind: "punct", value: ch });
		i++;
	}
	return result;
}
function hasPolicy(body: Token[]) {
	return (
		["cobalt-response", "amber-review", "general-desk"].every((value) =>
			body.some((token) => token.kind === "string" && token.value === value),
		) &&
		["17", "731", "2880"].every((value) =>
			body.some((token) => token.kind === "number" && token.value === value),
		)
	);
}
function invokes(body: Token[], name: string) {
	const expected = tokens(name);
	return body.some(
		(_token, index) =>
			![".", ":"].includes(body[index - 1]?.value ?? "") &&
			expected.every(
				(token, offset) =>
					body[index + offset]?.kind === token.kind &&
					body[index + offset]?.value === token.value,
			) &&
			body[index + expected.length]?.value === "(",
	);
}
function check(
	code: string,
	passed: boolean,
	message: string,
): FlowPilotE2ECheck {
	return {
		code: `retrieval.structural.${code}`,
		status: passed ? "pass" : "fail",
		message,
		expected: true,
		actual: passed,
	};
}
function record(value: unknown): Record<string, unknown> {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}

/** Static policy reachability and schema assertions do not certify runtime outputs or precedence. */
export function validateRetrievalStructure(
	snapshot: FlowPilotAppCreationSnapshot,
	tableSchema: unknown,
): FlowPilotE2ECheck[] {
	const page = snapshot.pages.find(
		(page) =>
			page.name === "intake_console" || page.semanticAlias === "intake_console",
	);
	let helperFound = false;
	let reachableFromBoundEvent = false;
	for (const board of snapshot.boards) {
		const source = board.flowScript ?? "";
		const parsed = extractFlowScriptWorkspaceSymbols(source);
		if (!parsed.complete) continue;
		const symbols = parsed.symbols
			.filter((symbol) => symbol.kind === "function" || symbol.kind === "event")
			.map((symbol) => {
				const all = tokens(source.slice(symbol.start, symbol.end));
				const body = all.slice(
					all.findIndex(
						(token) => token.value === "{" && token.kind === "punct",
					) + 1,
				);
				return { ...symbol, body };
			});
		const policies = symbols.filter(
			(symbol) => symbol.kind === "function" && hasPolicy(symbol.body),
		);
		helperFound ||= policies.length > 0;
		if (!page?.boardId || page.boardId !== board.id) continue;
		const entries = symbols.filter(
			(symbol) =>
				symbol.kind === "event" &&
				symbol.anchor?.kind === "node" &&
				snapshot.events.some(
					(event) =>
						event.boardId === board.id && event.nodeId === symbol.anchor?.id,
				),
		);
		const visited = new Set<string>();
		const pending = [...entries];
		while (pending.length) {
			const current = pending.pop();
			if (!current || visited.has(current.qualifiedName)) continue;
			visited.add(current.qualifiedName);
			for (const callee of symbols.filter(
				(symbol) => symbol.kind === "function",
			)) {
				const localModule = current.qualifiedName
					.split("::")
					.slice(0, -1)
					.join("::");
				const calleeModule = callee.qualifiedName
					.split("::")
					.slice(0, -1)
					.join("::");
				if (
					invokes(current.body, callee.qualifiedName) ||
					(localModule === calleeModule && invokes(current.body, callee.name))
				)
					pending.push(callee);
			}
		}
		reachableFromBoundEvent ||= policies.some((policy) =>
			visited.has(policy.qualifiedName),
		);
	}
	const fields = Array.isArray(record(tableSchema).fields)
		? (record(tableSchema).fields as unknown[])
		: [];
	const hasField = (name: string, accepted: RegExp) =>
		fields.some((value) => {
			const field = record(value);
			const raw = field.data_type ?? field.type;
			const type =
				typeof raw === "string" ? raw : (Object.keys(record(raw))[0] ?? "");
			return field.name === name && accepted.test(type);
		});
	return [
		check(
			"policy_helper",
			helperFound,
			"A canonical helper contains the source policy's exact queue labels and integer deadlines.",
		),
		check(
			"policy_reachable",
			reachableFromBoundEvent,
			"The policy helper is statically called from an app-registered Event on intake_console's owning board.",
		),
		check(
			"table_fields",
			hasField("summary", /^(?:string|utf8|largeutf8|utf8view)$/i) &&
				hasField("queue", /^(?:string|utf8|largeutf8|utf8view)$/i) &&
				hasField("response_minutes", /^(?:u?int(?:8|16|32|64)?|integer)$/i),
			"The authoritative intake_tickets schema has string summary and queue fields plus integer response_minutes.",
		),
	];
}
export function appendRetrievalChecks(
	report: FlowPilotE2ERunReport,
	extra: FlowPilotE2ECheck[],
): FlowPilotE2ERunReport {
	const checks = [...report.checks, ...extra];
	const failures = checks.filter((check) => check.status === "fail");
	return {
		...report,
		checks,
		failures,
		passed: report.passed && failures.length === 0,
		summary: {
			checks: checks.length,
			passed: checks.length - failures.length,
			failed: failures.length,
		},
	};
}
