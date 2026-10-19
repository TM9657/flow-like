// @vitest-environment happy-dom
import { act } from "react";
import { type Root, createRoot } from "react-dom/client";
import { AuthContext } from "react-oidc-context";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SavedStudioForecast } from "../../../../state/backend-state/query-state";
import { useSavedForecast, useSavedForecasts } from "./use-saved-forecast";

const fixture = vi.hoisted(() => ({
	ready: true,
	origin: "https://hub.example",
	profile: { id: "profile-a" },
	query: { getForecast: vi.fn(), listForecasts: vi.fn() },
}));
vi.mock("../../../../state/backend-state", () => ({
	useBackend: () => ({ profile: fixture.profile, queryState: fixture.query }),
	useBackendReady: () => fixture.ready,
}));
vi.mock("../../../../lib/api-url", () => ({
	getApiOrigin: () => fixture.origin,
}));

let root: Root;
let container: HTMLDivElement;
let auth: {
	isAuthenticated: boolean;
	isLoading: boolean;
	user: { profile: { sub: string } };
};
let renders: string[];
type Selection = { appId?: string; forecastId?: string; userScoped?: boolean };

function Forecast({
	appId = "app-a",
	forecastId = "forecast-a",
	userScoped = false,
}: Selection) {
	const resource = useSavedForecast(appId, forecastId, userScoped);
	const text = resource.data?.name ?? (resource.isError ? "error" : "loading");
	renders.push(text);
	return (
		<>
			<output>{text}</output>
			<button type="button" onClick={resource.refetch}>
				Refresh
			</button>
		</>
	);
}

function ForecastList({ appId = "app-a", userScoped = false }: Selection) {
	const resource = useSavedForecasts(appId, userScoped);
	return (
		<output>
			{resource.isError
				? "error"
				: (resource.data?.map((item) => item.name).join(",") ?? "loading")}
		</output>
	);
}

function snapshot(name: string, appId = "app-a", id = "forecast-a") {
	return { id, app_id: appId, name } as SavedStudioForecast;
}

function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((success, failure) => {
		resolve = success;
		reject = failure;
	});
	return { promise, resolve, reject };
}

async function render(selection: Selection = {}, list = false) {
	await act(async () => {
		root.render(
			<AuthContext.Provider value={auth as never}>
				{list ? <ForecastList {...selection} /> : <Forecast {...selection} />}
			</AuthContext.Provider>,
		);
	});
}
const displayed = () => container.querySelector("output")?.textContent;
const refresh = async () => {
	await act(async () => container.querySelector("button")?.click());
};

beforeEach(() => {
	Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
	fixture.ready = true;
	fixture.origin = "https://hub.example";
	fixture.profile = { id: "profile-a" };
	fixture.query = { getForecast: vi.fn(), listForecasts: vi.fn() };
	auth = {
		isAuthenticated: true,
		isLoading: false,
		user: { profile: { sub: "viewer-a" } },
	};
	renders = [];
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
});

afterEach(async () => {
	await act(async () => root.unmount());
	container.remove();
});

describe("saved forecast access identity", () => {
	it("refreshes the saved picker only for agent saves in its app and data scope", async () => {
		fixture.query.listForecasts.mockResolvedValueOnce([snapshot("Before")]);
		await render({}, true);
		for (const detail of [
			{ appId: "other", userScoped: false },
			{ appId: "app-a", userScoped: true },
		])
			await act(async () =>
				window.dispatchEvent(
					new CustomEvent("flow-like:forecast-saved", { detail }),
				),
			);
		expect(fixture.query.listForecasts).toHaveBeenCalledTimes(1);
		fixture.query.listForecasts.mockResolvedValueOnce([
			snapshot("Before"),
			snapshot("Agent prediction", "app-a", "new"),
		]);
		await act(async () =>
			window.dispatchEvent(
				new CustomEvent("flow-like:forecast-saved", {
					detail: { appId: "app-a", userScoped: false },
				}),
			),
		);
		expect(displayed()).toBe("Before,Agent prediction");
		expect(fixture.query.listForecasts).toHaveBeenCalledTimes(2);
	});
	it("ignores an earlier app and forecast response after navigating", async () => {
		const old = deferred<SavedStudioForecast>();
		fixture.query.getForecast
			.mockReturnValueOnce(old.promise)
			.mockResolvedValueOnce(snapshot("New", "app-b", "forecast-b"));
		await render();
		await render({ appId: "app-b", forecastId: "forecast-b" });
		expect(displayed()).toBe("New");
		await act(async () => old.resolve(snapshot("Old")));
		expect(displayed()).toBe("New");
		expect(renders).not.toContain("Old");
		expect(fixture.query.getForecast).toHaveBeenLastCalledWith(
			"app-b",
			"forecast-b",
			false,
		);
	});

	it("hides the project snapshot immediately when switching to personal scope", async () => {
		const personal = deferred<SavedStudioForecast>();
		fixture.query.getForecast
			.mockResolvedValueOnce(snapshot("Project"))
			.mockReturnValueOnce(personal.promise);
		await render();
		expect(displayed()).toBe("Project");
		renders = [];
		await render({ userScoped: true });
		expect(renders).not.toContain("Project");
		expect(displayed()).toBe("loading");
		expect(fixture.query.getForecast).toHaveBeenLastCalledWith(
			"app-a",
			"forecast-a",
			true,
		);
		await act(async () => personal.resolve(snapshot("Personal")));
		expect(displayed()).toBe("Personal");
	});

	it("hides prior values on viewer, profile, origin and logout changes", async () => {
		fixture.query.getForecast.mockResolvedValueOnce(snapshot("Private"));
		await render();
		const changes = [
			() => {
				auth = { ...auth, user: { profile: { sub: "viewer-b" } } };
			},
			() => {
				fixture.profile = { id: "profile-b" };
			},
			() => {
				fixture.origin = "https://other.example";
			},
			() => {
				auth = { ...auth, isAuthenticated: false };
			},
		];
		for (const change of changes) {
			const next = deferred<SavedStudioForecast>();
			fixture.query.getForecast.mockReturnValueOnce(next.promise);
			renders = [];
			change();
			await render();
			expect(renders).not.toContain("Private");
			expect(displayed()).toBe("loading");
			await act(async () => next.resolve(snapshot("Private")));
		}
		expect(auth.user.profile.sub).toBe("viewer-b");
		expect(fixture.query.getForecast).toHaveBeenCalledTimes(5);
	});

	it("revalidates when the backend owner changes and ignores that owner's late reply", async () => {
		const old = deferred<SavedStudioForecast>();
		fixture.query.getForecast.mockReturnValueOnce(old.promise);
		await render();
		fixture.query = {
			getForecast: vi.fn().mockResolvedValue(snapshot("Replacement")),
			listForecasts: vi.fn(),
		};
		await render();
		await act(async () => old.resolve(snapshot("Old backend")));
		expect(displayed()).toBe("Replacement");
		expect(renders).not.toContain("Old backend");
	});

	it("hides loaded data when a manual or focus refresh is denied", async () => {
		fixture.query.getForecast
			.mockResolvedValueOnce(snapshot("Allowed"))
			.mockRejectedValueOnce(new Error("403 Forbidden"));
		await render();
		renders = [];
		await refresh();
		expect(renders).not.toContain("Allowed");
		expect(displayed()).toBe("error");
		fixture.query.getForecast.mockResolvedValueOnce(snapshot("Restored"));
		await refresh();
		expect(displayed()).toBe("Restored");
		fixture.query.getForecast.mockRejectedValueOnce(new Error("404 Not found"));
		renders = [];
		await act(async () => window.dispatchEvent(new Event("focus")));
		expect(displayed()).toBe("error");
		expect(renders).not.toContain("Restored");
	});

	it("clears list data on a scope failure and ignores the previous list response", async () => {
		const old = deferred<SavedStudioForecast[]>();
		fixture.query.listForecasts
			.mockReturnValueOnce(old.promise)
			.mockRejectedValueOnce(new Error("403 Forbidden"));
		await render({}, true);
		await render({ userScoped: true }, true);
		await act(async () => old.resolve([snapshot("Private project forecast")]));
		expect(displayed()).toBe("error");
		expect(fixture.query.listForecasts).toHaveBeenLastCalledWith("app-a", true);
	});

	it("does not load or reveal prior data while auth or the backend is unready", async () => {
		fixture.query.getForecast.mockResolvedValue(snapshot("Private"));
		await render();
		fixture.ready = false;
		renders = [];
		await render();
		expect(renders).not.toContain("Private");
		fixture.ready = true;
		auth = { ...auth, isLoading: true };
		await render();
		expect(displayed()).toBe("loading");
		const reauthorized = deferred<SavedStudioForecast>();
		fixture.query.getForecast.mockReturnValueOnce(reauthorized.promise);
		auth = { ...auth, isLoading: false };
		renders = [];
		await render();
		expect(renders).not.toContain("Private");
		expect(displayed()).toBe("loading");
		await render({ forecastId: "" });
		await render({ appId: "" });
		expect(fixture.query.getForecast).toHaveBeenCalledTimes(2);
		await act(async () => reauthorized.resolve(snapshot("Old authorization")));
		expect(displayed()).toBe("loading");
	});

	it("rejects dot-only references and responses for another app or forecast", async () => {
		for (const unsafeId of [".", ".."]) {
			await render({ appId: unsafeId });
			expect(displayed()).toBe("error");
			await render({ forecastId: unsafeId });
			expect(displayed()).toBe("error");
		}
		expect(fixture.query.getForecast).not.toHaveBeenCalled();
		await render({ appId: ".." }, true);
		expect(displayed()).toBe("error");
		expect(fixture.query.listForecasts).not.toHaveBeenCalled();
		fixture.query.getForecast
			.mockResolvedValueOnce(snapshot("Other app", "app-b"))
			.mockResolvedValueOnce(snapshot("Other forecast", "app-a", "forecast-b"));
		await render();
		expect(displayed()).toBe("error");
		await refresh();
		expect(displayed()).toBe("error");
		fixture.query.listForecasts.mockResolvedValueOnce([
			snapshot("Other app", "app-b"),
		]);
		await render({}, true);
		expect(displayed()).toBe("error");
	});
});
