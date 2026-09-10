import {
	QueryClient,
	QueryClientProvider,
	focusManager,
} from "@tanstack/react-query";
import { act } from "react";
import { type Root, createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useHomeStorageImage } from "./use-home-storage-image";

const fixture = vi.hoisted(() => ({
	scope: ["origin", "profile", "viewer"],
	ready: true,
	download: vi.fn(),
}));
vi.mock("../../state/backend-state", () => ({
	useBackend: () => ({
		storageState: { downloadStorageItems: fixture.download },
	}),
	useBackendReady: () => fixture.ready,
}));
vi.mock("./home-content/shared", () => ({ useHomeScope: () => fixture.scope }));

let root: Root;
let container: HTMLDivElement;
let client: QueryClient;
function Image({ appId = "app", path = "image.png" }) {
	const result = useHomeStorageImage(appId, path);
	return (
		<>
			<output>{result.src ?? (result.error ? "error" : "loading")}</output>
			<button type="button" onClick={result.retry}>
				Retry
			</button>
		</>
	);
}

beforeEach(() => {
	Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
	vi.useFakeTimers();
	focusManager.setFocused(true);
	fixture.scope = ["origin", "profile", "viewer"];
	fixture.ready = true;
	fixture.download.mockReset();
	client = new QueryClient({
		defaultOptions: {
			queries: { retry: false, gcTime: Number.POSITIVE_INFINITY },
		},
	});
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
});

afterEach(async () => {
	await act(async () => root.unmount());
	client.clear();
	container.remove();
	focusManager.setFocused(undefined);
	vi.useRealTimers();
});

async function render(appId = "app", path = "image.png") {
	await act(async () =>
		root.render(
			<QueryClientProvider client={client}>
				<Image appId={appId} path={path} />
			</QueryClientProvider>,
		),
	);
	await act(async () => vi.advanceTimersByTimeAsync(1));
}
const displayed = () => container.querySelector("output")?.textContent;
const result = (url: string, prefix = "image.png") => [{ prefix, url }];
const signed = (minutes: number, signature: string) =>
	`https://images.example/image.png?se=${new Date(Date.now() + minutes * 60_000).toISOString()}&sig=${signature}`;

async function retry() {
	await act(async () => container.querySelector("button")?.click());
	await act(async () => vi.advanceTimersByTimeAsync(1));
}

describe("Home signed image URLs", () => {
	it.each(["app", "path", "viewer"])(
		"ignores an old request after switching %s",
		async (change) => {
			let settle: (value: ReturnType<typeof result>) => void = () => {};
			fixture.download.mockImplementationOnce(
				() =>
					new Promise((resolve) => {
						settle = resolve;
					}),
			);
			fixture.download.mockImplementation(async (_app, paths) =>
				result("https://new/image.png", paths[0]),
			);
			await render();
			if (change === "viewer") fixture.scope = ["origin", "profile", "other"];
			await render(
				change === "app" ? "other" : "app",
				change === "path" ? "other.png" : "image.png",
			);
			expect(displayed()).toBe("https://new/image.png");
			await act(async () => settle(result("https://old/image.png")));
			expect(displayed()).toBe("https://new/image.png");
		},
	);

	it("refreshes before signature expiry without downloading image bytes", async () => {
		const first = signed(6, "first");
		const second = signed(60, "second");
		fixture.download
			.mockResolvedValueOnce(result(first))
			.mockResolvedValue(result(second));
		await render();
		expect(displayed()).toBe(first);
		await act(async () => vi.advanceTimersByTimeAsync(60_000));
		expect(displayed()).toBe(second);
		expect(fixture.download).toHaveBeenCalledTimes(2);
		expect(fixture.download).toHaveBeenLastCalledWith("app", ["image.png"]);
	});

	it("does not poll unsigned desktop URLs", async () => {
		fixture.download.mockResolvedValue(result("asset://localhost/image.png"));
		await render();
		await act(async () => vi.advanceTimersByTimeAsync(3_600_000));
		expect(displayed()).toBe("asset://localhost/image.png");
		expect(fixture.download).toHaveBeenCalledTimes(1);
	});

	it("hides cached URLs when signing fails and allows an explicit retry", async () => {
		fixture.download.mockResolvedValueOnce(
			result("https://images.example/image.png"),
		);
		await render();
		fixture.download.mockRejectedValue(new Error("403 Forbidden"));
		await retry();
		expect(displayed()).toBe("error");
		await act(async () => vi.advanceTimersByTimeAsync(180_000));
		expect(fixture.download).toHaveBeenCalledTimes(2);
		fixture.download.mockResolvedValue(
			result("https://images.example/restored.png"),
		);
		await retry();
		expect(displayed()).toBe("https://images.example/restored.png");
	});

	it("does not show expired URLs restored from the query cache", async () => {
		client.setQueryData(
			["home", ...fixture.scope, "storage-image-url", "app", "image.png"],
			signed(-1, "expired"),
		);
		let settle: (value: ReturnType<typeof result>) => void = () => {};
		fixture.download.mockImplementation(
			() =>
				new Promise((resolve) => {
					settle = resolve;
				}),
		);
		await render();
		expect(displayed()).toBe("loading");
		const fresh = signed(60, "fresh");
		await act(async () => settle(result(fresh)));
		await act(async () => vi.advanceTimersByTimeAsync(1));
		expect(displayed()).toBe(fresh);
	});

	it("hides cached data while the backend is unavailable", async () => {
		fixture.download.mockResolvedValue(
			result("https://images.example/image.png"),
		);
		await render();
		fixture.ready = false;
		await render();
		expect(displayed()).toBe("loading");
		expect(fixture.download).toHaveBeenCalledTimes(1);
	});
});
