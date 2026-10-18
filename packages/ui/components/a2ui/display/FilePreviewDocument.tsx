"use client";

import {
	Suspense,
	lazy,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { useBackend } from "../../../state/backend-state";
import type { CraftViewer } from "../../storage/craft-viewers";
import {
	useActionContext,
	useComponentEventTrigger,
	useOnAction,
	useSetElementValue,
} from "../ActionHandler";
import { useData } from "../DataContext";
import { resolveEventActions } from "../event-handlers";
import type {
	FilePreviewEventDetails,
	FilePreviewEventName,
	FilePreviewExportedFile,
} from "../file-preview-events";
import { registerA2UINavigationGuard } from "../navigation-guard";
import type { FileEditorComponent, FilePreviewComponent } from "../types";

const CraftDocument = lazy(() =>
	import("../../storage/craft-document").then((module) => ({
		default: module.CraftDocument,
	})),
);
const WordcraftDocument = lazy(() =>
	import("../../storage/wordcraft-document").then((module) => ({
		default: module.WordcraftDocument,
	})),
);
const EXACT = { legacyFallback: false, wildcardFallback: false } as const;

export function filePreviewSaveActions(
	component: FilePreviewComponent | FileEditorComponent,
) {
	const { actions } = resolveEventActions(
		component.eventHandlers,
		"save",
		undefined,
		EXACT,
	);
	return actions.length > 0 &&
		actions.every(
			(action) =>
				action.name === "workflow_event" || action.name === "widget_event",
		)
		? actions
		: [];
}

interface FilePreviewDocumentProps {
	component: FilePreviewComponent | FileEditorComponent;
	componentId: string;
	surfaceId: string;
	source: string;
	url?: string;
	create?: boolean;
	filename: string;
	viewer: CraftViewer;
	editable: boolean;
	initialEditing: boolean;
	showDownload: boolean;
	onDirtyChange?: (dirty: boolean) => void;
	onBusyChange?: (busy: boolean) => void;
}

export function FilePreviewDocument(props: FilePreviewDocumentProps) {
	const {
		componentId,
		surfaceId,
		source,
		filename,
		viewer,
		editable,
		initialEditing,
		showDownload,
	} = props;
	const backend = useBackend();
	const actionContext = useActionContext();
	const triggerEvent = useComponentEventTrigger(componentId);
	const onAction = useOnAction();
	const setElementValue = useSetElementValue();
	const { setByPath } = useData();
	const latest = useRef({
		props,
		backend,
		actionContext,
		triggerEvent,
		onAction,
		setElementValue,
		setByPath,
	});
	latest.current = {
		props,
		backend,
		actionContext,
		triggerEvent,
		onAction,
		setElementValue,
		setByPath,
	};
	const session = useRef<AbortController | null>(null);
	const dirty = useRef(false);
	const busy = useRef(false);
	const opened = useRef(false);
	// A renewed storage signature must not remount an editor holding unsaved bytes.
	const [sessionUrl, setSessionUrl] = useState(props.url);
	useEffect(() => {
		if (!opened.current) setSessionUrl(props.url);
	}, [props.url]);

	useEffect(() => {
		const controller = new AbortController();
		session.current = controller;
		const unregister = registerA2UINavigationGuard(
			actionContext.appId,
			surfaceId,
			() =>
				!busy.current &&
				(!dirty.current || window.confirm("Discard unsaved document changes?")),
		);
		const protectUnload = (event: BeforeUnloadEvent) => {
			if (!dirty.current && !busy.current) return;
			event.preventDefault();
			event.returnValue = "";
		};
		const protectLink = (event: MouseEvent) => {
			if (!dirty.current && !busy.current) return;
			if (
				event.button !== 0 ||
				event.metaKey ||
				event.ctrlKey ||
				event.shiftKey ||
				event.altKey
			)
				return;
			const anchor = (event.target as Element | null)?.closest?.("a[href]");
			if (
				!(anchor instanceof HTMLAnchorElement) ||
				anchor.target === "_blank" ||
				anchor.hasAttribute("download")
			)
				return;
			const destination = new URL(anchor.href, window.location.href);
			if (
				destination.pathname === window.location.pathname &&
				destination.search === window.location.search
			)
				return;
			if (
				busy.current ||
				!window.confirm("Discard unsaved document changes?")
			) {
				event.preventDefault();
				event.stopImmediatePropagation();
			}
		};
		window.addEventListener("beforeunload", protectUnload);
		document.addEventListener("click", protectLink, true);
		return () => {
			controller.abort();
			unregister();
			window.removeEventListener("beforeunload", protectUnload);
			document.removeEventListener("click", protectLink, true);
		};
	}, [actionContext.appId, surfaceId]);

	const notify = useCallback(
		<Name extends FilePreviewEventName>(
			name: Name,
			context: FilePreviewEventDetails[Name] & {
				src?: string;
				filename?: string;
			},
		) => {
			if (
				session.current?.signal.aborted ||
				!latest.current.actionContext.isPreviewMode
			)
				return;
			const current = latest.current;
			const payload = { src: source, filename, ...context, actionId: name };
			// Notification failures must not change the result of a completed save.
			void Promise.resolve()
				.then(async () => {
					if (session.current?.signal.aborted) return;
					current.onAction?.({
						type: "userAction",
						name,
						surfaceId,
						sourceComponentId: componentId,
						timestamp: Date.now(),
						context: payload,
					});
					await current.triggerEvent(
						name,
						current.props.component,
						payload,
						EXACT,
					);
				})
				.catch((error) =>
					console.error(`File preview ${name} handler failed`, error),
				);
		},
		[componentId, surfaceId, source, filename],
	);

	const onSave = useCallback(
		async (file: File) => {
			const current = latest.current;
			const signal = session.current?.signal;
			const actions = filePreviewSaveActions(current.props.component);
			const assertActive = () => {
				if (!signal || signal.aborted)
					throw new Error("The document closed before saving finished.");
				if (
					!latest.current.props.editable ||
					!latest.current.actionContext.isPreviewMode
				)
					throw new Error("Editing is no longer enabled for this document.");
			};
			assertActive();
			if (!actions.length)
				throw new Error(
					"Configure a save workflow before saving this document.",
				);
			// Chat attachments use the same helper. Upload online regardless of where
			// the save workflow runs so it receives a backend URL, never local bytes.
			const url = await current.backend.helperState.fileToUrl(file, false);
			assertActive();
			if (!/^https?:\/\//i.test(url))
				throw new Error(
					"The edited file could not be uploaded to temporary backend storage. Sign in and try again.",
				);
			const value: FilePreviewExportedFile = {
				name: file.name,
				size: file.size,
				type: file.type,
				url,
				backendUrl: url,
			};
			const context = {
				actionId: "save",
				url,
				signedUrls: url,
				value,
				file: value,
				src: source,
				filename: file.name,
			};
			current.setElementValue?.(`${surfaceId}/${componentId}`, value);
			const binding = current.props.component.value;
			if (binding && "path" in binding) current.setByPath(binding.path, value);
			current.onAction?.({
				type: "userAction",
				name: "save",
				surfaceId,
				sourceComponentId: componentId,
				timestamp: Date.now(),
				context,
			});
			await current.triggerEvent(
				"save",
				{ eventHandlers: { save: actions } },
				context,
				{ ...EXACT, requireSuccess: true },
			);
			if (!signal?.aborted) notify("saved", context);
		},
		[componentId, surfaceId, source, notify],
	);

	const onDirtyChange = useCallback(
		(value: boolean) => {
			if (dirty.current === value) return;
			dirty.current = value;
			latest.current.props.onDirtyChange?.(value);
			notify("dirtyChange", { dirty: value });
		},
		[notify],
	);
	const onBusyChange = useCallback((value: boolean) => {
		busy.current = value;
		latest.current.props.onBusyChange?.(value);
	}, []);
	const onReady = useCallback(() => {
		opened.current = true;
	}, []);
	const onError = useCallback(
		(message: string) => notify("error", { message }),
		[notify],
	);
	const documentProps = {
		toolTarget: actionContext.appId
			? { appId: actionContext.appId, surfaceId, componentId }
			: undefined,
		url: sessionUrl,
		create: props.create,
		filename,
		editable,
		initialEditing,
		showDownload,
		onSave:
			filePreviewSaveActions(props.component).length > 0 ? onSave : undefined,
		onDirtyChange,
		onBusyChange,
		onError,
		onReady,
	};
	return (
		<Suspense
			fallback={
				<output className="p-4 text-sm text-muted-foreground">
					Loading document viewer...
				</output>
			}
		>
			{viewer.id === "wordcraft" ? (
				<WordcraftDocument {...documentProps} />
			) : (
				<CraftDocument {...documentProps} viewer={viewer} />
			)}
		</Suspense>
	);
}
