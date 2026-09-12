"use client";

import { useTranslation } from "@flow-like/locales";
import { PaletteIcon, SaveIcon, TriangleAlertIcon } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { cn } from "../../../../lib/utils";
import { useBackend } from "../../../../state/backend-state";
import { Button } from "../../../ui/button";
import { MonacoCodeEditor } from "../../../ui/monaco-code-editor";
import { Skeleton } from "../../../ui/skeleton";

/**
 * The app-wide stylesheet.
 *
 * Save is explicit, never debounced: `App::save` re-serializes and re-writes every board in
 * the process registry before it writes the manifest, so an autosave on a typing pause
 * would rewrite board files on every keystroke lull.
 */
export function AppStylesDocument({
	appId,
	className,
}: Readonly<{
	appId: string;
	className?: string;
}>) {
	const { t } = useTranslation("flow");
	const backend = useBackend();
	const [value, setValue] = useState("");
	const [saved, setSaved] = useState("");
	const [loading, setLoading] = useState(true);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const dirty = value !== saved;
	const dirtyRef = useRef(dirty);
	dirtyRef.current = dirty;

	const load = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const css = (await backend.appState.getAppStylesheet(appId)) ?? "";
			setValue(css);
			setSaved(css);
		} catch (cause) {
			console.error("Failed to load the app stylesheet", appId, cause);
			setError(
				cause instanceof Error ? cause.message : String(cause ?? "unknown"),
			);
		} finally {
			setLoading(false);
		}
	}, [appId, backend.appState]);

	useEffect(() => {
		void load();
	}, [load]);

	const save = useCallback(async () => {
		if (!dirtyRef.current) return;
		setSaving(true);
		try {
			await backend.appState.setAppStylesheet(appId, value);
			setSaved(value);
			toast.success(t("appStylesheetSaved", "App stylesheet saved"));
		} catch (cause) {
			console.error("Failed to save the app stylesheet", appId, cause);
			toast.error(
				cause instanceof Error ? cause.message : String(cause ?? "unknown"),
			);
		} finally {
			setSaving(false);
		}
	}, [appId, backend.appState, t, value]);

	useEffect(() => {
		const onKeyDown = (event: KeyboardEvent) => {
			if (event.key !== "s" || !(event.metaKey || event.ctrlKey)) return;
			event.preventDefault();
			void save();
		};
		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	}, [save]);

	// A tab closing is not a React unmount we can block, so the browser prompt is the only
	// thing standing between an unsaved sheet and a lost afternoon.
	useEffect(() => {
		const onBeforeUnload = (event: BeforeUnloadEvent) => {
			if (!dirtyRef.current) return;
			event.preventDefault();
			event.returnValue = "";
		};
		window.addEventListener("beforeunload", onBeforeUnload);
		return () => window.removeEventListener("beforeunload", onBeforeUnload);
	}, []);

	return (
		<div
			className={cn("flex h-full min-h-0 flex-col bg-background", className)}
		>
			<div className="flex shrink-0 items-center gap-2 border-b px-3 py-1.5">
				<PaletteIcon className="size-3.5 shrink-0 text-muted-foreground" />
				<span className="truncate font-mono text-xs">app.css</span>
				{dirty && (
					<span
						className="size-1.5 shrink-0 rounded-full bg-primary"
						aria-label={t("unsavedChanges", "Unsaved changes")}
					/>
				)}
				<span className="flex-1" />
				<Button
					size="sm"
					variant="ghost"
					className="h-6 gap-1.5 text-xs"
					disabled={!dirty || saving}
					onClick={() => void save()}
				>
					<SaveIcon className="size-3.5" />
					{t("save", "Save")}
				</Button>
			</div>

			<p className="shrink-0 border-b px-3 py-1.5 text-[11px] text-muted-foreground">
				{t(
					"appStylesheetScopeHint",
					"Applies to every page in this app. A page's own CSS wins where both set the same thing. :root, body and html are rewritten to the app root; @import is removed; @keyframes and @font-face names stay global, so prefix them.",
				)}
			</p>

			<div className="min-h-0 flex-1 overflow-hidden">
				{loading && <Skeleton className="h-full w-full" />}
				{!loading && error && (
					<div className="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground text-sm">
						<TriangleAlertIcon className="size-5 text-destructive" />
						<p>
							{t(
								"couldNotOpenTheAppStylesheet",
								"Could not open the app stylesheet",
							)}
						</p>
						<p className="max-w-md text-center text-xs">{error}</p>
						<Button size="sm" variant="outline" onClick={() => void load()}>
							{t("tryAgain", "Try again")}
						</Button>
					</div>
				)}
				{!loading && !error && (
					<MonacoCodeEditor
						value={value}
						onChange={setValue}
						language="css"
						height="100%"
						allowFullscreen
						showMinimap
					/>
				)}
			</div>
		</div>
	);
}
