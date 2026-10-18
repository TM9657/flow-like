"use client";

import { DownloadIcon, Loader2Icon, PencilIcon, SaveIcon } from "lucide-react";
import {
	type ReactNode,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import type { BulkUploadProgressCallback } from "../../lib/bulk-upload";
import { Button } from "../ui/button";
import { downloadCraftFile } from "./craft-download";
import type { CraftToolTarget } from "./craft-mcp-session";
import { CraftMcpTargetButton } from "./craft-mcp-target-button";
import {
	FileSaveProgress,
	type FileSaveStatus,
	fileSaveError,
	storageSaveProgress,
} from "./file-save-progress";
import { useCraftMcpSession } from "./use-craft-mcp-session";
import { ViewerAbout } from "./viewer-about";
import {
	DOCX_MIME,
	MAX_DOCX_BYTES,
	WORDCRAFT_CHANNEL,
	readWordcraftMessage,
} from "./wordcraft-protocol";

interface WordcraftDocumentProps {
	toolTarget?: CraftToolTarget;
	url?: string;
	create?: boolean;
	filename: string;
	title?: ReactNode;
	editable: boolean;
	initialEditing?: boolean;
	showDownload?: boolean;
	onSave?: (
		file: File,
		onProgress?: BulkUploadProgressCallback,
	) => Promise<void>;
	onDirtyChange?: (dirty: boolean) => void;
	onBusyChange?: (busy: boolean) => void;
	onError?: (message: string) => void;
	onReady?: () => void;
}

export function WordcraftDocument(props: WordcraftDocumentProps) {
	const [attempt, setAttempt] = useState(0);
	return (
		<WordcraftSession
			key={`${props.url}\n${props.create}\n${props.filename}\n${attempt}`}
			{...props}
			onRetry={() => setAttempt((value) => value + 1)}
		/>
	);
}

function WordcraftSession({
	toolTarget,
	url,
	create = false,
	filename,
	title,
	editable: requestedEditable,
	initialEditing = false,
	showDownload = true,
	onSave,
	onDirtyChange,
	onBusyChange,
	onError,
	onReady,
	onRetry,
}: WordcraftDocumentProps & { onRetry: () => void }) {
	const editable = requestedEditable && !!onSave;
	const frame = useRef<HTMLIFrameElement>(null);
	const [session] = useState(() => crypto.randomUUID());
	const [status, setStatus] = useState<"loading" | "ready" | "failed">(
		"loading",
	);
	const [editing, setEditing] = useState(initialEditing && editable);
	const [dirty, setDirty] = useState(false);
	const [saving, setSaving] = useState(false);
	const [saveProgress, setSaveProgress] = useState<FileSaveStatus | null>(null);
	const [error, setError] = useState("");
	const reportedError = useRef<string | null>(null);
	const [saveError, setSaveError] = useState(false);
	const [saved, setSaved] = useState(false);
	const [download, setDownload] = useState<Blob | null>(null);
	const [downloadUrl, setDownloadUrl] = useState(url ?? "");
	const active = useRef(true);
	const saveAction = useRef<() => void>(() => {});
	const pending = useRef<{
		id: string;
		timer: ReturnType<typeof setTimeout>;
		persisting: boolean;
		purpose: "save" | "download";
	} | null>(null);
	const latest = useRef({
		editable,
		editing,
		onSave,
		onDirtyChange,
		onBusyChange,
		onError,
		onReady,
	});
	latest.current = {
		editable,
		editing,
		onSave,
		onDirtyChange,
		onBusyChange,
		onError,
		onReady,
	};
	const reportError = useCallback((message: string) => {
		setError(message);
		if (reportedError.current === message) return;
		reportedError.current = message;
		latest.current.onError?.(message);
	}, []);

	useEffect(() => {
		if (!download) return;
		const objectUrl = URL.createObjectURL(download);
		setDownloadUrl(objectUrl);
		return () => URL.revokeObjectURL(objectUrl);
	}, [download]);

	const post = useCallback(
		(data: Record<string, unknown>, transfer: Transferable[] = []) => {
			const runtimeFrame = frame.current;
			if (!runtimeFrame?.contentWindow) return;
			runtimeFrame.contentWindow.postMessage(
				{ channel: WORDCRAFT_CHANNEL, session, ...data },
				new URL(runtimeFrame.src, runtimeFrame.ownerDocument.baseURI).origin,
				transfer,
			);
		},
		[session],
	);

	const finishSave = useCallback(() => {
		if (pending.current) clearTimeout(pending.current.timer);
		pending.current = null;
		setSaving(false);
		setSaveProgress(null);
		latest.current.onBusyChange?.(false);
	}, []);
	useCraftMcpSession({
		target: toolTarget,
		sessionId: session,
		editor: "wordcraft",
		channel: WORDCRAFT_CHANNEL,
		frame: () => frame.current,
		post,
		ready: status === "ready",
		editable,
		editing,
		busy: saving,
	});

	useEffect(() => {
		active.current = true;
		const runtimeFrame = frame.current;
		const ownerWindow = runtimeFrame?.ownerDocument.defaultView ?? window;
		const origin = runtimeFrame
			? new URL(runtimeFrame.src, runtimeFrame.ownerDocument.baseURI).origin
			: window.location.origin;
		const controller = new AbortController();
		let ready = false;
		let opened = false;
		let sent = false;
		let bytes: ArrayBuffer | undefined;
		const timeout = setTimeout(() => {
			if (!opened) {
				controller.abort();
				setStatus("failed");
				reportError(
					"The document viewer did not start. Try opening the file again.",
				);
			}
		}, 90_000);
		const open = () => {
			if (!ready || sent || (!create && !bytes) || controller.signal.aborted)
				return;
			sent = true;
			if (create) {
				post({
					type: "create",
					name: filename,
					readOnly: !latest.current.editable || !latest.current.editing,
				});
				return;
			}
			if (!bytes) return;
			post(
				{
					type: "open",
					name: filename,
					bytes,
					readOnly: !latest.current.editable || !latest.current.editing,
				},
				[bytes],
			);
			bytes = undefined;
		};
		const receive = (event: MessageEvent) => {
			const message = readWordcraftMessage(
				event,
				frame.current?.contentWindow ?? null,
				origin,
				session,
			);
			if (!message || controller.signal.aborted) return;
			switch (message.type) {
				case "ready":
					ready = true;
					open();
					break;
				case "opened":
					if (opened) break;
					opened = true;
					clearTimeout(timeout);
					setStatus("ready");
					latest.current.onReady?.();
					break;
				case "dirty":
					if (!opened) break;
					setDirty(message.dirty);
					if (message.dirty) setSaved(false);
					latest.current.onDirtyChange?.(message.dirty);
					break;
				case "save-request":
					if (opened) saveAction.current();
					break;
				case "error":
					if (message.requestId && message.requestId !== pending.current?.id)
						break;
					setSaveError(!!pending.current && !pending.current.persisting);
					reportError(message.error || "The document could not be processed.");
					if (!opened) {
						clearTimeout(timeout);
						setStatus("failed");
					}
					if (!pending.current?.persisting) finishSave();
					break;
				case "exported": {
					const request = pending.current;
					if (
						!request ||
						request.id !== message.requestId ||
						request.persisting
					)
						break;
					clearTimeout(request.timer);
					if (
						request.purpose === "save" &&
						(!latest.current.editable || !latest.current.onSave)
					) {
						setSaveError(true);
						reportError("You do not have permission to save this file.");
						finishSave();
						break;
					}
					request.persisting = true;
					setSaveProgress({ stage: "Saving file", progress: null });
					const file = new File([message.bytes], filename, { type: DOCX_MIME });
					if (request.purpose === "download") {
						try {
							downloadCraftFile(file);
						} catch (cause) {
							reportError(fileSaveError(cause));
						} finally {
							finishSave();
						}
						break;
					}
					const onSave = latest.current.onSave;
					if (!onSave) {
						finishSave();
						break;
					}
					void (async () => {
						try {
							await onSave(file, (percent, detail) => {
								if (!active.current || pending.current !== request) return;
								setSaveProgress(storageSaveProgress(percent, detail));
							});
							if (!active.current || pending.current !== request) return;
							setDownload(file);
							post({ type: "saved", revision: message.revision });
							setSaved(true);
						} catch (cause) {
							if (!active.current || pending.current !== request) return;
							setSaveError(true);
							reportError(fileSaveError(cause));
						} finally {
							if (active.current && pending.current === request) finishSave();
						}
					})();
					break;
				}
			}
		};
		ownerWindow.addEventListener("message", receive);
		void (async () => {
			try {
				if (create) {
					open();
					return;
				}
				if (!url)
					throw new Error(
						"Choose a document to open or create a new document.",
					);
				const response = await fetch(url, {
					signal: controller.signal,
					cache: "no-store",
				});
				if (!response.ok)
					throw new Error("The document could not be downloaded.");
				if (Number(response.headers.get("content-length")) > MAX_DOCX_BYTES)
					throw new Error("DOCX previews support files up to 50 MB.");
				const blob = await response.blob();
				if (blob.size > MAX_DOCX_BYTES)
					throw new Error("DOCX previews support files up to 50 MB.");
				if (blob.size === 0) throw new Error("This document is empty.");
				if (controller.signal.aborted) return;
				setDownload(blob);
				bytes = await blob.arrayBuffer();
				open();
			} catch (cause) {
				if (controller.signal.aborted) return;
				clearTimeout(timeout);
				setStatus("failed");
				reportError(
					cause instanceof Error ? cause.message : "Could not load document.",
				);
			}
		})();
		return () => {
			active.current = false;
			controller.abort();
			clearTimeout(timeout);
			if (pending.current) clearTimeout(pending.current.timer);
			pending.current = null;
			ownerWindow.removeEventListener("message", receive);
			latest.current.onDirtyChange?.(false);
			latest.current.onBusyChange?.(false);
		};
	}, [url, create, filename, session, post, finishSave, reportError]);

	useEffect(() => {
		if (!editable) setEditing(false);
		if (status === "ready")
			post({ type: "set-read-only", readOnly: !editable || !editing });
	}, [editable, editing, status, post]);

	function exportDocument(purpose: "save" | "download") {
		if (pending.current || status !== "ready") return;
		if (purpose === "save" && (!editable || !editing || !onSave)) return;
		const id = crypto.randomUUID();
		setError("");
		reportedError.current = null;
		setSaveError(false);
		if (purpose === "save") setSaved(false);
		setSaving(true);
		setSaveProgress({ stage: "Preparing file", progress: null });
		latest.current.onBusyChange?.(true);
		pending.current = {
			id,
			persisting: false,
			purpose,
			timer: setTimeout(() => {
				setSaveError(true);
				reportError("Preparing the document took too long.");
				finishSave();
			}, 30_000),
		};
		post({
			type: "export",
			requestId: id,
			...(purpose === "download" ? { purpose } : {}),
		});
	}
	function saveDocument() {
		exportDocument("save");
	}
	saveAction.current = onSave ? saveDocument : () => exportDocument("download");

	return (
		<div className="flex h-full min-h-0 flex-col bg-background">
			<div className="flex min-w-0 shrink-0 items-center gap-2 border-b px-3 py-3 pr-12">
				<div className="min-w-0 flex-1 truncate text-sm font-semibold">
					{title ?? filename}
				</div>
				<span
					className="hidden shrink-0 text-xs text-muted-foreground sm:inline"
					aria-live="polite"
				>
					{saving
						? pending.current?.purpose === "download"
							? "Preparing download..."
							: "Saving..."
						: dirty
							? "Unsaved changes"
							: saved
								? "Saved"
								: ""}
				</span>
				<ViewerAbout
					label="Flows Word"
					licensesUrl="/wordcraft/licenses/index.html"
				>
					{editing ? (
						<>
							Flows Word editing is experimental. Complex formatting may change
							when saved.
						</>
					) : undefined}
				</ViewerAbout>
				{toolTarget && status === "ready" && (
					<CraftMcpTargetButton target={toolTarget} onError={reportError} />
				)}
				{showDownload && (
					<Button asChild variant="ghost" size="sm" className="shrink-0">
						<a
							href={downloadUrl || undefined}
							aria-disabled={status !== "ready" || saving}
							onClick={(event) => {
								if (status !== "ready" || saving) {
									event.preventDefault();
									return;
								}
								if (editing || dirty || create) {
									event.preventDefault();
									exportDocument("download");
								}
							}}
							download={filename}
							target="_blank"
							rel="noopener noreferrer"
							aria-label={`Download ${filename}`}
						>
							<DownloadIcon />{" "}
							<span className="hidden sm:inline">Download</span>
						</a>
					</Button>
				)}
				{editable && !editing && (
					<Button
						variant="outline"
						size="sm"
						className="shrink-0"
						aria-label="Edit document"
						disabled={status !== "ready"}
						onClick={() => setEditing(true)}
					>
						<PencilIcon />{" "}
						<span className="hidden sm:inline">Edit document</span>
					</Button>
				)}
				{editable && editing && (
					<Button
						size="sm"
						className="shrink-0"
						aria-label="Save"
						disabled={status !== "ready" || saving}
						onClick={saveDocument}
					>
						{saving ? <Loader2Icon className="animate-spin" /> : <SaveIcon />}{" "}
						Save
					</Button>
				)}
			</div>
			{saving && saveProgress && <FileSaveProgress {...saveProgress} />}
			{error && (
				<div
					role="alert"
					className="shrink-0 border-b px-3 py-2 text-sm text-destructive"
				>
					{saveError && <p className="font-medium">Could not save file</p>}
					<p>{error}</p>
					{saveError && (
						<>
							<p>Your changes are still open. Try saving again.</p>
							{editable && editing && status === "ready" && !saving && (
								<Button
									variant="outline"
									size="sm"
									className="mt-2"
									onClick={saveDocument}
								>
									Try saving again
								</Button>
							)}
						</>
					)}
				</div>
			)}
			<div className="relative flex-1 min-h-0">
				{status !== "failed" && (
					<iframe
						ref={frame}
						src={`/wordcraft/index.html?session=${encodeURIComponent(session)}`}
						title={`Flows Word: ${filename}`}
						className="h-full w-full border-0"
						allow="clipboard-read; clipboard-write"
					/>
				)}
				{status === "loading" && (
					<output className="absolute inset-0 flex items-center justify-center gap-2 bg-background">
						<Loader2Icon className="size-4 animate-spin" /> Loading Flows
						Word...
					</output>
				)}
				{status === "failed" && (
					<div className="flex h-full items-center justify-center">
						<Button variant="outline" onClick={onRetry}>
							Try again
						</Button>
					</div>
				)}
			</div>
		</div>
	);
}
