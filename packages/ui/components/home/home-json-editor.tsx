"use client";

import { AlertCircle, Check, Code2, Copy, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "../ui/alert-dialog";
import { Button } from "../ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogTitle,
} from "../ui/dialog";
import { MonacoCodeEditor } from "../ui/monaco-code-editor";
import {
	formatHomeLayoutJson,
	parseHomeLayoutJson,
	trySerializeHomeLayout,
} from "./home-layout-json";
import type { IHomeLayout } from "./types";

interface HomeJsonEditorProps {
	layout: IHomeLayout;
	onApply: (layout: IHomeLayout) => void;
	onClose: () => void;
}

export function HomeJsonEditor({
	layout,
	onApply,
	onClose,
}: HomeJsonEditorProps) {
	const [initial] = useState(() => trySerializeHomeLayout(layout));
	const [json, setJson] = useState(initial.ok ? initial.json : "");
	const [appliedJson, setAppliedJson] = useState(json);
	const [error, setError] = useState<string | null>(
		initial.ok ? null : initial.error,
	);
	const [copied, setCopied] = useState(false);
	const [applied, setApplied] = useState(false);
	const [confirmDiscard, setConfirmDiscard] = useState(false);
	const closeButtonRef = useRef<HTMLButtonElement>(null);
	const dirty = json !== appliedJson;

	useEffect(() => {
		if (!dirty) return;
		const protect = (event: BeforeUnloadEvent) => {
			event.preventDefault();
			event.returnValue = "";
		};
		window.addEventListener("beforeunload", protect);
		return () => window.removeEventListener("beforeunload", protect);
	}, [dirty]);

	const requestClose = useCallback(() => {
		if (dirty) setConfirmDiscard(true);
		else onClose();
	}, [dirty, onClose]);

	const copy = useCallback(async () => {
		try {
			await navigator.clipboard.writeText(json);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch {
			setError(
				"Could not copy the layout. Select the JSON and copy it manually.",
			);
		}
	}, [json]);

	const format = useCallback(() => {
		const result = formatHomeLayoutJson(json);
		if (!result.ok) {
			setError(result.error);
			return;
		}
		setJson(result.json);
		setError(null);
		setApplied(false);
	}, [json]);

	const apply = useCallback(() => {
		const result = parseHomeLayoutJson(json);
		if (!result.ok) {
			setError(result.error);
			return;
		}
		const formatted = trySerializeHomeLayout(result.layout);
		if (!formatted.ok) {
			setError(formatted.error);
			return;
		}
		onApply(result.layout);
		setJson(formatted.json);
		setAppliedJson(formatted.json);
		setError(null);
		setApplied(true);
		setTimeout(() => setApplied(false), 2000);
	}, [json, onApply]);

	return (
		<>
			<Dialog
				open
				onOpenChange={(open) => {
					if (!open) requestClose();
				}}
			>
				<DialogContent
					showCloseButton={false}
					className="min-w-0 max-w-none gap-0 overflow-hidden p-0 sm:max-w-none"
					style={{
						width: "calc(100vw - 2rem)",
						maxWidth: "calc(100vw - 2rem)",
						minWidth: 0,
						height:
							"calc(100dvh - 2rem - var(--fl-safe-top, 0px) - var(--fl-safe-bottom, 0px))",
						maxHeight:
							"calc(100dvh - 2rem - var(--fl-safe-top, 0px) - var(--fl-safe-bottom, 0px))",
					}}
				>
					<div className="flex shrink-0 flex-col gap-3 border-b px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
						<div className="flex min-w-0 items-center gap-2">
							<Code2 className="h-5 w-5 shrink-0" />
							<div className="min-w-0">
								<DialogTitle className="truncate text-base">
									Home layout JSON
								</DialogTitle>
								<DialogDescription className="sr-only">
									Edit, copy, or paste the JSON for this home layout.
								</DialogDescription>
							</div>
						</div>
						<div className="flex flex-wrap items-center gap-2">
							<Button variant="outline" size="sm" onClick={format}>
								Format
							</Button>
							<Button variant="outline" size="sm" onClick={() => void copy()}>
								{copied ? (
									<>
										<Check className="h-4 w-4" />
										Copied
									</>
								) : (
									<>
										<Copy className="h-4 w-4" />
										Copy
									</>
								)}
							</Button>
							<Button size="sm" onClick={apply}>
								{applied ? (
									<>
										<Check className="h-4 w-4" />
										Applied
									</>
								) : (
									"Apply changes"
								)}
							</Button>
							<Button
								ref={closeButtonRef}
								variant="ghost"
								size="icon"
								className="h-8 w-8"
								onClick={requestClose}
								aria-label="Close JSON editor"
							>
								<X className="h-4 w-4" />
							</Button>
						</div>
					</div>

					{error && (
						<div
							role="alert"
							className="flex shrink-0 items-start gap-2 bg-destructive/10 px-4 py-2 text-destructive"
						>
							<AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
							<span className="text-sm">{error}</span>
						</div>
					)}

					<div className="min-h-0 min-w-0 flex-1 overflow-hidden">
						<MonacoCodeEditor
							value={json}
							onChange={(value) => {
								setJson(value);
								setError(null);
								setApplied(false);
							}}
							language="json"
							height="100%"
							showMinimap
							className="h-full rounded-none border-0"
						/>
					</div>

					<div className="shrink-0 border-t px-4 py-2 text-xs text-muted-foreground">
						Apply updates the current draft. Use Save or Publish in the home
						editor to keep it. Referenced apps and data must also be available
						in the target profile.
					</div>
				</DialogContent>
			</Dialog>
			<AlertDialog open={confirmDiscard} onOpenChange={setConfirmDiscard}>
				<AlertDialogContent
					onCloseAutoFocus={(event) => {
						if (closeButtonRef.current) {
							event.preventDefault();
							closeButtonRef.current.focus();
						}
					}}
				>
					<AlertDialogHeader>
						<AlertDialogTitle>Discard unapplied JSON changes?</AlertDialogTitle>
						<AlertDialogDescription>
							Your JSON edits have not been applied. The current Home draft will
							stay unchanged if you discard them.
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel>Keep editing</AlertDialogCancel>
						<AlertDialogAction onClick={onClose}>
							Discard JSON edits
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</>
	);
}
