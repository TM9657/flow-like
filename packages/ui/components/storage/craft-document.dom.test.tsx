import type { Window as HappyWindow } from "happy-dom";
import { type ComponentProps, type ReactNode, act } from "react";
import { type Root, createRoot } from "react-dom/client";
import {
	type Mock,
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	vi,
} from "vitest";
import { CraftDocument } from "./craft-document";
import { craftViewer } from "./craft-viewers";
const CRAFT_CHANNEL = "flow-like-craft";
const XLSX_MIME = requireViewer("report.xlsx").mime;

vi.mock("../ui/button", () => ({
	Button: ({
		asChild,
		children,
		variant: _variant,
		size: _size,
		...props
	}: ComponentProps<"button"> & {
		asChild?: boolean;
		children?: ReactNode;
		variant?: string;
		size?: string;
	}) =>
		asChild ? (
			<>{children}</>
		) : (
			<button type="button" {...props}>
				{children}
			</button>
		),
}));

function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((resolvePromise, rejectPromise) => {
		resolve = resolvePromise;
		reject = rejectPromise;
	});
	return { promise, resolve, reject };
}

const originalBytes = new Uint8Array([80, 75, 3, 4, 1, 2]);
const editedBytes = new Uint8Array([80, 75, 3, 4, 9, 8]);
let root: Root | null;
let container: HTMLDivElement;
let frameWindow: Window;
let send: ReturnType<typeof vi.fn>;
let fetchDocument: ReturnType<typeof vi.fn>;
let saveDocument: Mock<(file: File) => Promise<void>>;
let dirtyChange: Mock<(dirty: boolean) => void>;
let busyChange: Mock<(busy: boolean) => void>;
let iframeLoadingDisabled: boolean;

beforeEach(() => {
	Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
	const settings = (window as unknown as HappyWindow).happyDOM.settings;
	iframeLoadingDisabled = settings.navigation.disableChildFrameNavigation;
	settings.navigation.disableChildFrameNavigation = true;
	send = vi.fn();
	frameWindow = { postMessage: send } as unknown as Window;
	vi.spyOn(HTMLIFrameElement.prototype, "contentWindow", "get").mockReturnValue(
		frameWindow,
	);
	fetchDocument = vi.fn().mockResolvedValue(new Response(originalBytes));
	vi.stubGlobal("fetch", fetchDocument);
	saveDocument = vi.fn().mockResolvedValue(undefined);
	dirtyChange = vi.fn();
	busyChange = vi.fn();
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
});

afterEach(async () => {
	if (root) await act(async () => root?.unmount());
	root = null;
	container.remove();
	const settings = (window as unknown as HappyWindow).happyDOM.settings;
	settings.navigation.disableChildFrameNavigation = iframeLoadingDisabled;
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
	vi.unstubAllEnvs();
	vi.useRealTimers();
});

async function render(editable = true, url = "/files/report.xlsx") {
	await act(async () => {
		root?.render(
			<CraftDocument
				viewer={requireViewer("report.xlsx")}
				url={url}
				filename="report.xlsx"
				editable={editable}
				onSave={saveDocument}
				onDirtyChange={dirtyChange}
				onBusyChange={busyChange}
			/>,
		);
	});
}

function session() {
	const frame = container.querySelector("iframe");
	if (!frame) throw new Error("The viewer frame is not mounted");
	return new URL(frame.src, window.location.href).searchParams.get("session");
}

async function receive(
	message: Record<string, unknown>,
	overrides: MessageEventInit = {},
) {
	await act(async () => {
		window.dispatchEvent(
			new MessageEvent("message", {
				data: { channel: CRAFT_CHANNEL, session: session(), ...message },
				origin: window.location.origin,
				source: frameWindow,
				...overrides,
			}),
		);
	});
}

function messages(type: string) {
	return send.mock.calls
		.map(([message]) => message as Record<string, unknown>)
		.filter((message) => message.type === type);
}

function button(label: string) {
	const match = [...container.querySelectorAll("button")].find((item) =>
		item.textContent?.includes(label),
	);
	if (!match) throw new Error(`Missing button: ${label}`);
	return match;
}

async function click(label: string) {
	await act(async () => button(label).click());
}

async function openForEditing() {
	await render();
	await receive({ type: "ready" });
	await receive({ type: "opened" });
	await click("Edit");
	await receive({ type: "dirty", dirty: true });
}

async function exportDocument(revision = 4) {
	await click("Save");
	const request = messages("export").at(-1);
	if (!request) throw new Error("No export requested");
	await receive({
		type: "exported",
		requestId: request.requestId,
		bytes: editedBytes.slice().buffer,
		revision,
	});
	return request.requestId;
}

describe("Flows Sheets document host", () => {
	it("waits for both viewer readiness and downloaded bytes before opening", async () => {
		const download = deferred<Response>();
		fetchDocument.mockReturnValue(download.promise);
		await render();
		await receive({ type: "ready" });
		expect(messages("open")).toHaveLength(0);
		await act(async () => download.resolve(new Response(originalBytes)));
		expect(messages("open")).toHaveLength(1);
		const open = messages("open")[0];
		expect(open).toMatchObject({ name: "report.xlsx", readOnly: true });
		expect(new Uint8Array(open.bytes as ArrayBuffer)).toEqual(originalBytes);
		expect(send).toHaveBeenCalledWith(open, window.location.origin, [
			open.bytes,
		]);
		await receive({ type: "ready" });
		expect(messages("open")).toHaveLength(1);
	});

	it("does not open downloaded bytes until the active iframe sends ready", async () => {
		await render();
		expect(fetchDocument).toHaveBeenCalledWith(
			"/files/report.xlsx",
			expect.objectContaining({ cache: "no-store" }),
		);
		expect(messages("open")).toHaveLength(0);
		await receive({ type: "ready" }, { source: window });
		await receive({ type: "ready" }, { origin: "https://forged.example.test" });
		await receive({ type: "ready", session: "previous-session" });
		expect(messages("open")).toHaveLength(0);
		await receive({ type: "ready" });
		expect(messages("open")).toHaveLength(1);
	});

	it("starts read-only and enables editing only after the user chooses Edit", async () => {
		await render();
		expect(button("Edit").disabled).toBe(true);
		await receive({ type: "ready" });
		await receive({ type: "opened" });
		expect(messages("set-read-only").at(-1)).toMatchObject({ readOnly: true });
		expect(button("Edit").disabled).toBe(false);
		await click("Edit");
		expect(messages("set-read-only").at(-1)).toMatchObject({ readOnly: false });
		expect(
			container.querySelector('[aria-label="About Flows Sheets"]'),
		).not.toBeNull();
	});

	it("never exposes editing when storage write access is unavailable", async () => {
		await render(false);
		await receive({ type: "ready" });
		await receive({ type: "opened" });
		expect(container.textContent).not.toContain("Edit file");
		expect(container.textContent).not.toContain("Save");
		expect(messages("set-read-only").at(-1)).toMatchObject({ readOnly: true });
	});

	it("opens licenses in the app for read-only users without replacing the document", async () => {
		await render(false);
		const documentFrame = container.querySelector("iframe");
		await act(async () => {
			container
				.querySelector<HTMLButtonElement>('[aria-label="About Flows Sheets"]')
				?.click();
		});
		const dialog = document.querySelector('[role="dialog"]');
		expect(dialog).not.toBeNull();
		expect(dialog?.querySelector("iframe")?.getAttribute("src")).toBe(
			"/craft/gridcraft/licenses/index.html",
		);
		expect(dialog?.querySelector("iframe")?.title).toBe(
			"Flows Sheets third-party licenses and source",
		);
		await act(async () => {
			dialog
				?.querySelector<HTMLButtonElement>('[data-slot="dialog-close"]')
				?.click();
		});
		expect(document.querySelector('[role="dialog"]')).toBeNull();
		expect(container.querySelector("iframe")).toBe(documentFrame);
		expect(saveDocument).not.toHaveBeenCalled();
	});

	it("uploads binary spreadsheet bytes and acknowledges only after storage succeeds", async () => {
		const upload = deferred<void>();
		saveDocument.mockReturnValue(upload.promise);
		await openForEditing();
		const downloadBeforeSave = container.querySelector("a")?.href;
		await exportDocument(7);
		expect(saveDocument).toHaveBeenCalledOnce();
		const file = saveDocument.mock.calls[0][0] as File;
		expect(file.name).toBe("report.xlsx");
		expect(file.type).toBe(XLSX_MIME);
		expect(new Uint8Array(await file.arrayBuffer())).toEqual(editedBytes);
		expect(messages("saved")).toHaveLength(0);
		expect(button("Save").disabled).toBe(true);
		expect(busyChange).toHaveBeenLastCalledWith(true);
		await act(async () => upload.resolve());
		expect(container.querySelector("a")?.href).not.toBe(downloadBeforeSave);
		expect(messages("saved")).toEqual([
			expect.objectContaining({ type: "saved", revision: 7 }),
		]);
		expect(busyChange).toHaveBeenLastCalledWith(false);
		expect(dirtyChange).toHaveBeenLastCalledWith(true);
		await receive({ type: "dirty", dirty: false });
		expect(container.textContent).toContain("Saved");
	});

	it("retains dirty edits after a failed upload and permits another save", async () => {
		saveDocument.mockRejectedValueOnce(new Error("Storage is unavailable"));
		await openForEditing();
		const failedRequest = await exportDocument();
		expect(container.querySelector('[role="alert"]')?.textContent).toBe(
			"Storage is unavailable",
		);
		expect(container.textContent).toContain("Unsaved changes");
		expect(messages("saved")).toHaveLength(0);
		expect(dirtyChange).toHaveBeenLastCalledWith(true);
		expect(button("Save").disabled).toBe(false);
		const nextRequest = await exportDocument();
		expect(nextRequest).not.toBe(failedRequest);
		expect(saveDocument).toHaveBeenCalledTimes(2);
		expect(messages("saved")).toHaveLength(1);
		expect(container.querySelector('[role="alert"]')).toBeNull();
	});

	it("ignores stale exports and duplicate responses during and after upload", async () => {
		const upload = deferred<void>();
		saveDocument.mockReturnValue(upload.promise);
		await openForEditing();
		await click("Save");
		const requestId = messages("export").at(-1)?.requestId;
		await receive({
			type: "exported",
			requestId: "stale-save",
			bytes: editedBytes.slice().buffer,
			revision: 4,
		});
		expect(saveDocument).not.toHaveBeenCalled();
		const exported = {
			type: "exported",
			requestId,
			bytes: editedBytes.slice().buffer,
			revision: 4,
		};
		await receive(exported);
		await receive(exported);
		expect(saveDocument).toHaveBeenCalledOnce();
		await act(async () => upload.resolve());
		await receive(exported);
		expect(saveDocument).toHaveBeenCalledOnce();
		expect(messages("saved")).toHaveLength(1);
	});

	it("ignores forged dirty and export messages after opening", async () => {
		await openForEditing();
		await receive({ type: "dirty", dirty: false }, { source: window });
		expect(dirtyChange).toHaveBeenLastCalledWith(true);
		await click("Save");
		const requestId = messages("export").at(-1)?.requestId;
		await receive(
			{
				type: "exported",
				requestId,
				bytes: editedBytes.slice().buffer,
				revision: 1,
			},
			{ origin: "https://forged.example.test" },
		);
		expect(saveDocument).not.toHaveBeenCalled();
	});

	it("refuses an export when write permission is revoked while preparing it", async () => {
		await openForEditing();
		await click("Save");
		const requestId = messages("export").at(-1)?.requestId;
		await render(false);
		expect(messages("set-read-only").at(-1)).toMatchObject({ readOnly: true });
		await receive({
			type: "exported",
			requestId,
			bytes: editedBytes.slice().buffer,
			revision: 1,
		});
		expect(saveDocument).not.toHaveBeenCalled();
		expect(container.querySelector('[role="alert"]')?.textContent).toContain(
			"permission",
		);
		expect(busyChange).toHaveBeenLastCalledWith(false);
	});

	it("aborts download and resets host callbacks when unmounted", async () => {
		const download = deferred<Response>();
		fetchDocument.mockReturnValue(download.promise);
		await render();
		const signal = fetchDocument.mock.calls[0][1].signal as AbortSignal;
		expect(signal.aborted).toBe(false);
		await act(async () => root?.unmount());
		root = null;
		expect(signal.aborted).toBe(true);
		expect(dirtyChange).toHaveBeenLastCalledWith(false);
		expect(busyChange).toHaveBeenLastCalledWith(false);
		await act(async () => download.resolve(new Response(originalBytes)));
		expect(send).not.toHaveBeenCalled();
	});

	it("does not acknowledge a late upload after the editor has unmounted", async () => {
		const upload = deferred<void>();
		saveDocument.mockReturnValue(upload.promise);
		await openForEditing();
		await exportDocument();
		await act(async () => root?.unmount());
		root = null;
		await act(async () => upload.resolve());
		expect(messages("saved")).toHaveLength(0);
		expect(dirtyChange).toHaveBeenLastCalledWith(false);
		expect(busyChange).toHaveBeenLastCalledWith(false);
	});

	it("retries a failed download with a new iframe session", async () => {
		const download = deferred<Response>();
		fetchDocument.mockReturnValueOnce(download.promise);
		await render();
		const previousSession = session();
		await act(async () =>
			download.resolve(new Response(null, { status: 503 })),
		);
		expect(container.querySelector('[role="alert"]')?.textContent).toContain(
			"could not be downloaded",
		);
		expect(container.querySelector("iframe")).toBeNull();
		fetchDocument.mockResolvedValueOnce(new Response(originalBytes));
		await click("Try again");
		expect(session()).not.toBe(previousSession);
		await receive({ type: "ready", session: previousSession });
		expect(messages("open")).toHaveLength(0);
		await receive({ type: "ready" });
		expect(fetchDocument).toHaveBeenCalledTimes(2);
		expect(messages("open")).toHaveLength(1);
		expect(container.querySelector('[role="alert"]')).toBeNull();
	});
});

describe("native previews and conversion", () => {
	async function renderFile(
		name: string,
		initialEditing = false,
		editable = true,
	) {
		await act(async () =>
			root?.render(
				<CraftDocument
					viewer={requireViewer(name)}
					url={`/files/${name}`}
					filename={name}
					editable={editable}
					initialEditing={initialEditing}
					onSave={saveDocument}
					onDirtyChange={dirtyChange}
					onBusyChange={busyChange}
				/>,
			),
		);
	}
	it("reads PDF in the browser and mounts PDFCraft only after Edit", async () => {
		await renderFile("report.pdf");
		expect(container.querySelector("iframe")?.title).toBe(
			"PDF Preview: report.pdf",
		);
		expect(container.querySelector("iframe")?.src).toContain(
			"#toolbar=1&view=FitH",
		);
		expect(container.querySelector('iframe[src*="/craft/"]')).toBeNull();
		expect(messages("open")).toHaveLength(0);
		await click("Edit");
		expect(container.querySelector("iframe")?.src).toContain(
			"/craft/pdfcraft/index.html",
		);
		await receive({ type: "ready" });
		expect(messages("open").at(-1)).toMatchObject({
			name: "report.pdf",
			readOnly: false,
		});
	});
	it.each(["report.pdf", "drawing.svg", "sample.mp3", "sample.mp4"])(
		"opens %s editing directly from the inline preview action",
		async (name) => {
			await renderFile(name, true);
			expect(container.querySelector("iframe")?.src).toContain(
				`/craft/${requireViewer(name).id}/index.html`,
			);
			expect(container.querySelector('[aria-label="Edit file"]')).toBeNull();
			await receive({ type: "ready" });
			expect(messages("open").at(-1)).toMatchObject({
				name,
			});
			await receive({ type: "opened" });
			expect(messages("set-read-only").at(-1)).toMatchObject({
				readOnly: false,
			});
			await exportDocument();
			expect(saveDocument).toHaveBeenCalledOnce();
		},
	);
	it("keeps PDFs in read mode when initial editing lacks write permission", async () => {
		await renderFile("report.pdf", true, false);
		expect(container.querySelector("iframe")?.title).toBe(
			"PDF Preview: report.pdf",
		);
		expect(container.querySelector('iframe[src*="/craft/"]')).toBeNull();
		expect(container.querySelector('[aria-label="Save"]')).toBeNull();
	});
	it("uses a converted extension and keeps the conversion target stable across saves", async () => {
		await renderFile("legacy.xls");
		await receive({ type: "ready" });
		await receive({ type: "opened", warnings: ["A chart is unsupported."] });
		expect(container.textContent).toContain("A chart is unsupported.");
		await click("Edit");
		expect(button("Save as .xlsx")).toBeTruthy();
		await exportDocument();
		expect(messages("export").at(-1)?.format).toBe("xlsx");
		const first = saveDocument.mock.calls[0][0];
		expect(first.name).toMatch(/^legacy-converted-.+\.xlsx$/);
		expect(first.type).toBe(requireViewer("legacy.xlsx").mime);
		await exportDocument();
		expect(saveDocument.mock.calls[1][0].name).toBe(first.name);
	});
	it("shows embedded-preview import limits", async () => {
		await renderFile("photo.kra");
		await receive({ type: "ready" });
		await receive({ type: "opened", previewOnly: true });
		expect(container.textContent).toContain(
			"Original layers and raw image data are unavailable",
		);
	});
});

function requireViewer(name: string) {
	const viewer = craftViewer(name);
	if (!viewer) throw new Error(`Missing viewer for ${name}`);
	return viewer;
}

it("waits for a development build without loading a missing iframe or timing out", async () => {
	vi.stubEnv("NODE_ENV", "development");
	vi.useFakeTimers();
	let built = false;
	fetchDocument.mockImplementation(async (url: string) =>
		url.endsWith("runtime.json")
			? new Response("{}", { status: built ? 200 : 404 })
			: new Response(originalBytes),
	);
	await render();
	expect(container.querySelector("iframe")).toBeNull();
	expect(container.textContent).toContain(
		"Waiting for Flows Sheets to finish building",
	);
	await act(async () => {
		await vi.advanceTimersByTimeAsync(95_000);
	});
	expect(container.querySelector('[role="alert"]')).toBeNull();
	built = true;
	await act(async () => {
		await vi.advanceTimersByTimeAsync(2000);
	});
	expect(container.querySelector("iframe")?.src).toContain(
		"/craft/gridcraft/index.html",
	);
	await receive({ type: "ready" });
	expect(messages("open")).toHaveLength(1);
});
