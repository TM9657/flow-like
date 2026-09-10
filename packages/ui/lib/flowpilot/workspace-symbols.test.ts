import { describe, expect, it } from "vitest";
import {
	MAX_WORKSPACE_FLOWSCRIPT_BYTES,
	extractFlowScriptWorkspaceSymbols,
} from "./workspace-symbols";

describe("FlowScript workspace symbol extraction", () => {
	it("returns exact nested helper, event and interface spans with qualified names", () => {
		const source = `use ui::{ setText, navigateTo as navigate }
interface Invoice {
    id: string;
    details: { label: string; };
}
const state: Map<string, Invoice> = {}
module billing { //@l:billing-layer
    // Fetch invoices for the current customer.
    @cache({ ttl_ms: 1000 })
    function loadInvoices(customer: string): (invoices: Invoice[]) { //@l:load-layer
        const response = http::get({ url: customer })
        if (response) { return response }
        return []
    }
    module actions {
        eventsSimple refreshInvoices() { //@n:refresh-node
            billing::loadInvoices({ customer: "current" })
        }
    }
}
onStart() { log::info({ message: "ready" }) }
`;
		const result = extractFlowScriptWorkspaceSymbols(source);
		expect(result.complete).toBe(true);
		expect(result.validation).toBe("declaration_structure");
		expect(
			result.symbols.map((symbol) => [symbol.qualifiedName, symbol.kind]),
		).toEqual([
			["Invoice", "interface"],
			["billing", "module"],
			["billing::loadInvoices", "function"],
			["billing::actions", "module"],
			["billing::actions::refreshInvoices", "event"],
			["onStart", "event"],
		]);
		const helper = result.symbols[2];
		expect(helper).toMatchObject({
			signature:
				"function loadInvoices(customer: string): (invoices: Invoice[])",
			startLine: 8,
			endLine: 14,
			anchor: { kind: "layer", id: "load-layer" },
		});
		expect(
			source.slice(helper.start, helper.end),
		).toBe(`// Fetch invoices for the current customer.
    @cache({ ttl_ms: 1000 })
    function loadInvoices(customer: string): (invoices: Invoice[]) { //@l:load-layer
        const response = http::get({ url: customer })
        if (response) { return response }
        return []
    }`);
		expect(result.symbols[4].anchor).toEqual({
			kind: "node",
			id: "refresh-node",
		});
	});

	it("does not manufacture declarations or anchors from comments, strings or templates", () => {
		const source = [
			"// function fakeComment() {",
			'const text = "function fakeString() { //@l:fake"',
			"function actual(): (value: string) {",
			'  const quoted = "} \\" function counterfeit() {"',
			'  const template = `} function fakeTemplate() { ${string::format({ value: `nested ${"}"}` })}`',
			"  // } function fakeBodyComment() { //@l:counterfeit",
			"  return template",
			"}",
			"eventsSimple realEvent() {}",
		].join("\n");
		const result = extractFlowScriptWorkspaceSymbols(source);
		expect(result.complete).toBe(true);
		expect(result.symbols.map((symbol) => symbol.qualifiedName)).toEqual([
			"actual",
			"realEvent",
		]);
		expect(result.symbols[0].anchor).toBeUndefined();
		expect(source.slice(result.symbols[0].start, result.symbols[0].end)).toBe(
			source.slice(
				source.indexOf("function actual"),
				source.indexOf("\neventsSimple"),
			),
		);
	});

	it("keeps module and detached contextual event names", () => {
		const result = extractFlowScriptWorkspaceSymbols(
			"module() {} detached() {} detached { functionCall({ x: 1 }) }",
		);
		expect(result.complete).toBe(true);
		expect(
			result.symbols.map((symbol) => [symbol.qualifiedName, symbol.kind]),
		).toEqual([
			["module", "event"],
			["detached", "event"],
		]);
	});

	it("accepts Unicode names and reports UTF-16 offsets with CRLF source lines", () => {
		const source =
			'// 😀 heading\r\nconst café: string = "☕"\r\nmodule büro {\r\n function résumé(): (total: int) { return 3 }\r\n}\r\n';
		const result = extractFlowScriptWorkspaceSymbols(source);
		expect(result.complete).toBe(true);
		const helper = result.symbols[1];
		expect(helper.qualifiedName).toBe("büro::résumé");
		expect(helper.start).toBe(source.indexOf("function résumé"));
		expect(helper.startLine).toBe(4);
		expect(helper.endLine).toBe(4);
	});

	it("accepts canonical root declarations, typed containers and multi-line literals", () => {
		const source = `use string::*, data::db as db
use ui::{ setText, navigateTo as navigate, }
@description("Shared state")
let state: Map<string, geometry<Point>> = {}
const title = "line one
line two"
function transform(points: Set<geometry<Point>>, rows: Row[], count: int): (result: Row[]) {
  return rows
}
`;
		const result = extractFlowScriptWorkspaceSymbols(source);
		expect(result.complete).toBe(true);
		expect(result.symbols).toHaveLength(1);
		expect(result.symbols[0].name).toBe("transform");
	});

	it("does not attach trailing anchors or previous declaration documentation to the next symbol", () => {
		const source = `function first() { log::info({ message: "hi" }) } //@l:unowned
//@l:standalone
function second() {
  //@l:body-comment
}
// Third helper.
function third() {}
`;
		const result = extractFlowScriptWorkspaceSymbols(source);
		expect(result.complete).toBe(true);
		expect(result.symbols[1].start).toBe(source.indexOf("function second"));
		expect(result.symbols.every((symbol) => !symbol.anchor)).toBe(true);
		expect(source.slice(result.symbols[2].start, result.symbols[2].end)).toBe(
			"// Third helper.\nfunction third() {}",
		);
	});

	it("permits the same local helper name in distinct module paths", () => {
		const result = extractFlowScriptWorkspaceSymbols(
			"module a { function load() {} } module b { function load() {} }",
		);
		expect(result.complete).toBe(true);
		expect(
			result.symbols
				.filter((symbol) => symbol.kind === "function")
				.map((symbol) => symbol.qualifiedName),
		).toEqual(["a::load", "b::load"]);
	});

	it.each([
		["function load() {} function load() {}", "ambiguous_symbol"],
		["module a {} module a {}", "ambiguous_symbol"],
		[
			"function a() { //@l:same\n} function b() { //@l:same\n}",
			"ambiguous_anchor",
		],
		["function complete() {} function truncated() {", "unbalanced_delimiter"],
		["function broken() { call([)] }", "unbalanced_delimiter"],
		[
			'function broken() { const value = "unterminated }',
			"unterminated_literal",
		],
		[
			"function broken() { const value = `unterminated ${value} }",
			"unterminated_literal",
		],
		[
			"function broken() { const value = `oops ${value` }",
			"unterminated_literal",
		],
		["function broken() { const value = `oops ${}` }", "invalid_template"],
		['function broken() { const value = "\\q" }', "invalid_escape"],
		["function wrong(param) {}", "invalid_declaration"],
		["function // interrupted header\n wrong() {}", "invalid_declaration"],
		[
			'@cache("x") // interrupted decorator\n function wrong() {}',
			"invalid_declaration",
		],
		["function wrong(): void {}", "invalid_declaration"],
		["module wrong { interface Local { id: string } }", "invalid_declaration"],
		["module wrong { const state = 1 }", "invalid_declaration"],
		["function good() {} unexpected leftovers", "invalid_declaration"],
		["/* function fake() {} */ function real() {}", "unsupported_syntax"],
	])("rejects unreliable symbol extraction for %s", (source, code) => {
		const result = extractFlowScriptWorkspaceSymbols(source);
		expect(result.complete).toBe(false);
		expect(result.symbols).toEqual([]);
		expect(result.diagnostics[0].code).toBe(code);
		expect(result.diagnostics[0].line).toBeGreaterThan(0);
	});

	it("bounds bytes, nesting, token volume and symbol count", () => {
		const sources = [
			[`//${"é".repeat(MAX_WORKSPACE_FLOWSCRIPT_BYTES / 2)}`, "source_limit"],
			["module a {".repeat(129) + "}".repeat(129), "nesting_limit"],
			[";".repeat(131_073), "token_limit"],
			[
				Array.from(
					{ length: 2_049 },
					(_, index) => `function f${index}() {}`,
				).join("\n"),
				"symbol_limit",
			],
		];
		for (const [source, code] of sources) {
			const result = extractFlowScriptWorkspaceSymbols(source);
			expect(result.complete).toBe(false);
			expect(result.symbols).toEqual([]);
			expect(result.diagnostics[0].code).toBe(code);
		}
	});

	it("distinguishes an empty valid document from failed indexing", () => {
		expect(extractFlowScriptWorkspaceSymbols("// No workflows yet.\n")).toEqual(
			{
				symbols: [],
				complete: true,
				diagnostics: [],
				validation: "declaration_structure",
			},
		);
	});
});
