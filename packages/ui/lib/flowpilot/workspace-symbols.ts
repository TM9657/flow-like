export const MAX_WORKSPACE_FLOWSCRIPT_BYTES = 1_048_576;
const MAX_TOKENS = 131_072;
const MAX_SYMBOLS = 2_048;
const MAX_DEPTH = 128;

export interface FlowScriptWorkspaceSymbol {
	name: string;
	qualifiedName: string;
	kind: "function" | "event" | "interface" | "module";
	signature: string;
	/** UTF-16 source offsets, with an exclusive end, suitable for String.slice. */
	start: number;
	end: number;
	/** One-based inclusive source lines. */
	startLine: number;
	endLine: number;
	anchor?: { kind: "node" | "variable" | "layer"; id: string };
}

export interface FlowScriptWorkspaceSymbolDiagnostic {
	code: string;
	message: string;
	offset: number;
	line: number;
}

export interface FlowScriptWorkspaceSymbolIndex {
	symbols: FlowScriptWorkspaceSymbol[];
	complete: boolean;
	diagnostics: FlowScriptWorkspaceSymbolDiagnostic[];
	/** Body expressions and catalog compatibility still require the FlowScript compiler. */
	validation: "declaration_structure";
}

interface Token {
	text: string;
	kind: "identifier" | "literal" | "punctuation";
	start: number;
	end: number;
	line: number;
}

interface Comment {
	start: number;
	end: number;
	line: number;
}

class IndexFailure extends Error {
	constructor(readonly diagnostic: FlowScriptWorkspaceSymbolDiagnostic) {
		super(diagnostic.message);
	}
}

/**
 * Index canonical board declarations without guessing a read location from substring matches.
 * This checks lexical boundaries, balanced delimiters and declaration headers. It does not
 * replace the Rust parser's expression, interface-field or catalog validation.
 */
export function extractFlowScriptWorkspaceSymbols(
	source: string,
): FlowScriptWorkspaceSymbolIndex {
	function failure(
		code: string,
		message: string,
		offset: number,
		line: number,
	): never {
		throw new IndexFailure({ code, message, offset, line });
	}
	try {
		if (
			source.length > MAX_WORKSPACE_FLOWSCRIPT_BYTES ||
			new TextEncoder().encode(source).byteLength >
				MAX_WORKSPACE_FLOWSCRIPT_BYTES
		) {
			failure(
				"source_limit",
				"FlowScript exceeds the 1 MiB symbol index limit.",
				0,
				1,
			);
		}
		const tokens: Token[] = [];
		const comments: Comment[] = [];
		const commentsByLine = new Map<number, Comment>();
		const pairs = new Map<number, number>();
		const opens: number[] = [];
		const numberPattern = /(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?/y;
		let offset = 0;
		let line = 1;
		const advance = () => {
			if (source[offset] === "\n") line++;
			offset++;
		};
		const skipComment = (record: boolean) => {
			const start = offset;
			const commentLine = line;
			while (offset < source.length && source[offset] !== "\n") advance();
			if (record) {
				const comment = { start, end: offset, line: commentLine };
				comments.push(comment);
				commentsByLine.set(commentLine, comment);
			}
		};
		const skipLiteral = (depth: number): void => {
			const quote = source[offset];
			const start = offset;
			const startLine = line;
			if (depth > MAX_DEPTH)
				failure(
					"nesting_limit",
					"Literal nesting exceeds the symbol index limit.",
					start,
					startLine,
				);
			advance();
			while (offset < source.length) {
				const char = source[offset];
				if (char === quote) {
					advance();
					return;
				}
				if (char === "\\") {
					advance();
					if (offset >= source.length) break;
					if (
						!"\"'\\nrtbf/u".includes(source[offset]) &&
						!(quote === "`" && "`$".includes(source[offset]))
					) {
						failure(
							"invalid_escape",
							"Invalid FlowScript string escape.",
							offset,
							line,
						);
					}
					if (source[offset] === "u") {
						if (!/^[0-9a-fA-F]{4}$/.test(source.slice(offset + 1, offset + 5)))
							failure(
								"invalid_escape",
								"Invalid Unicode escape.",
								offset,
								line,
							);
						offset += 5;
					} else advance();
					continue;
				}
				if (quote === "`" && char === "$" && source[offset + 1] === "{") {
					offset += 2;
					const expressionStart = offset;
					const stack = ["}"];
					while (offset < source.length && stack.length > 0) {
						const inner = source[offset];
						if (inner === '"' || inner === "'" || inner === "`") {
							skipLiteral(depth + 1);
							continue;
						}
						if (inner === "/" && source[offset + 1] === "/") {
							skipComment(false);
							continue;
						}
						if (inner === "/" && source[offset + 1] === "*")
							failure(
								"unsupported_syntax",
								"Block comments are not canonical FlowScript syntax.",
								offset,
								line,
							);
						if ("({[".includes(inner)) {
							stack.push(inner === "(" ? ")" : inner === "{" ? "}" : "]");
							if (stack.length + depth > MAX_DEPTH)
								failure(
									"nesting_limit",
									"Template nesting exceeds the symbol index limit.",
									offset,
									line,
								);
						} else if (")}]".includes(inner)) {
							if (stack.pop() !== inner)
								failure(
									"unbalanced_delimiter",
									"Mismatched template expression delimiter.",
									offset,
									line,
								);
						}
						advance();
					}
					if (stack.length > 0)
						failure(
							"unterminated_literal",
							"Unterminated template interpolation.",
							start,
							startLine,
						);
					if (!source.slice(expressionStart, offset - 1).trim())
						failure(
							"invalid_template",
							"Empty template interpolation.",
							expressionStart,
							line,
						);
					continue;
				}
				advance();
			}
			failure(
				"unterminated_literal",
				"Unterminated FlowScript literal.",
				start,
				startLine,
			);
		};
		while (offset < source.length) {
			const char = String.fromCodePoint(source.codePointAt(offset) ?? 0);
			if (/\s/u.test(char)) {
				advance();
				continue;
			}
			if (char === "/" && source[offset + 1] === "/") {
				skipComment(true);
				continue;
			}
			if (char === "/" && source[offset + 1] === "*")
				failure(
					"unsupported_syntax",
					"Block comments are not canonical FlowScript syntax.",
					offset,
					line,
				);
			const start = offset;
			const tokenLine = line;
			let kind: Token["kind"] = "punctuation";
			if (char === '"' || char === "'" || char === "`") {
				kind = "literal";
				skipLiteral(0);
			} else if (/[\p{L}_$]/u.test(char)) {
				kind = "identifier";
				offset += char.length;
				while (offset < source.length) {
					const next = String.fromCodePoint(source.codePointAt(offset) ?? 0);
					if (!/[\p{L}\p{N}_$]/u.test(next)) break;
					offset += next.length;
				}
			} else if (
				/\d/.test(char) ||
				(char === "." && /\d/.test(source[offset + 1] ?? ""))
			) {
				kind = "literal";
				numberPattern.lastIndex = offset;
				const number = numberPattern.exec(source);
				offset += number?.[0].length ?? 1;
			} else {
				if (!"@(){}[],;:.=?!><|+-*/%^&".includes(char))
					failure(
						"unsupported_syntax",
						"Unrecognized FlowScript token.",
						offset,
						line,
					);
				offset += char === ":" && source[offset + 1] === ":" ? 2 : 1;
			}
			const token = {
				text: source.slice(start, offset),
				kind,
				start,
				end: offset,
				line: tokenLine,
			};
			const index = tokens.length;
			tokens.push(token);
			if (tokens.length > MAX_TOKENS)
				failure(
					"token_limit",
					"FlowScript exceeds the symbol index token limit.",
					start,
					tokenLine,
				);
			if (kind === "punctuation" && "({[".includes(token.text)) {
				opens.push(index);
				if (opens.length > MAX_DEPTH)
					failure(
						"nesting_limit",
						"FlowScript nesting exceeds the symbol index limit.",
						start,
						tokenLine,
					);
			} else if (kind === "punctuation" && ")}]".includes(token.text)) {
				const open = opens.pop();
				if (
					open === undefined ||
					"({[".indexOf(tokens[open].text) !== ")}]".indexOf(token.text)
				)
					failure(
						"unbalanced_delimiter",
						"Mismatched FlowScript delimiter.",
						start,
						tokenLine,
					);
				pairs.set(open, index);
			}
		}
		if (opens.length) {
			const token = tokens[opens[opens.length - 1]];
			failure(
				"unbalanced_delimiter",
				"Unclosed FlowScript delimiter.",
				token.start,
				token.line,
			);
		}

		const symbols: FlowScriptWorkspaceSymbol[] = [];
		const names = new Set<string>();
		const anchors = new Set<string>();
		let cursor = 0;
		const current = () => tokens[cursor];
		function failHere(message: string): never {
			return failure(
				"invalid_declaration",
				message,
				current()?.start ?? source.length,
				current()?.line ?? line,
			);
		}
		const eat = (text: string) => {
			if (current()?.text !== text) return false;
			cursor++;
			return true;
		};
		const expect = (text: string) => {
			if (!eat(text)) failHere(`Expected ${text} in a FlowScript declaration.`);
		};
		const identifier = (): Token => {
			const token = current();
			if (token?.kind !== "identifier")
				failHere("Expected a FlowScript declaration name.");
			cursor++;
			return token;
		};
		const skipGroup = (opening: string) => {
			const openingIndex = cursor;
			expect(opening);
			const closing = pairs.get(openingIndex);
			if (closing === undefined) failHere("Missing declaration delimiter.");
			cursor = closing + 1;
		};
		const typeRef = () => {
			const base = identifier().text;
			if ((base === "Map" || base === "Set") && eat("<")) {
				if (base === "Map") {
					if (identifier().text !== "string")
						failHere("FlowScript Map keys must be strings.");
					expect(",");
				}
				const inner = identifier().text;
				if (inner === "geometry" && eat("<")) {
					identifier();
					expect(">");
				}
				expect(">");
			} else {
				if (base === "geometry" && eat("<")) {
					identifier();
					expect(">");
				}
				if (eat("[")) expect("]");
			}
		};
		const parameters = () => {
			expect("(");
			while (!eat(")")) {
				identifier();
				expect(":");
				typeRef();
				if (!eat(",")) {
					expect(")");
					break;
				}
			}
		};
		const literal = () => {
			if (current()?.text === "{" || current()?.text === "[") {
				skipGroup(current().text);
				return;
			}
			eat("-");
			const token = current();
			if (
				token?.kind !== "literal" &&
				!["true", "false", "null"].includes(token?.text ?? "")
			)
				failHere("Expected a literal variable default.");
			cursor++;
		};
		const useTree = () => {
			identifier();
			while (eat("::")) {
				if (eat("*")) return;
				if (eat("{")) {
					do {
						identifier();
						if (eat("as")) identifier();
					} while (eat(",") && current()?.text !== "}");
					expect("}");
					return;
				}
				identifier();
			}
			if (eat("as")) identifier();
		};
		const leadingDocumentation = (start: number, floor: number) => {
			let candidate = start;
			let lo = 0;
			let hi = comments.length;
			while (lo < hi) {
				const mid = (lo + hi) >>> 1;
				if (comments[mid].end <= start) lo = mid + 1;
				else hi = mid;
			}
			for (let i = lo - 1; i >= 0; i--) {
				const comment = comments[i];
				if (comment.end > candidate) continue;
				if (comment.start < floor) break;
				const gap = source.slice(comment.end, candidate);
				if (gap.trim() || (gap.match(/\n/g)?.length ?? 0) > 1) break;
				const lineStart = source.lastIndexOf("\n", comment.start - 1) + 1;
				if (source.slice(lineStart, comment.start).trim()) break;
				if (/\/\/@[nvl]:/.test(source.slice(comment.start, comment.end))) break;
				candidate = comment.start;
			}
			return candidate;
		};
		const rejectHeaderComments = (start: number, end: number) => {
			let lo = 0;
			let hi = comments.length;
			while (lo < hi) {
				const mid = (lo + hi) >>> 1;
				if (comments[mid].start < start) lo = mid + 1;
				else hi = mid;
			}
			const comment = comments[lo];
			if (comment && comment.start < end) {
				failure(
					"invalid_declaration",
					"Comments must precede or follow a declaration header.",
					comment.start,
					comment.line,
				);
			}
		};
		const scope = (path: string[], stop: number) => {
			let previousEnd = cursor > 0 ? tokens[cursor - 1].end : 0;
			while (cursor < stop) {
				if (eat(";")) {
					previousEnd = tokens[cursor - 1].end;
					continue;
				}
				const first = current();
				if (!first) break;
				let decorated = false;
				while (eat("@")) {
					decorated = true;
					identifier();
					if (current()?.text === "(") skipGroup("(");
				}
				const head = current();
				const keyword = identifier().text;
				if (
					path.length &&
					["const", "let", "use", "interface"].includes(keyword)
				)
					failHere(
						"Variables, uses and interfaces belong at the root of a FlowScript file.",
					);
				if (decorated && !["const", "let", "function"].includes(keyword))
					failHere("This FlowScript declaration does not support decorators.");
				if (keyword === "use") {
					do {
						useTree();
					} while (eat(","));
					previousEnd = tokens[cursor - 1].end;
					continue;
				}
				if (keyword === "const" || keyword === "let") {
					identifier();
					if (eat("=")) literal();
					else {
						expect(":");
						typeRef();
						if (eat("=")) literal();
					}
					previousEnd = tokens[cursor - 1].end;
					continue;
				}
				if (keyword === "detached" && current()?.text === "{") {
					skipGroup("{");
					previousEnd = tokens[cursor - 1].end;
					continue;
				}
				let kind: FlowScriptWorkspaceSymbol["kind"];
				let name: string;
				if (
					keyword === "module" &&
					current()?.kind === "identifier" &&
					tokens[cursor + 1]?.text === "{"
				) {
					kind = "module";
					name = identifier().text;
				} else if (keyword === "interface") {
					kind = "interface";
					name = identifier().text;
				} else if (keyword === "function") {
					kind = "function";
					name = identifier().text;
					parameters();
					if (eat(":")) parameters();
				} else {
					kind = "event";
					name = current()?.kind === "identifier" ? identifier().text : keyword;
					parameters();
				}
				const openIndex = cursor;
				const open = current();
				expect("{");
				rejectHeaderComments(first.start, open.end);
				const closeIndex = pairs.get(openIndex);
				if (closeIndex === undefined || closeIndex >= stop)
					failHere("Declaration extends outside its enclosing module.");
				const close = tokens[closeIndex];
				const qualifiedName = [...path, name].join("::");
				if (names.has(qualifiedName))
					failure(
						"ambiguous_symbol",
						"Duplicate qualified FlowScript symbol name.",
						head.start,
						head.line,
					);
				names.add(qualifiedName);
				const start = leadingDocumentation(first.start, previousEnd);
				const startLine =
					first.line -
					(source.slice(start, first.start).match(/\n/g)?.length ?? 0);
				const symbol: FlowScriptWorkspaceSymbol = {
					name,
					qualifiedName,
					kind,
					signature: source.slice(head.start, open.start).trim(),
					start,
					end: close.end,
					startLine,
					endLine: close.line,
				};
				const comment = commentsByLine.get(open.line);
				const anchor =
					comment &&
					comment.start >= open.end &&
					!source.slice(open.end, comment.start).trim() &&
					/\/\/@([nvl]):([A-Za-z0-9_-]+)[ \t\r]*$/.exec(
						source.slice(comment.start, comment.end),
					);
				if (anchor) {
					const key = `${anchor[1]}:${anchor[2]}`;
					if (anchors.has(key))
						failure(
							"ambiguous_anchor",
							"Duplicate FlowScript declaration anchor.",
							comment.start,
							comment.line,
						);
					anchors.add(key);
					symbol.anchor = {
						kind:
							anchor[1] === "n"
								? "node"
								: anchor[1] === "v"
									? "variable"
									: "layer",
						id: anchor[2],
					};
				}
				symbols.push(symbol);
				if (symbols.length > MAX_SYMBOLS)
					failure(
						"symbol_limit",
						"FlowScript exceeds the symbol index declaration limit.",
						head.start,
						head.line,
					);
				if (kind === "module") scope([...path, name], closeIndex);
				cursor = closeIndex + 1;
				previousEnd = close.end;
			}
		};
		scope([], tokens.length);
		return {
			symbols,
			complete: true,
			diagnostics: [],
			validation: "declaration_structure",
		};
	} catch (error) {
		if (!(error instanceof IndexFailure)) throw error;
		return {
			symbols: [],
			complete: false,
			diagnostics: [error.diagnostic],
			validation: "declaration_structure",
		};
	}
}
