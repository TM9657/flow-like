import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type ReactNode, act, useEffect } from "react";
import { type Root, createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiResponseError } from "../../lib/api-error";
import { AdminHomePage } from "./admin-home-page";
import type { HomeEditorProps } from "./home-editor";
import type { IHomeDefault, IHomeDefaults, IHomeLayout } from "./types";

const fixture = vi.hoisted(() => ({
	profile: {
		id: "admin-profile",
		name: "Admin profile",
		hub: "https://home-fixture.invalid",
	},
	permission: 128,
	editorProps: null as HomeEditorProps | null,
	saveError: undefined as unknown,
	getDefaults: vi.fn(),
	saveDefault: vi.fn(),
}));

vi.mock("next/navigation", () => ({
	useSearchParams: () => new URLSearchParams(),
}));
vi.mock("next/link", () => ({
	default: ({ children, href }: { children: ReactNode; href: string }) => (
		<a href={href}>{children}</a>
	),
}));
vi.mock("react-oidc-context", () => ({
	useAuth: () => ({
		isAuthenticated: true,
		user: { profile: { sub: "admin-user" } },
	}),
}));
vi.mock("../../state/backend-state", () => {
	const userState = {
		async getProfile() {
			return fixture.profile;
		},
		async getInfo() {
			return { permission: fixture.permission };
		},
		getHomeDefaults: fixture.getDefaults,
		saveHomeDefault: fixture.saveDefault,
	};
	return {
		useBackendReady: () => true,
		useBackend: () => ({
			profile: fixture.profile,
			userState,
			apiState: { get: async () => [] },
		}),
	};
});

// Exercise the real admin page and queries at the editor's public boundary.
// HomeEditor/Monaco behavior has separate browser coverage. The shortcut below
// deliberately calls onSave even when the button is disabled, testing the
// admin page's own write guard rather than reproducing the editor's guard.
vi.mock("./home-editor", () => ({
	HomeEditor: function EditorBoundary(props: HomeEditorProps) {
		fixture.editorProps = props;
		const save = async () => {
			fixture.saveError = undefined;
			try {
				await props.onSave(props.layout);
			} catch (error) {
				fixture.saveError = error;
			}
		};
		useEffect(() => {
			const shortcut = (event: KeyboardEvent) => {
				if ((event.ctrlKey || event.metaKey) && event.key === "s") {
					event.preventDefault();
					void save();
				}
			};
			window.addEventListener("keydown", shortcut);
			return () => window.removeEventListener("keydown", shortcut);
		});
		return (
			<section aria-label="Home editor boundary">
				{props.toolbar}
				<button
					type="button"
					disabled={props.disabled}
					onClick={() =>
						props.onEditingChange?.(true, {
							baseRevision: props.draftRevision,
						})
					}
				>
					Edit default
				</button>
				<button
					type="button"
					disabled={Boolean(props.saveBlocked) || props.disabled}
					onClick={() => void save()}
				>
					Publish
				</button>
				<button type="button" onClick={() => props.onEditingChange?.(false)}>
					Discard draft
				</button>
				{props.saveBlocked && <output>{props.saveBlocked}</output>}
			</section>
		);
	},
}));

const layout: IHomeLayout = {
	version: 1,
	title: "Published Home",
	widgets: [],
};
const record = (revision: string): IHomeDefault => ({
	id: "main",
	revision,
	layout,
});
let defaults: IHomeDefaults;
let client: QueryClient;
let root: Root;
let container: HTMLDivElement;

beforeEach(() => {
	Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
	fixture.permission = 128;
	fixture.editorProps = null;
	fixture.saveError = undefined;
	fixture.getDefaults.mockReset();
	fixture.saveDefault.mockReset();
	defaults = { main: record("r1"), profile: null };
	fixture.getDefaults.mockImplementation(async () => structuredClone(defaults));
	fixture.saveDefault.mockImplementation(async () => record("r2"));
	client = new QueryClient({
		defaultOptions: {
			queries: {
				retry: false,
				retryDelay: 0,
				refetchOnWindowFocus: false,
				gcTime: Number.POSITIVE_INFINITY,
			},
		},
	});
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
	localStorage.clear();
});

afterEach(async () => {
	await act(async () => root.unmount());
	client.clear();
	container.remove();
	localStorage.clear();
});

async function settled(assertion: () => void) {
	let lastError: unknown;
	for (let attempt = 0; attempt < 100; attempt++) {
		await act(async () => {
			await new Promise((resolve) => setTimeout(resolve, 10));
		});
		try {
			assertion();
			return;
		} catch (error) {
			lastError = error;
		}
	}
	throw lastError;
}

async function renderPage() {
	await act(async () => {
		root.render(
			<QueryClientProvider client={client}>
				<AdminHomePage />
			</QueryClientProvider>,
		);
	});
	await settled(() => expect(fixture.editorProps).not.toBeNull());
}

function button(label: string) {
	const element = Array.from(container.querySelectorAll("button")).find(
		(candidate) => candidate.textContent === label,
	);
	if (!element) throw new Error(`Missing button: ${label}`);
	return element;
}

async function restoreDraft(baseRevision: string | null | undefined) {
	await act(async () => {
		fixture.editorProps?.onEditingChange?.(true, { baseRevision });
	});
}

async function keyboardSave() {
	await act(async () => {
		window.dispatchEvent(
			new KeyboardEvent("keydown", {
				key: "s",
				ctrlKey: true,
				bubbles: true,
				cancelable: true,
			}),
		);
	});
}

describe("AdminHomePage draft publication", () => {
	it("publishes a current draft with its captured revision", async () => {
		await renderPage();
		expect(fixture.editorProps?.draftRevision).toBe("r1");
		await act(async () => button("Edit default").click());
		expect(button("Publish").disabled).toBe(false);
		await act(async () => button("Publish").click());
		await settled(() => expect(fixture.saveDefault).toHaveBeenCalledOnce());
		expect(fixture.saveDefault).toHaveBeenCalledWith("main", layout, "r1");
		expect(fixture.saveError).toBeUndefined();
	});

	it.each([
		{ published: "r2", restored: "r1" },
		{ published: "r1", restored: null },
		{ published: null, restored: "r1" },
	])(
		"blocks restored revision $restored against published revision $published",
		async ({ published, restored }) => {
			defaults.main = published === null ? null : record(published);
			await renderPage();
			await restoreDraft(restored);
			await settled(() => expect(button("Publish").disabled).toBe(true));
			expect(fixture.editorProps?.saveBlocked).toBeTruthy();
			await keyboardSave();
			expect(fixture.saveDefault).not.toHaveBeenCalled();
			expect(fixture.saveError).toBeInstanceOf(Error);
			await expect(fixture.editorProps?.onReset()).rejects.toThrow(
				"published default changed",
			);
			expect(fixture.saveDefault).not.toHaveBeenCalled();
		},
	);

	it("preserves null when creating a default for the first time", async () => {
		defaults.main = null;
		await renderPage();
		expect(fixture.editorProps?.draftRevision).toBeNull();
		await restoreDraft(null);
		expect(button("Publish").disabled).toBe(false);
		await keyboardSave();
		await settled(() => expect(fixture.saveDefault).toHaveBeenCalledOnce());
		expect(fixture.saveDefault.mock.calls[0]?.[2]).toBeNull();
	});

	it("keeps a 409 blocked even when the cached revision has not changed", async () => {
		fixture.saveDefault.mockRejectedValue(
			new ApiResponseError({ status: 409, message: "Newer default exists" }),
		);
		await renderPage();
		await restoreDraft("r1");
		await keyboardSave();
		await settled(() => expect(button("Publish").disabled).toBe(true));
		expect(fixture.editorProps?.saveBlocked).toBeTruthy();
		expect(container.textContent).toContain(
			"The published default has changed",
		);
		await keyboardSave();
		expect(fixture.saveDefault).toHaveBeenCalledOnce();
	});

	it("retains a stable draft key when the published revision changes", async () => {
		await renderPage();
		const key = fixture.editorProps?.draftKey;
		defaults.main = record("r2");
		await act(async () => {
			await client.invalidateQueries({ queryKey: ["home-defaults"] });
		});
		await settled(() => expect(fixture.editorProps?.draftRevision).toBe("r2"));
		expect(fixture.editorProps?.draftKey).toBe(key);
	});

	it("keeps the recovery editor open when a conflict refetch fails", async () => {
		fixture.saveDefault.mockRejectedValue(
			new ApiResponseError({ status: 409, message: "Newer default exists" }),
		);
		await renderPage();
		fixture.getDefaults.mockRejectedValue(new Error("Connection unavailable"));
		await restoreDraft("r1");
		await keyboardSave();
		await settled(() => expect(fixture.getDefaults).toHaveBeenCalledTimes(3));
		await settled(() => {
			expect(
				container.querySelector('[aria-label="Home editor boundary"]'),
			).not.toBeNull();
			expect(button("Publish").disabled).toBe(true);
		});
		await keyboardSave();
		expect(fixture.saveDefault).toHaveBeenCalledOnce();
	});

	it("waits for the latest default after discard before opening a fresh edit", async () => {
		await renderPage();
		await restoreDraft("older-revision");
		let release: (value: IHomeDefaults) => void = () => {};
		fixture.getDefaults.mockImplementationOnce(
			() =>
				new Promise<IHomeDefaults>((resolve) => {
					release = resolve;
				}),
		);
		await act(async () => button("Discard draft").click());
		await settled(() => expect(fixture.getDefaults).toHaveBeenCalledTimes(2));
		expect(button("Edit default").disabled).toBe(true);
		await act(async () => {
			release({ main: record("latest-revision"), profile: null });
		});
		await settled(() => {
			expect(fixture.editorProps?.draftRevision).toBe("latest-revision");
			expect(button("Edit default").disabled).toBe(false);
		});
		await act(async () => button("Edit default").click());
		await keyboardSave();
		expect(fixture.saveDefault).toHaveBeenCalledWith(
			"main",
			layout,
			"latest-revision",
		);
	});
});
