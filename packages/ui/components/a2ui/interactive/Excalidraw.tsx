"use client";

import type { ImportedDataState } from "@excalidraw/excalidraw/data/types";
import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";
import type {
	AppState,
	BinaryFiles,
	ExcalidrawInitialDataState,
} from "@excalidraw/excalidraw/types";
import "@excalidraw/excalidraw/index.css";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { cn } from "../../../lib/utils";
import { Button } from "../../ui/button";
import {
	useActionContext,
	useComponentEventTrigger,
	useOnAction,
	useSetElementValue,
} from "../ActionHandler";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import { resolveEventActions } from "../event-handlers";
import type {
	ExcalidrawEventDetails,
	ExcalidrawEventName,
	ExcalidrawScene,
} from "../excalidraw-events";
import {
	ExcalidrawSession,
	parseExcalidrawScene,
	positiveNumber,
} from "../excalidraw-session";
import {
	useBoundInputValue,
	valueRevisionOf,
} from "../hooks/use-bound-input-value";
import { useElementRef } from "../hooks/use-element-ref";
import type { ExcalidrawComponent } from "../types";

type ExcalidrawLibrary = typeof import("@excalidraw/excalidraw");
const EXACT = { legacyFallback: false, wildcardFallback: false };

function readScene(value: unknown): {
	scene?: ExcalidrawScene;
	error?: string;
} {
	try {
		return { scene: parseExcalidrawScene(value) };
	} catch (error) {
		return {
			error:
				error instanceof Error ? error.message : "Unable to load this drawing.",
		};
	}
}

export function A2UIExcalidraw(props: ComponentProps<ExcalidrawComponent>) {
	return (
		<ExcalidrawInput
			key={`${props.surfaceId}/${props.componentId}`}
			{...props}
		/>
	);
}

function ExcalidrawInput({
	component,
	componentId,
	surfaceId,
	style,
	elementRef,
}: ComponentProps<ExcalidrawComponent>) {
	const { resolve } = useData();
	const triggerEvent = useComponentEventTrigger(componentId);
	const onAction = useOnAction();
	const setElementValue = useSetElementValue();
	const { isPreviewMode } = useActionContext();
	const containerRef = useRef<HTMLDivElement>(null);
	const rootRef = useElementRef(elementRef, containerRef);
	const [library, setLibrary] = useState<ExcalidrawLibrary>();
	const [document, setDocument] = useState(() => ({
		...readScene(
			component.value === undefined ? undefined : resolve(component.value),
		),
		revision: 0,
	}));
	const [error, setError] = useState<string>();
	const [dirty, setDirty] = useState(false);
	const [busy, setBusy] = useState(false);
	const [ready, setReady] = useState(false);
	const active = useRef(false);
	const initialized = useRef(-1);
	const currentDocument = useRef(document);
	currentDocument.current = document;
	const latest = useRef({
		component,
		triggerEvent,
		onAction,
		setElementValue,
		isPreviewMode,
	});
	latest.current = {
		component,
		triggerEvent,
		onAction,
		setElementValue,
		isPreviewMode,
	};

	const notify = useCallback(
		<Name extends ExcalidrawEventName>(
			name: Name,
			details: ExcalidrawEventDetails[Name],
		) => {
			if (!active.current || !latest.current.isPreviewMode) return;
			const context = { ...details, actionId: name };
			latest.current.onAction?.({
				type: "userAction",
				name,
				surfaceId,
				sourceComponentId: componentId,
				timestamp: Date.now(),
				context,
			});
			void latest.current
				.triggerEvent(name, latest.current.component, context, EXACT)
				.catch((reason) =>
					console.error(`Excalidraw ${name} handler failed`, reason),
				);
		},
		[componentId, surfaceId],
	);

	const reportError = useCallback(
		(operation: "load" | "save" | "autoSave", reason: unknown) => {
			if (!active.current) return;
			const message = reason instanceof Error ? reason.message : String(reason);
			setError(message);
			notify("error", { message, operation });
		},
		[notify],
	);

	const [session] = useState(
		() =>
			new ExcalidrawSession({
				onBusy: (value) => {
					if (active.current) setBusy(value);
				},
				onDirty: (value) => {
					if (!active.current) return;
					setDirty(value);
					notify("dirtyChange", { dirty: value });
				},
				onSave: async (name, scene, changeCount) => {
					if (!active.current || !latest.current.isPreviewMode)
						throw new Error("The drawing is no longer active.");
					setError(undefined);
					const context = { actionId: name, value: scene, scene, changeCount };
					latest.current.onAction?.({
						type: "userAction",
						name,
						surfaceId,
						sourceComponentId: componentId,
						timestamp: Date.now(),
						context,
					});
					await latest.current.triggerEvent(
						name,
						latest.current.component,
						context,
						{ ...EXACT, requireSuccess: true },
					);
				},
				onError: reportError,
			}),
	);

	const replaceScene = useCallback(
		(value: unknown) => {
			session.reset();
			setReady(false);
			setError(undefined);
			setDocument((previous) => ({
				...readScene(value),
				revision: previous.revision + 1,
			}));
		},
		[session],
	);
	const [, setValue] = useBoundInputValue<unknown>(component.value, undefined, {
		revision: valueRevisionOf(component),
		onExternalValue: replaceScene,
	});
	const readOnly =
		Boolean(component.readOnly && resolve(component.readOnly)) ||
		!isPreviewMode;
	const autoSave =
		(component.autoSave === undefined ||
			Boolean(resolve(component.autoSave))) &&
		resolveEventActions(component.eventHandlers, "autoSave", undefined, EXACT)
			.actions.length > 0;
	const delay = positiveNumber(
		component.autoSaveDelayMs && resolve(component.autoSaveDelayMs),
		1500,
		100,
	);
	const minimum = positiveNumber(
		component.autoSaveMinChanges && resolve(component.autoSaveMinChanges),
		1,
	);
	const theme =
		component.theme && resolve(component.theme) === "dark" ? "dark" : "light";
	const width = component.width
		? (resolve(component.width) as string | number)
		: "100%";
	const height = component.height
		? (resolve(component.height) as string | number)
		: "600px";
	// Upstream restores initialData asynchronously without reporting restore errors.
	// Validate it here so a malformed imported element produces our typed error event.
	const prepared = useMemo(() => {
		if (!library || !document.scene) return {};
		try {
			return {
				initialData: {
					...library.restore(
						document.scene as unknown as ImportedDataState,
						null,
						null,
						{ repairBindings: true },
					),
					scrollToContent: true,
				},
			};
		} catch (reason) {
			return {
				error:
					reason instanceof Error
						? reason.message
						: "Unable to restore this drawing.",
			};
		}
	}, [library, document.scene]);

	useEffect(() => {
		active.current = true;
		return () => {
			active.current = false;
			initialized.current = -1;
			session.dispose();
		};
	}, [session]);

	useEffect(() => {
		let cancelled = false;
		(
			window as Window & { EXCALIDRAW_ASSET_PATH?: string }
		).EXCALIDRAW_ASSET_PATH = new URL(
			process.env.NEXT_PUBLIC_EXCALIDRAW_ASSET_PATH ?? "/excalidraw/0.18.1/",
			window.location.href,
		).href;
		import("@excalidraw/excalidraw")
			.then((module) => {
				if (!cancelled) setLibrary(module);
			})
			.catch((reason) => {
				if (!cancelled) reportError("load", reason);
			});
		return () => {
			cancelled = true;
		};
	}, [reportError]);

	useEffect(() => {
		const loadError = document.error ?? prepared.error;
		if (loadError) reportError("load", loadError);
	}, [document, prepared, reportError]);

	useEffect(() => {
		session.configure({
			readOnly,
			autoSave,
			autoSaveDelayMs: delay,
			autoSaveMinChanges: minimum,
		});
	}, [session, readOnly, autoSave, delay, minimum]);

	const onChange = useCallback(
		(
			elements: readonly ExcalidrawElement[],
			appState: AppState,
			files: BinaryFiles,
		) => {
			if (
				!library ||
				!active.current ||
				appState.isLoading ||
				currentDocument.current.revision !== document.revision
			)
				return;
			try {
				const scene = parseExcalidrawScene(
					library.serializeAsJSON(elements, appState, files, "local"),
				);
				if (initialized.current !== document.revision) {
					initialized.current = document.revision;
					session.reset(scene);
					setReady(true);
					if (latest.current.isPreviewMode)
						latest.current.setElementValue?.(
							`${surfaceId}/${componentId}`,
							scene,
						);
					notify("load", { value: scene, scene });
					return;
				}
				if (!session.change(scene)) return;
				setValue(scene);
				// Update element-value readers without executing a workflow for each edit.
				latest.current.onAction?.({
					type: "userAction",
					name: "change",
					surfaceId,
					sourceComponentId: componentId,
					timestamp: Date.now(),
					context: { value: scene },
				});
			} catch (reason) {
				reportError("load", reason);
			}
		},
		[
			library,
			document.revision,
			session,
			notify,
			setValue,
			surfaceId,
			componentId,
			reportError,
		],
	);

	const Editor = library?.Excalidraw;
	return (
		<div
			ref={rootRef}
			className={cn(
				"flex min-h-[200px] min-w-0 flex-col overflow-hidden rounded-md border",
				resolveStyle(style),
			)}
			style={{ width, height, ...resolveInlineStyle(style) }}
			onKeyDownCapture={(event) => {
				if (
					(event.ctrlKey || event.metaKey) &&
					event.key.toLowerCase() === "s"
				) {
					event.preventDefault();
					event.stopPropagation();
					if (ready && !readOnly) void session.save();
				}
			}}
		>
			{!readOnly && (
				<div className="flex items-center justify-between gap-2 border-b px-3 py-2">
					<output className="text-muted-foreground text-xs">
						{busy
							? "Saving…"
							: dirty
								? "Unsaved changes"
								: "No unsaved changes"}
					</output>
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={!ready || busy}
						onClick={() => {
							void session.save();
						}}
					>
						Save
					</Button>
				</div>
			)}
			{error && (
				<div
					role="alert"
					className="border-b px-3 py-2 text-destructive text-sm"
				>
					{error}
				</div>
			)}
			<div className="relative min-h-0 flex-1">
				{Editor && prepared.initialData ? (
					<Editor
						key={document.revision}
						initialData={prepared.initialData as ExcalidrawInitialDataState}
						onChange={onChange}
						viewModeEnabled={readOnly}
						theme={theme}
						UIOptions={{
							canvasActions: {
								saveToActiveFile: false,
								export: false,
								toggleTheme: false,
							},
						}}
					/>
				) : (
					!error && (
						<output className="block p-4 text-muted-foreground text-sm">
							Loading drawing…
						</output>
					)
				)}
			</div>
		</div>
	);
}
