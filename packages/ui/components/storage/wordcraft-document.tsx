"use client";

import {
	DownloadIcon,
	InfoIcon,
	Loader2Icon,
	PencilIcon,
	SaveIcon,
} from "lucide-react";
import {
	type ReactNode,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { Button } from "../ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import {
	DOCX_MIME,
	MAX_DOCX_BYTES,
	WORDCRAFT_CHANNEL,
	readWordcraftMessage,
} from "./wordcraft-protocol";

interface WordcraftDocumentProps {
	url: string;
	filename: string;
	title?: ReactNode;
	editable: boolean;
	onSave: (file: File) => Promise<void>;
	onDirtyChange?: (dirty: boolean) => void;
	onBusyChange?: (busy: boolean) => void;
}

export function WordcraftDocument(props: WordcraftDocumentProps) {
	const [attempt, setAttempt] = useState(0);
	return (
		<WordcraftSession
			key={`${props.url}\n${props.filename}\n${attempt}`}
			{...props}
			onRetry={() => setAttempt((value) => value + 1)}
		/>
	);
}

function WordcraftSession({
	url,
	filename,
	title,
	editable,
	onSave,
	onDirtyChange,
	onBusyChange,
	onRetry,
}: WordcraftDocumentProps & { onRetry: () => void }) {
	const frame = useRef<HTMLIFrameElement>(null);
	const [session] = useState(() => crypto.randomUUID());
	const [status, setStatus] = useState<"loading" | "ready" | "failed">(
		"loading",
	);
	const [editing, setEditing] = useState(false);
	const [dirty, setDirty] = useState(false);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState("");
	const [saved, setSaved] = useState(false);
	const [download, setDownload] = useState<Blob | null>(null);
	const [downloadUrl, setDownloadUrl] = useState(url);
	const active = useRef(true);
	const saveAction = useRef<() => void>(() => {});
	const pending = useRef<{
		id: string;
		timer: ReturnType<typeof setTimeout>;
		persisting: boolean;
	} | null>(null);
	const latest = useRef({ editable, onSave, onDirtyChange, onBusyChange });
	latest.current = { editable, onSave, onDirtyChange, onBusyChange };

	useEffect(() => {
		if (!download) return;
		const objectUrl = URL.createObjectURL(download);
		setDownloadUrl(objectUrl);
		return () => URL.revokeObjectURL(objectUrl);
	}, [download]);

	const post = useCallback(
		(data: Record<string, unknown>, transfer: Transferable[] = []) => {
			frame.current?.contentWindow?.postMessage(
				{ channel: WORDCRAFT_CHANNEL, session, ...data },
				window.location.origin,
				transfer,
			);
		},
		[session],
	);

	const finishSave = useCallback(() => {
		if (pending.current) clearTimeout(pending.current.timer);
		pending.current = null;
		setSaving(false);
		latest.current.onBusyChange?.(false);
	}, []);

	useEffect(() => {
		active.current = true;
		const controller = new AbortController();
		let ready = false;
		let opened = false;
		let bytes: ArrayBuffer | undefined;
		const timeout = setTimeout(() => {
			if (!opened) {
				controller.abort();
				setStatus("failed");
				setError(
					"The document viewer did not start. Try opening the file again.",
				);
			}
		}, 90_000);
		const open = () => {
			if (!ready || !bytes || controller.signal.aborted) return;
			post({ type: "open", name: filename, bytes, readOnly: true }, [bytes]);
			bytes = undefined;
		};
		const receive = (event: MessageEvent) => {
			const message = readWordcraftMessage(
				event,
				frame.current?.contentWindow ?? null,
				window.location.origin,
				session,
			);
			if (!message || controller.signal.aborted) return;
			switch (message.type) {
				case "ready":
					ready = true;
					open();
					break;
				case "opened":
					opened = true;
					clearTimeout(timeout);
					setStatus("ready");
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
					setError(message.error || "The document could not be processed.");
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
					if (!latest.current.editable) {
						setError(
							"You do not have permission to save files in this storage.",
						);
						finishSave();
						break;
					}
					request.persisting = true;
					const file = new File([message.bytes], filename, { type: DOCX_MIME });
					void (async () => {
						try {
							await latest.current.onSave(file);
							if (!active.current) return;
							setDownload(file);
							post({ type: "saved", revision: message.revision });
							setSaved(true);
						} catch (cause) {
							if (!active.current) return;
							setError(
								cause instanceof Error
									? cause.message
									: "The document could not be saved. Your changes are still open.",
							);
						} finally {
							if (active.current) finishSave();
						}
					})();
					break;
				}
			}
		};
		window.addEventListener("message", receive);
		void (async () => {
			try {
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
				setError(
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
			window.removeEventListener("message", receive);
			latest.current.onDirtyChange?.(false);
			latest.current.onBusyChange?.(false);
		};
	}, [url, filename, session, post, finishSave]);

	useEffect(() => {
		if (!editable) setEditing(false);
		if (status === "ready")
			post({ type: "set-read-only", readOnly: !editable || !editing });
	}, [editable, editing, status, post]);

	function saveDocument() {
		if (!editable || !editing || pending.current || status !== "ready") return;
		const id = crypto.randomUUID();
		setError("");
		setSaved(false);
		setSaving(true);
		latest.current.onBusyChange?.(true);
		pending.current = {
			id,
			persisting: false,
			timer: setTimeout(() => {
				setError(
					"Preparing the document took too long. Your changes are still open.",
				);
				finishSave();
			}, 30_000),
		};
		post({ type: "export", requestId: id });
	}
	saveAction.current = saveDocument;

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
						? "Saving..."
						: dirty
							? "Unsaved changes"
							: saved
								? "Saved"
								: ""}
				</span>
				{editing && (
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="ghost"
								size="icon"
								className="size-8 shrink-0 text-muted-foreground"
								aria-label="About Flows Word"
							>
								<InfoIcon />
							</Button>
						</TooltipTrigger>
						<TooltipContent>
							Flows Word editing is experimental. Complex formatting may change
							when saved.
						</TooltipContent>
					</Tooltip>
				)}
				<Button asChild variant="ghost" size="sm" className="shrink-0">
					<a
						href={downloadUrl}
						download={filename}
						target="_blank"
						rel="noopener noreferrer"
						aria-label={`Download ${filename}`}
					>
						<DownloadIcon /> <span className="hidden sm:inline">Download</span>
					</a>
				</Button>
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
			{error && (
				<p role="alert" className="border-b px-3 py-2 text-sm text-destructive">
					{error}
				</p>
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
