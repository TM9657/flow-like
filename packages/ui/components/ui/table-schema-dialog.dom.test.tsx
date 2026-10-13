import { afterAll, afterEach, expect, mock, test } from "bun:test";
import { Window } from "happy-dom";
import { act } from "react";
import type { IColumnMetadata } from "../../lib/column-metadata";
import type {
	IAddColumnPayload,
	IIndexConfig,
} from "../../state/backend-state/db-state";
import type { LanceSchema } from "./lance-viewer";

class InertObserver {
	observe() {}
	unobserve() {}
	disconnect() {}
	takeRecords() {
		return [];
	}
}

const window = new Window({ url: "https://localhost" });
Object.assign(window, { SyntaxError, TypeError, Error });
// Radix Select scrolls and captures pointers on elements; happy-dom lacks both.
const elementProto = window.Element.prototype as unknown as Record<
	string,
	unknown
>;
elementProto.scrollIntoView ??= () => {};
elementProto.hasPointerCapture ??= () => false;
elementProto.releasePointerCapture ??= () => {};
const globals = {
	window,
	document: window.document,
	navigator: window.navigator,
	HTMLElement: window.HTMLElement,
	HTMLInputElement: window.HTMLInputElement,
	HTMLTextAreaElement: window.HTMLTextAreaElement,
	HTMLSelectElement: window.HTMLSelectElement,
	HTMLButtonElement: window.HTMLButtonElement,
	IntersectionObserver: InertObserver,
	DOMRect: window.DOMRect,
	Element: window.Element,
	Node: window.Node,
	Text: window.Text,
	DocumentFragment: window.DocumentFragment,
	NodeFilter: window.NodeFilter,
	MutationObserver: window.MutationObserver,
	ResizeObserver: window.ResizeObserver,
	Event: window.Event,
	CustomEvent: window.CustomEvent,
	InputEvent: window.InputEvent,
	KeyboardEvent: window.KeyboardEvent,
	FocusEvent: window.FocusEvent,
	MouseEvent: window.MouseEvent,
	PointerEvent: window.PointerEvent,
	getComputedStyle: window.getComputedStyle.bind(window),
	requestAnimationFrame: (callback: FrameRequestCallback) =>
		setTimeout(() => callback(0), 0),
	cancelAnimationFrame: (id: number) => clearTimeout(id),
	IS_REACT_ACT_ENVIRONMENT: true,
};
// bun keeps globals and module mocks for every later file in the process, so both are
// captured first and put back in afterAll.
const globalDescriptors = Object.keys(globals).map(
	(key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)] as const,
);
Object.assign(globalThis, globals);
const actualLocales = { ...(await import("@flow-like/locales")) };

const translate = (
	_key: string,
	fallback: string | Record<string, unknown> = "",
	variables: Record<string, unknown> = {},
) => {
	const options = typeof fallback === "string" ? variables : fallback;
	const text =
		typeof fallback === "string"
			? fallback
			: String(
					options.count === 1
						? options.defaultValue_one
						: (options.defaultValue_other ?? options.defaultValue ?? ""),
				);
	return text.replace(/\{\{(\w+)\}\}/g, (_, key) => String(options[key] ?? ""));
};
mock.module("@flow-like/locales", () => ({
	...actualLocales,
	useTranslation: () => ({ t: translate }),
}));

const { createRoot } = await import("react-dom/client");
const { TableSchemaDialog } = await import("./table-schema-dialog");

afterAll(() => {
	mock.restore();
	mock.module("@flow-like/locales", () => actualLocales);
	for (const [key, descriptor] of globalDescriptors) {
		if (descriptor) Object.defineProperty(globalThis, key, descriptor);
		else Reflect.deleteProperty(globalThis, key);
	}
});

const cleanups: (() => Promise<void>)[] = [];
afterEach(async () => {
	for (const cleanup of cleanups.splice(0)) await cleanup();
});

const schema: LanceSchema = {
	table: "entities",
	primaryKey: "id",
	fields: [
		{ name: "id", kind: "string", nullable: false, keyEligible: true },
		{ name: "source_id", kind: "string", nullable: false, keyEligible: true },
		{ name: "label", kind: "string", nullable: true },
	],
};

const indices: IIndexConfig[] = [
	{ name: "id_idx", index_type: "BTree", columns: ["id"] },
];

type Props = Parameters<typeof TableSchemaDialog>[0];

async function openDialog(props: Partial<Props> = {}) {
	const container = window.document.createElement("div");
	window.document.body.appendChild(container);
	const root = createRoot(container as unknown as HTMLElement);
	cleanups.push(async () => {
		await act(async () => root.unmount());
		container.remove();
	});
	await act(async () =>
		root.render(
			<TableSchemaDialog
				schema={schema}
				tableName="entities"
				rowCount={12480}
				onGetIndices={async () => indices}
				{...props}
			/>,
		),
	);
	await act(async () => buttonNamed("Schema", container).click());
	const dialog = window.document.querySelector('[role="dialog"]');
	if (!dialog) throw new Error("schema dialog did not open");
	return dialog;
}

/** happy-dom nodes don't satisfy lib.dom's `ParentNode`, so scopes are typed structurally. */
type Scope = { querySelectorAll(selectors: string): ArrayLike<unknown> };

function buttons(scope: Scope = window.document.body) {
	return Array.from(scope.querySelectorAll("button")) as HTMLButtonElement[];
}

function buttonNamed(label: string, scope?: Scope) {
	const match = buttons(scope).find(
		(button) =>
			button.textContent?.trim() === label ||
			button.getAttribute("aria-label") === label,
	);
	if (!match) throw new Error(`no button named "${label}"`);
	return match;
}

function railItem(name: string) {
	const match = buttons().find(
		(button) =>
			button.querySelector(".font-mono")?.textContent === name &&
			button.closest("nav"),
	);
	if (!match) throw new Error(`no rail item for "${name}"`);
	return match;
}

async function click(button: HTMLButtonElement) {
	await act(async () => button.click());
}

async function type(
	input: HTMLInputElement | HTMLTextAreaElement,
	value: string,
) {
	const prototype =
		input.tagName === "TEXTAREA"
			? window.HTMLTextAreaElement.prototype
			: window.HTMLInputElement.prototype;
	const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
	await act(async () => {
		setter?.call(input, value);
		input.dispatchEvent(
			new window.InputEvent("input", { bubbles: true }) as unknown as Event,
		);
	});
}

/** The control a `<label>` with exactly this text points at. */
function labelled<T = HTMLInputElement>(text: string): T {
	const label = Array.from(window.document.body.querySelectorAll("label")).find(
		(element) => element.textContent?.trim() === text,
	);
	const control = label?.getAttribute("for")
		? window.document.getElementById(label.getAttribute("for") ?? "")
		: null;
	if (!control) throw new Error(`no control labelled "${text}"`);
	return control as unknown as T;
}

function option(label: string) {
	const match = Array.from(
		window.document.body.querySelectorAll('[role="option"]'),
	).find((element) => element.textContent?.trim() === label);
	if (!match) throw new Error(`no option "${label}"`);
	return match as unknown as HTMLButtonElement;
}

const salary = {
	name: "salary",
	kind: "number",
	nullable: true,
	columnMetadata: {
		display_name: "Annual salary",
		description: "Gross, in EUR",
		classification: "confidential",
		pii: true,
		read_attributes: ["hr"],
	},
} satisfies LanceSchema["fields"][number];

test("dropping a column waits for confirmation", async () => {
	const dropped: string[][] = [];
	const dialog = await openDialog({
		onDropColumns: async (columns) => {
			dropped.push(columns);
		},
	});
	await click(railItem("label"));
	await click(buttonNamed("Drop column"));
	expect(dropped).toEqual([]);
	expect(dialog.textContent).toContain("Drop label?");
	await click(buttonNamed("Drop label"));
	expect(dropped).toEqual([["label"]]);
});

test("a failed drop keeps the confirmation open", async () => {
	const dialog = await openDialog({
		onDropColumns: async () => {
			throw new Error("Commit conflict");
		},
	});
	await click(railItem("label"));
	await click(buttonNamed("Drop column"));
	await click(buttonNamed("Drop label"));
	expect(dialog.textContent).toContain("Drop label?");
	expect(buttonNamed("Drop label").disabled).toBe(false);
});

test("the key column can't be made nullable or dropped", async () => {
	const altered: [string, boolean][] = [];
	const dialog = await openDialog({
		onAlterColumn: async (column, nullable) => {
			altered.push([column, nullable]);
		},
		onDropColumns: async () => {},
	});
	expect(dialog.querySelector("h3")?.textContent).toContain("id");
	expect(dialog.textContent).toContain("Key stays required");
	expect(() => buttonNamed("Allow NULL")).toThrow();
	expect(dialog.textContent).toContain("The table key can't be dropped");
	expect(() => buttonNamed("Drop column")).toThrow();

	await click(railItem("source_id"));
	await click(buttonNamed("Allow NULL"));
	expect(altered).toEqual([]);
	await click(buttonNamed("Allow NULL"));
	expect(altered).toEqual([["source_id", true]]);
});

test("a nullable column can be made required again", async () => {
	const altered: [string, boolean][] = [];
	await openDialog({
		onAlterColumn: async (column, nullable) => {
			altered.push([column, nullable]);
		},
	});
	await click(railItem("label"));
	await click(buttonNamed("Make required"));
	expect(altered).toEqual([["label", false]]);
});

test("Set as key is disabled for columns that can't be the key", async () => {
	const keyed: string[] = [];
	const keyless = { ...schema, primaryKey: undefined };
	await openDialog({
		schema: keyless,
		onSetPrimaryKey: async (column) => {
			keyed.push(column);
		},
	});
	await click(railItem("label"));
	expect(buttonNamed("Set as key").disabled).toBe(true);

	await click(railItem("source_id"));
	await click(buttonNamed("Set as key"));
	expect(keyed).toEqual([]);
	await click(buttonNamed("Set as key"));
	expect(keyed).toEqual(["source_id"]);
});

test("a new column needs a literal that matches its type", async () => {
	const added: IAddColumnPayload[] = [];
	const dialog = await openDialog({
		onAddColumn: async (column) => {
			added.push(column);
		},
	});
	await click(buttonNamed("New column"));
	const name = dialog.querySelector(
		'input[placeholder="column_name"]',
	) as unknown as HTMLInputElement;
	await type(name, "confidence");
	const double = buttons(dialog).find((button) =>
		button.textContent?.startsWith("Double"),
	);
	if (!double) throw new Error("no Double type tile");
	await click(double);
	await click(buttonNamed("A value"));
	const value = dialog.querySelector(
		'input[aria-label="Default value"]',
	) as unknown as HTMLInputElement;
	await type(value, "abc");
	await click(buttonNamed("Add Column"));
	expect(added).toEqual([]);
	expect(dialog.textContent).toContain("Enter a number");

	await type(value, "0.5");
	await click(buttonNamed("Add Column"));
	expect(added).toEqual([
		{ name: "confidence", sql_expression: "CAST(0.5 AS DOUBLE)" },
	]);
});

test("the last column cannot be dropped", async () => {
	const dropped = mock(async (_columns: string[]) => {});
	const dialog = await openDialog({
		schema: {
			table: "vectors",
			fields: [{ name: "embedding", kind: "vector", dims: 384 }],
		},
		onDropColumns: dropped,
	});
	expect(dialog.textContent).toContain(
		"Add a replacement before dropping this one",
	);
	expect(() => buttonNamed("Drop column")).toThrow();
	expect(dropped).not.toHaveBeenCalled();
});

test("vector columns can be dropped after confirmation", async () => {
	const dropped = mock(async (_columns: string[]) => {});
	const dialog = await openDialog({
		schema: {
			...schema,
			fields: [
				...schema.fields,
				{ name: "embedding", kind: "vector", dims: 384 },
			],
		},
		onGetIndices: async () => [
			{ name: "embedding_idx", index_type: "IvfPq", columns: ["embedding"] },
		],
		onDropColumns: dropped,
	});
	await click(railItem("embedding"));
	await click(buttonNamed("Drop column"));
	expect(dropped).not.toHaveBeenCalled();
	expect(dialog.textContent).toContain("Deletes the column and its");
	await click(buttonNamed("Drop embedding"));
	expect(dropped).toHaveBeenCalledWith(["embedding"]);
});

test("a replacement vector column requires a valid dimension and starts empty", async () => {
	const added = mock(async (_column: IAddColumnPayload) => {});
	const dialog = await openDialog({ onAddColumn: added });
	await click(buttonNamed("New column"));
	await type(
		dialog.querySelector(
			'input[placeholder="column_name"]',
		) as unknown as HTMLInputElement,
		"embedding",
	);
	await click(buttonNamed("VectorFloat32"));
	const dimensions = dialog.querySelector(
		'input[type="number"]',
	) as unknown as HTMLInputElement;
	expect(dialog.textContent).toContain("Existing rows start with NULL");
	expect(() => buttonNamed("A value")).toThrow();
	for (const invalid of ["", "0", "-1", "1.5", "2147483648"]) {
		await type(dimensions, invalid);
		await click(buttonNamed("Add Column"));
		expect(added).not.toHaveBeenCalled();
		expect(dimensions.getAttribute("aria-invalid")).toBe("true");
	}
	await type(dimensions, "1536");
	await click(buttonNamed("Add Column"));
	expect(added).toHaveBeenCalledWith({
		name: "embedding",
		type: "vector",
		vector_size: 1536,
	});
});

test("without write callbacks the dialog is read-only", async () => {
	const dialog = await openDialog();
	expect(dialog.textContent).toContain("Read-only");
	for (const label of [
		"New column",
		"Drop column",
		"Allow NULL",
		"Set as key",
		"Drop index",
		"Save",
	]) {
		expect(() => buttonNamed(label)).toThrow();
	}
});

test("column details are edited and saved as one replacing write", async () => {
	const saved: [string, IColumnMetadata][] = [];
	const dialog = await openDialog({
		onSetColumnMetadata: async (column, metadata) => {
			saved.push([column, metadata]);
		},
		canManageColumnAccess: true,
		knownAttributes: ["finance", "hr"],
	});
	await click(railItem("label"));
	expect(buttonNamed("Save").disabled).toBe(true);

	await type(labelled("Display name"), " Label text ");
	await type(
		labelled<HTMLTextAreaElement>("Description"),
		"Free text shown on cards",
	);
	await click(labelled<HTMLButtonElement>("Classification"));
	await click(option("Confidential"));
	await click(labelled<HTMLButtonElement>("Contains personal data"));
	await click(buttonNamed("hr"));
	expect(dialog.textContent).toContain("Unsaved changes");
	await click(buttonNamed("Save"));

	expect(saved).toEqual([
		[
			"label",
			{
				display_name: "Label text",
				description: "Free text shown on cards",
				classification: "confidential",
				pii: true,
				read_attributes: ["hr"],
			},
		],
	]);
	expect(buttonNamed("Save").disabled).toBe(true);
	expect(dialog.textContent).not.toContain("Unsaved changes");
});

test("a failed save keeps the edited details", async () => {
	const dialog = await openDialog({
		onSetColumnMetadata: async () => {
			throw new Error("Column 'label' has 200 characters");
		},
	});
	await click(railItem("label"));
	await type(labelled("Display name"), "Label text");
	await click(buttonNamed("Save"));
	expect(labelled("Display name").value).toBe("Label text");
	expect(buttonNamed("Save").disabled).toBe(false);
	expect(dialog.textContent).toContain("Unsaved changes");
});

test("without the metadata callback column details are read-only", async () => {
	const dialog = await openDialog({
		schema: { ...schema, fields: [...schema.fields, salary] },
	});
	const rail = railItem("salary");
	expect(rail.textContent).toContain("Annual salary");
	expect(rail.querySelector('[aria-label="Read-restricted"]')).not.toBeNull();
	expect(
		rail.querySelector('[aria-label="Contains personal data"]'),
	).not.toBeNull();

	await click(rail);
	for (const text of ["Annual salary", "Gross, in EUR", "Confidential", "PII"])
		expect(dialog.textContent).toContain(text);
	expect(dialog.querySelector("textarea")).toBeNull();
	expect(dialog.querySelector('[role="switch"]')).toBeNull();
	expect(() => labelled("Display name")).toThrow();
	expect(() => buttonNamed("Save")).toThrow();
});

test("only admins can change who may read a column", async () => {
	const saved: [string, IColumnMetadata][] = [];
	const dialog = await openDialog({
		schema: { ...schema, fields: [...schema.fields, salary] },
		onSetColumnMetadata: async (column, metadata) => {
			saved.push([column, metadata]);
		},
		knownAttributes: ["finance", "hr"],
	});
	await click(railItem("salary"));
	expect(labelled("Allowed to read").disabled).toBe(true);
	expect(buttonNamed("Remove hr").disabled).toBe(true);
	expect(() => buttonNamed("finance")).toThrow();
	expect(dialog.textContent).toContain(
		"Only admins and owners can change who may read a column.",
	);

	await type(labelled("Display name"), "Salary");
	await click(buttonNamed("Save"));
	expect(saved[0]?.[1]).toEqual({
		display_name: "Salary",
		description: "Gross, in EUR",
		classification: "confidential",
		pii: true,
		read_attributes: ["hr"],
	});
});
