import postcss, { type Root, type Rule } from "postcss";

type TailwindCompiler = Awaited<
	ReturnType<typeof import("tailwindcss")["compile"]>
>;

const RUNTIME_STYLE_ATTRIBUTE = "data-a2ui-runtime-tailwind";
const RUNTIME_ROOT_ATTRIBUTE = "data-a2ui-runtime-tailwind-root";
const RUNTIME_SCOPE = `:where([${RUNTIME_ROOT_ATTRIBUTE}], [${RUNTIME_ROOT_ATTRIBUTE}] *)`;
const THEME_VARIABLE =
	/^--(?:color|font|text|tracking|leading|spacing|breakpoint|container|radius|shadow|inset-shadow|drop-shadow|blur|perspective|aspect|ease|animate|default)(?:-|$)/;

/** Reference the installed theme without emitting another theme or preflight. */
export function runtimeTailwindTheme(
	entries: Iterable<[string, string]>,
): string {
	const variables = new Map(entries);
	const declarations: string[] = [];
	const aliases: string[] = [];
	for (const [name, value] of variables) {
		if (!THEME_VARIABLE.test(name) || !value.trim()) continue;
		declarations.push(`${name}: ${value};`);
		// Match the inline color aliases in global.css, including scoped themes.
		if (name.startsWith("--color-") && variables.has(`--${name.slice(8)}`)) {
			aliases.push(`${name}: var(--${name.slice(8)});`);
		}
	}
	if (variables.has("--radius")) {
		aliases.push(
			"--radius-sm: calc(var(--radius) - 4px);",
			"--radius-md: calc(var(--radius) - 2px);",
			"--radius-lg: var(--radius);",
			"--radius-xl: calc(var(--radius) + 4px);",
		);
	}
	return `@theme reference { ${declarations.join("\n")} }
@theme inline { ${aliases.join("\n")} }
@custom-variant dark (&:is(.dark *));
@layer utilities { @tailwind utilities; }`;
}

function readTheme(doc: Document): Array<[string, string]> {
	const styles = doc.defaultView?.getComputedStyle(doc.documentElement);
	if (!styles) return [];
	// Preview frames can mount before their copied stylesheet links finish loading.
	if (!styles.getPropertyValue("--spacing") && doc.defaultView?.frameElement) {
		return readTheme(doc.defaultView.frameElement.ownerDocument);
	}
	return Array.from({ length: styles.length }, (_, index) => styles.item(index))
		.filter((name) => name.startsWith("--"))
		.map((name) => [name, styles.getPropertyValue(name)]);
}

interface DocumentRuntime {
	compiler?: Promise<TailwindCompiler>;
	candidates: Set<string>;
	style: HTMLStyleElement;
	scheduled: boolean;
	users: number;
	stopWaitingForTheme?: () => void;
}

const runtimes = new WeakMap<Document, DocumentRuntime>();
const rootUsers = new WeakMap<HTMLElement, number>();

export function scopeRuntimeTailwind(css: string): string {
	const stylesheet = postcss.parse(css);
	stylesheet.walkRules((rule) => {
		let parent: Rule["parent"] | Root["parent"] = rule.parent;
		while (parent) {
			// Variants inherit their utility's scope. Keyframes are animation steps.
			if (
				parent.type === "rule" ||
				(parent.type === "atrule" && /keyframes$/i.test(parent.name))
			) {
				return;
			}
			parent = parent.parent;
		}
		rule.selectors = rule.selectors.map(
			(selector) => `${RUNTIME_SCOPE}${selector === "*" ? "" : selector}`,
		);
	});
	return stylesheet.toString();
}

function getRuntime(doc: Document): DocumentRuntime {
	let runtime = runtimes.get(doc);
	if (!runtime) {
		const style = doc.createElement("style");
		style.setAttribute(RUNTIME_STYLE_ATTRIBUTE, "");
		runtime = { candidates: new Set(), style, scheduled: false, users: 0 };
		runtimes.set(doc, runtime);
	}
	return runtime;
}

function scheduleCompilation(doc: Document, runtime: DocumentRuntime): void {
	if (runtime.scheduled) return;
	runtime.scheduled = true;
	queueMicrotask(async () => {
		try {
			if (!runtime.compiler) {
				const theme = readTheme(doc);
				if (!theme.some(([name]) => name === "--spacing")) {
					waitForTheme(doc, runtime);
					return;
				}
				runtime.stopWaitingForTheme?.();
				runtime.compiler = import("tailwindcss").then(({ compile }) =>
					compile(runtimeTailwindTheme(theme)),
				);
			}
			const compiler = await runtime.compiler;
			if (runtime.users === 0) return;
			// A global .hidden emitted here would override the host sidebar's md:flex.
			const css = scopeRuntimeTailwind(
				compiler.build(Array.from(runtime.candidates)),
			);
			if (runtime.style.textContent !== css) runtime.style.textContent = css;
			if (!runtime.style.isConnected) doc.head.appendChild(runtime.style);
		} catch (error) {
			runtime.compiler = undefined;
			console.warn("[a2ui] Could not compile runtime Tailwind classes:", error);
		} finally {
			runtime.scheduled = false;
		}
	});
}

function waitForTheme(doc: Document, runtime: DocumentRuntime): void {
	if (runtime.stopWaitingForTheme) return;
	const Observer = doc.defaultView?.MutationObserver;
	if (!Observer) return;
	const retry = () => scheduleCompilation(doc, runtime);
	const observer = new Observer(retry);
	observer.observe(doc.head, {
		childList: true,
		subtree: true,
		characterData: true,
	});
	observer.observe(doc.documentElement, {
		attributes: true,
		attributeFilter: ["style", "class"],
	});
	doc.addEventListener("load", retry, true);
	runtime.stopWaitingForTheme = () => {
		observer.disconnect();
		doc.removeEventListener("load", retry, true);
		runtime.stopWaitingForTheme = undefined;
	};
}

/**
 * Compile classes that arrive through saved pages, widget parameters, or live updates.
 * Each document owns its stylesheet so iframe media queries use the preview viewport.
 * Utilities apply only to observed roots, including their portaled content.
 */
export function observeRuntimeTailwind(root: HTMLElement): () => void {
	const doc = root.ownerDocument;
	const Observer = doc.defaultView?.MutationObserver;
	if (!Observer) return () => {};
	const runtime = getRuntime(doc);
	runtime.users += 1;
	rootUsers.set(root, (rootUsers.get(root) ?? 0) + 1);
	root.setAttribute(RUNTIME_ROOT_ATTRIBUTE, "");

	const collect = (element: Element) => {
		let changed = false;
		for (const candidate of Array.from(element.classList)) {
			if (runtime.candidates.has(candidate)) continue;
			runtime.candidates.add(candidate);
			changed = true;
		}
		return changed;
	};
	const collectTree = (node: Node) => {
		if (node.nodeType !== 1) return false;
		const element = node as Element;
		let changed = collect(element);
		for (const child of Array.from(element.querySelectorAll("[class]"))) {
			changed = collect(child) || changed;
		}
		return changed;
	};
	const observer = new Observer((records) => {
		let changed = false;
		for (const record of records) {
			if (record.type === "attributes") {
				changed = collect(record.target as Element) || changed;
			} else {
				for (const node of Array.from(record.addedNodes)) {
					changed = collectTree(node) || changed;
				}
			}
		}
		if (changed) scheduleCompilation(doc, runtime);
	});
	observer.observe(root, {
		subtree: true,
		childList: true,
		attributes: true,
		attributeFilter: ["class"],
	});
	collectTree(root);
	scheduleCompilation(doc, runtime);
	return () => {
		observer.disconnect();
		const remaining = (rootUsers.get(root) ?? 1) - 1;
		if (remaining === 0) {
			rootUsers.delete(root);
			root.removeAttribute(RUNTIME_ROOT_ATTRIBUTE);
		} else {
			rootUsers.set(root, remaining);
		}
		runtime.users -= 1;
		if (runtime.users === 0) {
			runtime.stopWaitingForTheme?.();
			queueMicrotask(() => {
				if (runtime.users === 0) runtime.style.remove();
			});
		}
	};
}
