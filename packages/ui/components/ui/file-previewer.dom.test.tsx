import { act } from "react";
import { type Root, createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { FilePreviewer } from "./file-previewer";

vi.mock("@flow-like/locales", () => ({
	useTranslation: () => ({ t: (_key: string, fallback: string) => fallback }),
}));
vi.mock("./audio-preview", () => ({ AudioPreview: () => null }));
vi.mock("./monaco-file-editor", () => ({ MonacoFileEditor: () => null }));
vi.mock("./text-editor", () => ({ TextEditor: () => null }));

const svg =
	'<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect width="120" height="80" fill="red"/></svg>';
let container: HTMLDivElement;
let root: Root | null;
const fetchFile = vi.fn();

beforeEach(() => {
	Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
	fetchFile.mockReset().mockImplementation(
		async () =>
			new Response(svg, {
				headers: {
					"Content-Type": "application/octet-stream",
					"Content-Disposition": "attachment",
				},
			}),
	);
	vi.stubGlobal("fetch", fetchFile);
	vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:svg-preview");
	vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
});

afterEach(async () => {
	if (root) await act(async () => root?.unmount());
	container.remove();
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

async function render(url = "/storage/image.svg") {
	await act(async () =>
		root?.render(<FilePreviewer url={url} filename="image.svg" />),
	);
}

it("previews SVG storage attachments as SVG images without embedding their markup", async () => {
	await render();
	expect(fetchFile).toHaveBeenCalledWith("/storage/image.svg", {
		cache: "no-store",
	});
	const blob = vi.mocked(URL.createObjectURL).mock.calls[0][0] as Blob;
	expect(blob.type).toBe("image/svg+xml");
	expect(await blob.text()).toBe(svg);
	expect(container.querySelector("img")?.getAttribute("src")).toBe(
		"blob:svg-preview",
	);
	expect(container.querySelector("svg, iframe, object")).toBeNull();
});

it("releases SVG URLs when switching files and closing the preview", async () => {
	vi.mocked(URL.createObjectURL)
		.mockReturnValueOnce("blob:first")
		.mockReturnValueOnce("blob:second");
	await render("/first.svg");
	await render("/second.svg");
	expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:first");
	expect(fetchFile).toHaveBeenLastCalledWith("/second.svg", {
		cache: "no-store",
	});
	await act(async () => root?.unmount());
	root = null;
	expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:second");
});

it("keeps a direct SVG source available when fetching is unavailable", async () => {
	fetchFile.mockRejectedValueOnce(new Error("custom asset protocol"));
	vi.spyOn(console, "warn").mockImplementation(() => {});
	await render("asset://local/image.svg");
	expect(container.querySelector("img")?.getAttribute("src")).toBe(
		"asset://local/image.svg",
	);
	expect(URL.createObjectURL).not.toHaveBeenCalled();
});
