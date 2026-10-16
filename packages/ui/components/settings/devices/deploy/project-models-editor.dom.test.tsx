import { afterAll, afterEach, describe, expect, test } from "bun:test";
import type { IApp } from "../../../../lib/schema/app/app";
import { type IBit, IBitTypes } from "../../../../lib/schema/bit/bit";
import type { IBackendState } from "../../../../state/backend-state";
import {
	advance,
	byRole,
	click,
	installDom,
	typeInto,
} from "../testing/dom-harness";

const dom = installDom();
const { mountDevices, cleanupDevices, preloadDevices } = await import(
	"../testing/mount-devices"
);
const { ProjectModelsEditor } = await import("./project-models-editor");
await preloadDevices();
afterEach(async () => {
	await cleanupDevices();
	await dom.cleanup();
});
afterAll(dom.restore);

function model(
	id: string,
	name: string,
	hub = "models.example",
	type = IBitTypes.Llm,
): IBit {
	return {
		id,
		hub,
		type,
		meta: { en: { name } },
		parameters: {},
	} as unknown as IBit;
}

async function open(options: { bits?: string[]; failSave?: boolean } = {}) {
	const existing = model("existing", "Existing model");
	const custom = model(
		"private",
		"Private decision model",
		"models.example",
		IBitTypes.SystemOne,
	);
	const found = model("distant", "Distant model");
	let current = {
		id: "project",
		bits: options.bits ?? ["models.example:existing", "config", "unavailable"],
		packages: {},
		version: "1.0.0",
	} as unknown as IApp;
	const reads: [string, string | undefined][] = [];
	const writes: IApp[] = [];
	const saved: IApp[] = [];
	const editing: boolean[] = [];
	const searches: string[] = [];
	const backend = {
		appState: {
			getApp: async () => structuredClone(current),
			getAppAuthoritative: async () => structuredClone(current),
			updateAppAuthoritative: async (next: IApp) => {
				if (options.failSave) throw new Error("Saving was refused");
				writes.push(next);
				current = next;
			},
		},
		bitState: {
			getProfileBits: async () => [existing],
			listCustomBits: async () => [custom],
			searchBits: async (query: { search?: string }) => {
				searches.push(query.search ?? "");
				return query.search ? [found] : [existing];
			},
			getBit: async (id: string, hub?: string) => {
				reads.push([id, hub]);
				if (id === "unavailable") throw new Error("Unavailable");
				return id === "config"
					? model(id, "Tokenizer config", "", IBitTypes.Config)
					: existing;
			},
		},
	} as unknown as Partial<IBackendState>;
	const view = await mountDevices(
		<ProjectModelsEditor
			appId="project"
			onSaved={(app) => saved.push(app)}
			onEditingChange={(value) => editing.push(value)}
		/>,
		{ backend },
	);
	await click(byRole("button", "Edit models"));
	await view.settle();
	return {
		...view,
		reads,
		writes,
		saved,
		editing,
		searches,
		change: (patch: Partial<IApp>) => {
			current = { ...current, ...patch };
		},
	};
}

describe("project model choices", () => {
	test("qualified roots stay selected once; saves preserve references, other dependencies, and fresh app fields", async () => {
		const view = await open();
		expect(view.reads).toContainEqual(["existing", "models.example"]);
		expect(
			view.container.querySelectorAll(
				'[role="checkbox"][data-state="checked"]',
			),
		).toHaveLength(1);
		expect(
			byRole("checkbox", "Existing model").getAttribute("aria-checked"),
		).toBe("true");
		await click(byRole("checkbox", "Private decision model"));
		view.change({ version: "1.1.0" });
		await click(byRole("button", "Save models"));
		await view.settle();
		expect(view.writes).toHaveLength(1);
		expect(view.writes[0]).toMatchObject({
			version: "1.1.0",
			bits: [
				"models.example:existing",
				"config",
				"unavailable",
				"models.example:private",
			],
		});
		expect(view.saved).toEqual(view.writes);
		expect(view.editing).toEqual([true, false]);
	});

	test("removing a qualified root removes it, while nonmodel and unreadable dependencies remain", async () => {
		const view = await open();
		await click(byRole("checkbox", "Existing model"));
		await click(byRole("button", "Save models"));
		await view.settle();
		expect(view.writes[0]?.bits).toEqual(["config", "unavailable"]);
	});

	test("catalog search can add a model outside the initial results", async () => {
		const view = await open();
		await typeInto(byRole("textbox", "Search models"), "Distant");
		await advance(250);
		await view.settle();
		expect(view.searches).toContain("Distant");
		await click(byRole("checkbox", "Distant model"));
		await click(byRole("button", "Save models"));
		await view.settle();
		expect(view.writes[0]?.bits).toContain("models.example:distant");
	});

	test("concurrent dependency changes stop the save and keep the editor open", async () => {
		const view = await open();
		await click(byRole("checkbox", "Private decision model"));
		view.change({ bits: ["someone-elses-model"] });
		await click(byRole("button", "Save models"));
		await view.settle();
		expect(view.writes).toHaveLength(0);
		expect(byRole("alert").textContent).toContain("models changed elsewhere");
		expect(view.editing).toEqual([true]);
	});

	test("concurrent model reordering is preserved by refusing a stale save", async () => {
		const view = await open();
		await click(byRole("checkbox", "Private decision model"));
		view.change({ bits: ["config", "models.example:existing", "unavailable"] });
		await click(byRole("button", "Save models"));
		await view.settle();
		expect(view.writes).toHaveLength(0);
		expect(byRole("alert").textContent).toContain("models changed elsewhere");
	});

	test("a failed save keeps the editor open; leaving it releases the preparation pause", async () => {
		const view = await open({ failSave: true });
		await click(byRole("checkbox", "Private decision model"));
		await click(byRole("button", "Save models"));
		await view.settle();
		expect(byRole("alert").textContent).toContain("Saving was refused");
		expect(view.saved).toHaveLength(0);
		await view.unmount();
		expect(view.editing).toEqual([true, false]);
	});
});
