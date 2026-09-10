"use client";

import { useTranslation } from "@flow-like/locales";
import {
	AlertTriangleIcon,
	CheckCircle2Icon,
	FileCode2Icon,
	ImportIcon,
	InfoIcon,
	Loader2Icon,
	UploadIcon,
	XCircleIcon,
} from "lucide-react";
import {
	type DragEvent,
	useCallback,
	useDeferredValue,
	useEffect,
	useId,
	useMemo,
	useRef,
	useState,
} from "react";
import { toast } from "sonner";
import { useInvoke } from "../../../hooks";
import type { IGenericCommand } from "../../../lib";
import {
	MAIN_FILE_ID,
	MAIN_FILE_LABEL,
	MODULE_FILE_EXTENSION,
	fileModuleId,
	validateModuleName,
} from "../../../lib/flow-modules";
import { detectFormat } from "../../../lib/importer/detect";
import {
	type ImportTarget,
	buildImportCommands,
	importedModuleId,
} from "../../../lib/importer/import-commands";
import { translateImport } from "../../../lib/importer/translate";
import type {
	ImportDetection,
	ImportFormat,
	TranslationDiagnostic,
	TranslationResult,
} from "../../../lib/importer/types";
import type { IBoard } from "../../../lib/schema/flow/board";
import { cn } from "../../../lib/utils";
import { useBackend } from "../../../state/backend-state";
import { Badge } from "../../ui/badge";
import { Button } from "../../ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../ui/dialog";
import { Input } from "../../ui/input";
import { Label } from "../../ui/label";
import { RadioGroup, RadioGroupItem } from "../../ui/radio-group";
import { ScrollArea } from "../../ui/scroll-area";
import { Textarea } from "../../ui/textarea";

type ImportSource = Exclude<ImportFormat, "unknown">;
type ImportPlacement = "module" | "current";

interface SourceOption {
	id: ImportSource;
	label: string;
	description: string;
	accept: string;
	badge?: string;
}

/**
 * Every importer the board knows, as a picker. Detection is by content, so the
 * tile only chooses the file filter and the copy; a Dify file dropped while the
 * BPMN tile is selected still imports as Dify, and the tile follows.
 */
function useSourceOptions(): SourceOption[] {
	const { t } = useTranslation("flow");
	return useMemo(
		() => [
			{
				id: "bpmn",
				label: t("importSourceBpmn", "BPMN 2.0"),
				description: t(
					"importSourceBpmnDescription",
					"Process diagrams from Camunda, Flowable, bpmn.io and other BPMN tools. Every element is placed; steps without a node become layers to model.",
				),
				accept: ".bpmn,.xml,application/xml,text/xml",
			},
			{
				id: "n8n",
				label: t("importSourceN8n", "n8n"),
				description: t(
					"importSourceN8nDescription",
					"Workflow JSON exported from n8n. Common nodes map directly, the rest become layers to model.",
				),
				accept: ".json,application/json",
				badge: t("importPartialSupport", "Partial"),
			},
			{
				id: "dify",
				label: t("importSourceDify", "Dify"),
				description: t(
					"importSourceDifyDescription",
					"App DSL exported from Dify as YAML or JSON. Core nodes map directly, the rest become layers to model.",
				),
				accept: ".yml,.yaml,.json,application/json",
				badge: t("importPartialSupport", "Partial"),
			},
		],
		[t],
	);
}

/** `Order handling` stays; `order_handling-v2.bpmn` becomes `order handling v2`. */
function suggestModuleName(
	fileName: string | undefined,
	fallback: string,
): string {
	const raw = fileName ? fileName.replace(/\.[^.]+$/, "").trim() : fallback;
	return raw.replace(/[_-]+/g, " ").replace(/\s+/g, " ").trim() || fallback;
}

function uniqueModuleName(
	base: string,
	layers: IBoard["layers"] | undefined,
	parentId: string | null,
	reservedRoots: readonly string[],
): string {
	if (!validateModuleName(base, layers, parentId, reservedRoots)) return base;
	for (let i = 2; i < 100; i += 1) {
		const candidate = `${base} ${i}`;
		if (!validateModuleName(candidate, layers, parentId, reservedRoots)) {
			return candidate;
		}
	}
	return base;
}

/**
 * Reads an exported workflow file and places it on the board: BPMN 2.0 today,
 * n8n and Dify through their existing translators. What a format cannot express
 * as a node becomes a layer to model, which the preview counts before anything
 * is written.
 */
export function ImportFlowDialog({
	open,
	onOpenChange,
	appId,
	board,
	currentFileId,
	reservedRoots,
	executeCommands,
	onImported,
}: Readonly<{
	open: boolean;
	onOpenChange: (open: boolean) => void;
	appId: string;
	board?: IBoard;
	/** `main` or a module layer id: the file an "into current file" import lands in. */
	currentFileId: string;
	reservedRoots: readonly string[];
	/** Runs the whole import as one undo step. */
	executeCommands: (commands: IGenericCommand[]) => Promise<unknown>;
	/** Called with the new module's id, or `null` when imported into the open file. */
	onImported: (moduleId: string | null) => void;
}>) {
	const { t } = useTranslation("flow");
	const backend = useBackend();
	const options = useSourceOptions();

	const catalog = useInvoke(
		backend.boardState.getCatalog,
		backend.boardState,
		[appId],
		open && Boolean(appId),
	);

	const [source, setSource] = useState<ImportSource>("bpmn");
	const [text, setText] = useState("");
	const [fileName, setFileName] = useState<string | undefined>();
	const [pasting, setPasting] = useState(false);
	const [placement, setPlacement] = useState<ImportPlacement>("module");
	const [moduleName, setModuleName] = useState("");
	const [nameEdited, setNameEdited] = useState(false);
	const [busy, setBusy] = useState(false);

	useEffect(() => {
		if (open) return;
		setSource("bpmn");
		setText("");
		setFileName(undefined);
		setPasting(false);
		setPlacement("module");
		setModuleName("");
		setNameEdited(false);
		setBusy(false);
	}, [open]);

	// Typing in the paste field would otherwise re-parse and re-translate the
	// whole document on every keystroke.
	const deferredText = useDeferredValue(text);
	const detection = useMemo<ImportDetection | undefined>(() => {
		const trimmed = deferredText.trim();
		return trimmed ? detectFormat(trimmed) : undefined;
	}, [deferredText]);

	const catalogReady = Boolean(catalog.data);
	const result = useMemo(() => {
		if (!detection || detection.format === "unknown") return undefined;
		try {
			return translateImport(detection, catalog.data);
		} catch (error) {
			console.error("Import translation failed", error);
			return undefined;
		}
	}, [detection, catalog.data]);

	useEffect(() => {
		if (detection && detection.format !== "unknown")
			setSource(detection.format);
	}, [detection]);

	const currentModuleId = fileModuleId(currentFileId) ?? null;
	const currentFileLabel = useMemo(() => {
		if (currentFileId === MAIN_FILE_ID) return MAIN_FILE_LABEL;
		const layer = board?.layers?.[currentFileId];
		return layer ? `${layer.name}${MODULE_FILE_EXTENSION}` : MAIN_FILE_LABEL;
	}, [board?.layers, currentFileId]);

	useEffect(() => {
		if (nameEdited || !result) return;
		setModuleName(
			uniqueModuleName(
				suggestModuleName(fileName, result.board.name),
				board?.layers,
				null,
				reservedRoots,
			),
		);
	}, [result, fileName, nameEdited, board?.layers, reservedRoots]);

	const nameError = useMemo(() => {
		if (placement !== "module") return null;
		const error = validateModuleName(
			moduleName,
			board?.layers,
			null,
			reservedRoots,
		);
		switch (error) {
			case null:
				return null;
			case "empty":
				return t("nameIsRequired", "Name is required");
			case "reserved":
				return t("thatNameIsReserved", "That name is reserved");
			case "duplicate":
				return t("thatNameIsAlreadyTaken", "That name is already taken");
			default:
				return t("useALetterOrDigitName", "Use a letter or digit name");
		}
	}, [placement, moduleName, board?.layers, reservedRoots, t]);

	const readFile = useCallback(async (file: File | undefined) => {
		if (!file) return;
		setFileName(file.name);
		setText(await file.text());
		setPasting(false);
	}, []);

	// A translation that produced no node at all — every element failed against
	// this deployment's catalog — would import an empty module.
	const placeable = Boolean(
		result && Object.keys(result.board.nodes).length > 0,
	);
	// The translation is only faithful against the real catalog: without it the
	// translators fall back to their built-in pin tables and place nodes a
	// deployment may not even have.
	const canImport =
		placeable &&
		catalogReady &&
		!busy &&
		(placement === "current" || (moduleName.trim() !== "" && !nameError));

	const runImport = useCallback(async () => {
		if (!result) return;
		setBusy(true);
		try {
			const target: ImportTarget =
				placement === "module"
					? { kind: "module", name: moduleName.trim(), parentId: null }
					: { kind: "layer", layerId: currentModuleId ?? undefined };
			const plan = buildImportCommands(result, target);
			const executed = await executeCommands(plan.commands);
			// The command layer resolves with nothing (after its own toast) when it
			// refused the batch — a read-only version, or no backend.
			if (!executed) {
				setBusy(false);
				return;
			}
			toast.success(
				result.stats.todo > 0
					? t("importedElementsWithPlaceholders", {
							defaultValue_one:
								"Imported {{count}} element, {{todo}} still need modelling",
							defaultValue_other:
								"Imported {{count}} elements, {{todo}} still need modelling",
							count: result.stats.totalNodes,
							todo: result.stats.todo,
						})
					: t("importedElements", {
							defaultValue_one: "Imported {{count}} element",
							defaultValue_other: "Imported {{count}} elements",
							count: result.stats.totalNodes,
						}),
			);
			onImported(
				placement === "module"
					? (importedModuleId(executed as IGenericCommand[]) ?? null)
					: null,
			);
			onOpenChange(false);
		} catch (error) {
			// `executeCommands` has already reported why; keep the dialog open with
			// the parsed file so the import can be retried.
			console.error("Import failed", error);
			setBusy(false);
		}
	}, [
		result,
		placement,
		moduleName,
		currentModuleId,
		executeCommands,
		onImported,
		onOpenChange,
		t,
	]);

	const active = options.find((option) => option.id === source) ?? options[0];
	const detectedLabel =
		options.find((option) => option.id === detection?.format)?.label ??
		active.label;

	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				if (next || !busy) onOpenChange(next);
			}}
		>
			<DialogContent
				className="flex max-h-[85vh] flex-col sm:max-w-2xl"
				onDoubleClick={(event) => event.stopPropagation()}
				// A file released anywhere but the intake zone would otherwise be
				// opened by the browser, replacing the app.
				onDragOver={(event) => event.preventDefault()}
				onDrop={(event) => event.preventDefault()}
				onEscapeKeyDown={(event) => {
					if (busy) event.preventDefault();
				}}
				onPointerDownOutside={(event) => {
					if (busy) event.preventDefault();
				}}
			>
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<ImportIcon className="size-4" />
						{t("importFlow", "Import flow")}
					</DialogTitle>
					<DialogDescription>
						{t(
							"importFlowDescription",
							"Bring an existing process or workflow onto this board. Steps that have a node are placed as nodes; the rest become layers you can open and model.",
						)}
					</DialogDescription>
				</DialogHeader>

				<div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto pr-1">
					<SourcePicker
						options={options}
						selected={source}
						disabled={busy}
						onSelect={setSource}
					/>

					<FileIntake
						accept={active.accept}
						fileName={fileName}
						disabled={busy}
						pasting={pasting}
						onTogglePaste={() => setPasting((value) => !value)}
						onFile={(file) => void readFile(file)}
					/>

					{pasting && (
						<Textarea
							autoFocus
							value={text}
							spellCheck={false}
							placeholder={t(
								"pasteTheExportedFileContentHere",
								"Paste the exported file content here",
							)}
							className="max-h-40 min-h-24 font-mono text-xs"
							onChange={(event) => {
								setFileName(undefined);
								setText(event.target.value);
							}}
						/>
					)}

					{detection?.format === "unknown" && (
						<p className="flex items-start gap-2 text-xs text-destructive">
							<XCircleIcon className="mt-0.5 size-3.5 shrink-0" />
							{detection.error
								? t("couldNotReadImportWithReason", {
										defaultValue: "Could not read this file: {{reason}}",
										reason: detection.error,
									})
								: t(
										"couldNotRecogniseImport",
										"This does not look like a BPMN, n8n or Dify export.",
									)}
						</p>
					)}

					{result && (
						<ImportPreview
							result={result}
							sourceLabel={detectedLabel}
							catalogReady={catalogReady}
						/>
					)}

					{result && !placeable && (
						<p className="flex items-start gap-2 text-xs text-destructive">
							<XCircleIcon className="mt-0.5 size-3.5 shrink-0" />
							{t(
								"nothingCouldBePlacedFromThisFile",
								"Nothing from this file could be placed on the board. The notes above say which nodes were unavailable.",
							)}
						</p>
					)}

					{result && placeable && (
						<PlacementPicker
							placement={placement}
							onPlacement={setPlacement}
							moduleName={moduleName}
							nameError={nameError}
							currentFileLabel={currentFileLabel}
							disabled={busy}
							onModuleName={(value) => {
								setNameEdited(true);
								setModuleName(value);
							}}
						/>
					)}
				</div>

				<DialogFooter>
					<Button
						variant="outline"
						disabled={busy}
						onClick={() => onOpenChange(false)}
					>
						{t("cancel", "Cancel")}
					</Button>
					<Button disabled={!canImport} onClick={() => void runImport()}>
						{busy ? (
							<Loader2Icon className="size-4 animate-spin" />
						) : (
							<ImportIcon className="size-4" />
						)}
						{t("import", "Import")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function SourcePicker({
	options,
	selected,
	disabled,
	onSelect,
}: Readonly<{
	options: SourceOption[];
	selected: ImportSource;
	disabled: boolean;
	onSelect: (source: ImportSource) => void;
}>) {
	return (
		<div className="grid grid-cols-3 gap-2">
			{options.map((option) => (
				<button
					key={option.id}
					type="button"
					aria-pressed={selected === option.id}
					disabled={disabled}
					onClick={() => onSelect(option.id)}
					className={cn(
						"flex flex-col rounded-lg border p-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
						selected === option.id
							? "border-primary bg-primary/10"
							: "hover:bg-muted/50",
					)}
				>
					<span className="flex items-center gap-2">
						<FileCode2Icon className="size-4 shrink-0 text-primary" />
						<span className="text-sm font-medium">{option.label}</span>
						{option.badge && (
							<Badge variant="outline" className="ml-auto text-[10px]">
								{option.badge}
							</Badge>
						)}
					</span>
					<span className="mt-1 text-xs leading-snug text-muted-foreground">
						{option.description}
					</span>
				</button>
			))}
		</div>
	);
}

function FileIntake({
	accept,
	fileName,
	disabled,
	pasting,
	onTogglePaste,
	onFile,
}: Readonly<{
	accept: string;
	fileName?: string;
	disabled: boolean;
	pasting: boolean;
	onTogglePaste: () => void;
	onFile: (file: File | undefined) => void;
}>) {
	const { t } = useTranslation("flow");
	const input = useRef<HTMLInputElement | null>(null);
	const textareaId = useId();
	const [dragging, setDragging] = useState(false);

	const drop = useCallback(
		(event: DragEvent<HTMLDivElement>) => {
			event.preventDefault();
			setDragging(false);
			onFile(event.dataTransfer.files?.[0]);
		},
		[onFile],
	);

	return (
		<div
			className={cn(
				"flex flex-col items-center gap-2 rounded-xl border border-dashed bg-muted/20 p-4 text-center transition-colors",
				dragging && "border-primary bg-primary/5",
			)}
			onDragOver={(event) => {
				event.preventDefault();
				setDragging(true);
			}}
			onDragLeave={() => setDragging(false)}
			onDrop={drop}
		>
			<Button
				type="button"
				variant="outline"
				size="sm"
				disabled={disabled}
				onClick={() => input.current?.click()}
			>
				<UploadIcon className="size-4" />
				{t("chooseFile", "Choose file")}
			</Button>
			<p className="text-xs text-muted-foreground">
				{fileName ??
					t(
						"dropAFileHereOrPasteBelow",
						"Or drop a file here. You can also paste the content.",
					)}
			</p>
			<button
				type="button"
				aria-expanded={pasting}
				aria-controls={textareaId}
				className="text-xs text-primary underline-offset-2 hover:underline"
				onClick={onTogglePaste}
			>
				{pasting
					? t("hidePasteField", "Hide paste field")
					: t("pasteInstead", "Paste instead")}
			</button>
			<input
				ref={input}
				type="file"
				accept={accept}
				className="sr-only"
				tabIndex={-1}
				aria-label={t("chooseFile", "Choose file")}
				onChange={(event) => {
					const file = event.target.files?.[0];
					event.target.value = "";
					onFile(file);
				}}
			/>
		</div>
	);
}

function PlacementPicker({
	placement,
	onPlacement,
	moduleName,
	nameError,
	currentFileLabel,
	disabled,
	onModuleName,
}: Readonly<{
	placement: ImportPlacement;
	onPlacement: (placement: ImportPlacement) => void;
	moduleName: string;
	nameError: string | null;
	currentFileLabel: string;
	disabled: boolean;
	onModuleName: (name: string) => void;
}>) {
	const { t } = useTranslation("flow");
	const moduleId = useId();
	const currentId = useId();
	const nameId = useId();
	const errorId = useId();

	return (
		<RadioGroup
			value={placement}
			onValueChange={(value) => onPlacement(value as ImportPlacement)}
			className="gap-2 rounded-lg border p-3"
		>
			<span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
				{t("placeImportAs", "Place as")}
			</span>
			<div className="flex items-center gap-2">
				<RadioGroupItem value="module" id={moduleId} disabled={disabled} />
				<Label htmlFor={moduleId} className="shrink-0 text-sm font-normal">
					{t("newModuleFile", "New module")}
				</Label>
				<Input
					id={nameId}
					value={moduleName}
					disabled={placement !== "module" || disabled}
					aria-label={t("moduleName", "Module name")}
					aria-invalid={Boolean(nameError)}
					aria-describedby={nameError ? errorId : undefined}
					className="h-7 font-mono text-xs"
					onChange={(event) => onModuleName(event.target.value)}
				/>
				<span className="shrink-0 font-mono text-xs text-muted-foreground">
					{MODULE_FILE_EXTENSION}
				</span>
			</div>
			{nameError && placement === "module" && (
				<span id={errorId} className="pl-6 text-[11px] text-destructive">
					{nameError}
				</span>
			)}
			<div className="flex items-center gap-2">
				<RadioGroupItem value="current" id={currentId} disabled={disabled} />
				<Label htmlFor={currentId} className="text-sm font-normal">
					{t("intoCurrentFile", {
						defaultValue: "Into {{file}}",
						file: currentFileLabel,
					})}
				</Label>
			</div>
		</RadioGroup>
	);
}

const MAX_SHOWN_DIAGNOSTICS = 6;

function ImportPreview({
	result,
	sourceLabel,
	catalogReady,
}: Readonly<{
	result: TranslationResult;
	sourceLabel: string;
	catalogReady: boolean;
}>) {
	const { t } = useTranslation("flow");
	const { stats } = result;
	const problems = useMemo(
		() =>
			[...result.diagnostics]
				.filter((diagnostic) => diagnostic.level !== "info")
				.sort((a, b) => levelRank(a) - levelRank(b)),
		[result.diagnostics],
	);
	const [showAll, setShowAll] = useState(false);
	const shown = showAll ? problems : problems.slice(0, MAX_SHOWN_DIAGNOSTICS);

	return (
		<div className="flex flex-col gap-2 rounded-lg border bg-card p-3">
			<div className="flex flex-wrap items-center gap-2 text-sm">
				{catalogReady ? (
					<CheckCircle2Icon className="size-4 text-emerald-500" />
				) : (
					<Loader2Icon className="size-4 animate-spin text-muted-foreground" />
				)}
				<span className="font-medium">
					{t("detectedImport", {
						defaultValue: "{{source}} · {{name}}",
						source: sourceLabel,
						name: result.board.name,
					})}
				</span>
				{!catalogReady && (
					<span className="text-xs text-muted-foreground">
						{t("catalogStillLoading", "Catalog still loading")}
					</span>
				)}
			</div>
			<div className="flex flex-wrap gap-1.5 text-[11px] tabular-nums">
				<Badge variant="secondary">
					{t("countElements", {
						defaultValue_one: "{{count}} element",
						defaultValue_other: "{{count}} elements",
						count: stats.totalNodes,
					})}
				</Badge>
				<Badge variant="outline">
					{t("countMapped", {
						defaultValue_one: "{{count}} mapped",
						defaultValue_other: "{{count}} mapped",
						count: stats.directMapped,
					})}
				</Badge>
				{stats.composed > 0 && (
					<Badge variant="outline">
						{t("countComposed", {
							defaultValue_one: "{{count}} composed",
							defaultValue_other: "{{count}} composed",
							count: stats.composed,
						})}
					</Badge>
				)}
				<Badge
					variant="outline"
					className={cn(
						stats.todo > 0 &&
							"border-amber-500/50 text-amber-600 dark:text-amber-400",
					)}
				>
					{t("countToModel", {
						defaultValue_one: "{{count}} to model",
						defaultValue_other: "{{count}} to model",
						count: stats.todo,
					})}
				</Badge>
				<Badge variant="outline">
					{t("countConnections", {
						defaultValue_one: "{{count}} connection",
						defaultValue_other: "{{count}} connections",
						count: stats.connections,
					})}
				</Badge>
				{stats.variables > 0 && (
					<Badge variant="outline">
						{t("countVariables", {
							defaultValue_one: "{{count}} variable",
							defaultValue_other: "{{count}} variables",
							count: stats.variables,
						})}
					</Badge>
				)}
			</div>
			{problems.length > 0 && (
				<ScrollArea className={cn(showAll && "max-h-48")}>
					<ul className="space-y-1 pr-3">
						{shown.map((diagnostic, index) => (
							<li
								key={`${diagnostic.nodeId ?? ""}-${index}`}
								className="flex items-start gap-1.5 text-xs"
							>
								<DiagnosticIcon level={diagnostic.level} />
								<span className="min-w-0 break-words text-muted-foreground">
									{diagnostic.nodeName && (
										<span className="font-medium text-foreground">
											{diagnostic.nodeName}:{" "}
										</span>
									)}
									{diagnostic.message}
								</span>
							</li>
						))}
					</ul>
				</ScrollArea>
			)}
			{problems.length > MAX_SHOWN_DIAGNOSTICS && (
				<button
					type="button"
					className="self-start text-xs text-primary underline-offset-2 hover:underline"
					onClick={() => setShowAll((value) => !value)}
				>
					{showAll
						? t("showFewer", "Show fewer")
						: t("showAllCountNotes", {
								defaultValue_one: "Show the remaining note",
								defaultValue_other: "Show all {{count}} notes",
								count: problems.length,
							})}
				</button>
			)}
		</div>
	);
}

function levelRank(diagnostic: TranslationDiagnostic): number {
	return diagnostic.level === "error" ? 0 : diagnostic.level === "warn" ? 1 : 2;
}

function DiagnosticIcon({
	level,
}: Readonly<{ level: TranslationDiagnostic["level"] }>) {
	if (level === "error") {
		return <XCircleIcon className="mt-0.5 size-3 shrink-0 text-destructive" />;
	}
	if (level === "warn") {
		return (
			<AlertTriangleIcon className="mt-0.5 size-3 shrink-0 text-amber-500" />
		);
	}
	return <InfoIcon className="mt-0.5 size-3 shrink-0 text-muted-foreground" />;
}
