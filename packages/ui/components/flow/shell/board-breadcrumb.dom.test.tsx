import {
	afterAll,
	afterEach,
	beforeEach,
	describe,
	expect,
	mock,
	test,
} from "bun:test";
import { Window } from "happy-dom";
import { type ComponentProps, act } from "react";
import { BoardBreadcrumb } from "./board-breadcrumb";

const testWindow = new Window({ url: "https://localhost" });
Object.assign(testWindow, { SyntaxError, TypeError, Error });
const domGlobals = {
	window: testWindow,
	document: testWindow.document,
	navigator: testWindow.navigator,
	HTMLElement: testWindow.HTMLElement,
	Element: testWindow.Element,
	Node: testWindow.Node,
	IS_REACT_ACT_ENVIRONMENT: true,
};
const previousGlobals = new Map(
	Object.keys(domGlobals).map((key) => [
		key,
		Object.getOwnPropertyDescriptor(globalThis, key),
	]),
);
const installDom = () => Object.assign(globalThis, domGlobals);
installDom();

const { createRoot } = await import("react-dom/client");
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;

beforeEach(() => {
	installDom();
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
});

afterEach(() => {
	act(() => root.unmount());
	container.remove();
});

afterAll(() => {
	for (const [key, descriptor] of previousGlobals) {
		if (descriptor) Object.defineProperty(globalThis, key, descriptor);
		else Reflect.deleteProperty(globalThis, key);
	}
	testWindow.happyDOM.abort();
});

function render(props: Partial<ComponentProps<typeof BoardBreadcrumb>> = {}) {
	const onJumpToLayer = mock((_path: string) => {});
	act(() =>
		root.render(
			<BoardBreadcrumb
				fileLabel="main.flow"
				layerNames={new Map()}
				onJumpToLayer={onJumpToLayer}
				{...props}
			/>,
		),
	);
	return {
		onJumpToLayer,
		buttons: Array.from(container.querySelectorAll("button")),
	};
}

describe("file breadcrumbs", () => {
	test("keeps main-file ancestor destinations and disables the current layer", () => {
		const { buttons, onJumpToLayer } = render({
			layerPath: "outer/inner",
			layerNames: new Map([
				["outer", "Prepare"],
				["inner", "Transform"],
			]),
		});
		expect(buttons.map((button) => button.textContent)).toEqual([
			"main.flow",
			"Prepare",
			"Transform",
		]);
		act(() => {
			for (const button of buttons) button.click();
		});
		expect(onJumpToLayer.mock.calls).toEqual([["root"], ["outer"]]);
		expect(buttons[2].disabled).toBe(true);
		expect(buttons[2].getAttribute("aria-current")).toBe("page");
	});

	test("returns from a module function to that module", () => {
		const { buttons, onJumpToLayer } = render({
			fileLabel: "payments.flow",
			fileRootPath: "payments",
			layerPath: "payments/charge",
			layerNames: new Map([
				["payments", "Payments"],
				["charge", "Charge"],
			]),
		});
		expect(buttons.map((button) => button.textContent)).toEqual([
			"payments.flow",
			"Charge",
		]);
		act(() => buttons[0].click());
		expect(onJumpToLayer.mock.calls).toEqual([["payments"]]);
	});

	test("shows only nested-module contents and keeps full ancestor paths", () => {
		const { buttons, onJumpToLayer } = render({
			fileLabel: "checkout/payments.flow",
			fileRootPath: "checkout/payments",
			layerPath: "checkout/payments/prepare/validate",
			layerNames: new Map([
				["prepare", "Prepare"],
				["validate", "Validate"],
			]),
		});
		expect(buttons.map((button) => button.textContent)).toEqual([
			"checkout/payments.flow",
			"Prepare",
			"Validate",
		]);
		act(() => {
			buttons[0].click();
			buttons[1].click();
		});
		expect(onJumpToLayer.mock.calls).toEqual([
			["checkout/payments"],
			["checkout/payments/prepare"],
		]);
	});

	test.each([
		{},
		{ fileRootPath: "root", layerPath: "root" },
		{ fileRootPath: "payments", layerPath: "payments" },
		{ fileRootPath: "checkout/payments", layerPath: "checkout/payments" },
	])("renders nothing at file root %j", (props) => {
		render(props);
		expect(container.querySelector("nav")).toBeNull();
	});

	test("does not expose ancestors for a stale file/path pair", () => {
		render({
			fileRootPath: "payments",
			layerPath: "payments-copy/charge",
		});
		expect(container.querySelector("nav")).toBeNull();
	});
});
