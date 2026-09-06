import { describe, expect, test } from "bun:test";
import { Window } from "happy-dom";
import { act } from "react";
import { GEOMETRY_BOARD_FORMAT_VERSION } from "../../../lib/board-format";
import { geometryMarker } from "../../../lib/geometry";
import type { IBackendState } from "../../../state/backend-state";

async function setup() {
	const window = new Window();
	Object.assign(window, { SyntaxError, TypeError });
	Object.assign(globalThis, {
		document: window.document,
		Element: window.Element,
		Event: window.Event,
		HTMLElement: window.HTMLElement,
		HTMLTextAreaElement: window.HTMLTextAreaElement,
		MouseEvent: window.MouseEvent,
		Node: window.Node,
		navigator: window.navigator,
		window,
		Document: window.Document,
		DocumentFragment: window.DocumentFragment,
		Text: window.Text,
		getComputedStyle: window.getComputedStyle.bind(window),
		requestAnimationFrame: window.requestAnimationFrame.bind(window),
		cancelAnimationFrame: window.cancelAnimationFrame.bind(window),
		IS_REACT_ACT_ENVIRONMENT: true,
	});
	const { createRoot } = await import("react-dom/client");
	const { GeometryValueInput } = await import("./geometry-variable");
	const container = window.document.createElement("div");
	window.document.body.append(container);
	return {
		window,
		container,
		root: createRoot(container as unknown as HTMLElement),
		GeometryValueInput,
	};
}

describe("Geometry editor rendering", () => {
	test("shows subtype errors and preserves the visible invalid draft", async () => {
		const { container, root, GeometryValueInput } = await setup();
		await act(async () => {
			root.render(
				<GeometryValueInput
					value={{
						type: "LineString",
						coordinates: [
							[0, 0],
							[1, 1],
						],
					}}
					schema={geometryMarker("Point")}
					preview={false}
					onChange={() => {}}
				/>,
			);
		});
		expect(
			container.querySelector("textarea")?.getAttribute("aria-invalid"),
		).toBe("true");
		expect(container.querySelector('[role="alert"]')?.textContent).toContain(
			"Point",
		);
		expect(container.querySelector("textarea")?.value).toContain("LineString");
		await act(async () => root.unmount());
	});

	test("renders unset values without inventing a coordinate and conceals secrets", async () => {
		const { container, root, GeometryValueInput } = await setup();
		await act(async () =>
			root.render(
				<GeometryValueInput
					value={null}
					schema={geometryMarker("Point")}
					preview={false}
					onChange={() => {}}
				/>,
			),
		);
		expect(container.querySelector("textarea")?.value).toBe("");
		expect(container.querySelector('[role="alert"]')).toBeNull();
		await act(async () =>
			root.render(
				<GeometryValueInput
					value={{ type: "Point", coordinates: [13, 52] }}
					secret
					preview={false}
					onChange={() => {}}
				/>,
			),
		);
		expect(container.querySelector("textarea")).toBeNull();
		expect(container.querySelector('input[type="password"]')).not.toBeNull();
		await act(async () => container.querySelector("button")?.click());
		expect(container.querySelector("textarea")?.value).toContain("Point");
		await act(async () => root.unmount());
	});
});

test("Geometry creation requires a compatible backend board format", async () => {
	const { container, root } = await setup();
	const { useBoardFormat } = await import("../../../hooks/use-board-format");
	const { useBackendStore } = await import("../../../state/backend-state");
	const previous = useBackendStore.getState().backend;
	const probe = (appId: string) => {
		function Probe() {
			return (
				<span>
					{useBoardFormat(appId) >= GEOMETRY_BOARD_FORMAT_VERSION
						? "enabled"
						: "disabled"}
				</span>
			);
		}
		return <Probe />;
	};
	try {
		await act(async () => {
			useBackendStore.setState({
				backend: { boardState: {} } as unknown as IBackendState,
			});
			root.render(probe("old"));
		});
		expect(container.textContent).toBe("disabled");
		await act(async () => {
			useBackendStore.setState({
				backend: {
					boardState: {
						getBoardFormat: async (appId: string) => ({
							board_format_version: appId === "new" ? 2 : 1,
						}),
					},
				} as unknown as IBackendState,
			});
			root.render(probe("new"));
		});
		expect(container.textContent).toBe("enabled");
		await act(async () => root.render(probe("old")));
		expect(container.textContent).toBe("disabled");
	} finally {
		await act(async () => root.unmount());
		useBackendStore.setState({ backend: previous });
	}
});

test("board format negotiation ignores stale responses after switching apps", async () => {
	const { container, root } = await setup();
	const { useBoardFormat } = await import("../../../hooks/use-board-format");
	const { useBackendStore } = await import("../../../state/backend-state");
	const previous = useBackendStore.getState().backend;
	let resolveFirst!: (value: { board_format_version: number }) => void;
	const first = new Promise<{ board_format_version: number }>((resolve) => {
		resolveFirst = resolve;
	});
	function Probe({ appId }: { appId: string }) {
		return <span>{useBoardFormat(appId)}</span>;
	}
	try {
		await act(async () => {
			useBackendStore.setState({
				backend: {
					boardState: {
						getBoardFormat: async (appId: string) => {
							if (appId === "first") return first;
							if (appId === "offline") throw new Error("Unavailable");
							return { board_format_version: 3 };
						},
					},
				} as unknown as IBackendState,
			});
			root.render(<Probe appId="first" />);
		});
		expect(container.textContent).toBe("1");
		await act(async () => root.render(<Probe appId="second" />));
		expect(container.textContent).toBe("3");
		await act(async () => resolveFirst({ board_format_version: 2 }));
		expect(container.textContent).toBe("3");
		await act(async () => root.render(<Probe appId="offline" />));
		expect(container.textContent).toBe("1");
	} finally {
		await act(async () => root.unmount());
		useBackendStore.setState({ backend: previous });
	}
});
