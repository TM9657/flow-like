import { act, useEffect } from "react";
import { type Root, createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	MobileHeader,
	MobileHeaderProvider,
	useMobileHeader,
} from "../ui/mobile-header";
import HomeAppEmbedRuntime from "./home-content/app-embed-runtime";
import type { HomeEmbedTarget } from "./home-content/config";

vi.mock("@flow-like/flow-like-ui", () => ({
	SidebarTrigger: () => <button type="button">Open menu</button>,
}));
vi.mock("@flow-like/locales", () => ({
	useTranslation: () => ({ t: (_key: string, fallback: string) => fallback }),
}));
vi.mock("../../lib/event-config-use", () => ({ USE_EVENT_CONFIG: {} }));

// Model the interface's toolbar registration while retaining the real runtime
// boundary, mobile header provider, and mobile toolbar rendering.
vi.mock("../interfaces/use-page-content", () => ({
	UsePageContent: function EmbeddedInterface({
		appId,
		onNavigate,
	}: {
		appId: string;
		onNavigate: (next: { routePath: string }) => void;
	}) {
		const { update } = useMobileHeader();
		useEffect(() => {
			update({
				left: (
					<button type="button" onClick={() => onNavigate({ routePath: "/" })}>
						{appId} Home
					</button>
				),
			});
		}, [appId, onNavigate, update]);
		return <div>{appId} content</div>;
	},
}));

function WorkspaceControls() {
	useMobileHeader({ left: <button type="button">Workspace</button> }, []);
	return null;
}

let root: Root;
let container: HTMLDivElement;

const target = (appId: string): HomeEmbedTarget => ({
	appId,
	routePath: "/",
	eventId: null,
	queryParams: {},
});

beforeEach(() => {
	Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
});

afterEach(async () => {
	await act(async () => root.unmount());
	container.remove();
	document.documentElement.style.removeProperty("--mobile-header-height");
});

describe("Home app widget mobile controls", () => {
	it("keeps each widget's navigation usable without replacing the workspace toolbar", async () => {
		const firstNavigate = vi.fn();
		const secondNavigate = vi.fn();
		const render = async (showFirst: boolean) => {
			await act(async () => {
				root.render(
					<MobileHeaderProvider>
						<WorkspaceControls />
						<section data-workspace>
							<MobileHeader showSidebarTrigger={false} />
						</section>
						{showFirst && (
							<section data-first>
								<HomeAppEmbedRuntime
									target={target("First app")}
									active
									onNavigate={firstNavigate}
								/>
							</section>
						)}
						<section data-second>
							<HomeAppEmbedRuntime
								target={target("Second app")}
								active
								onNavigate={secondNavigate}
							/>
						</section>
					</MobileHeaderProvider>,
				);
			});
		};

		await render(true);
		expect(container.querySelector("[data-workspace]")?.textContent).toBe(
			"Workspace",
		);
		const firstButton = container.querySelector<HTMLButtonElement>(
			"[data-first] button",
		);
		const secondButton = container.querySelector<HTMLButtonElement>(
			"[data-second] button",
		);
		expect(firstButton?.textContent).toBe("First app Home");
		expect(secondButton?.textContent).toBe("Second app Home");
		await act(async () => firstButton?.click());
		expect(firstNavigate).toHaveBeenCalledWith({ routePath: "/" });
		expect(secondNavigate).not.toHaveBeenCalled();
		await act(async () => secondButton?.click());
		expect(secondNavigate).toHaveBeenCalledWith({ routePath: "/" });

		await render(false);
		expect(container.querySelector("[data-workspace]")?.textContent).toBe(
			"Workspace",
		);
		expect(container.querySelector("[data-second] button")?.textContent).toBe(
			"Second app Home",
		);
	});

	it("leaves the page's mobile header offset unchanged when a widget mounts or unmounts", async () => {
		document.documentElement.style.setProperty(
			"--mobile-header-height",
			"73px",
		);
		await act(async () => {
			root.render(
				<HomeAppEmbedRuntime
					target={target("Embedded app")}
					active
					onNavigate={vi.fn()}
				/>,
			);
		});
		expect(container.querySelector("button")?.textContent).toBe(
			"Embedded app Home",
		);
		expect(
			document.documentElement.style.getPropertyValue("--mobile-header-height"),
		).toBe("73px");

		await act(async () => root.render(null));
		expect(
			document.documentElement.style.getPropertyValue("--mobile-header-height"),
		).toBe("73px");
	});
});
