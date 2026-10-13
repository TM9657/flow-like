"use client";

import { useTranslation } from "@flow-like/locales";
import * as RadioGroupPrimitive from "@radix-ui/react-radio-group";
import {
	AlertTriangle,
	ChevronLeft,
	Database,
	Fingerprint,
	History,
	KeyRound,
	Loader2,
	Lock,
	Plus,
	Search,
	Settings,
	Trash2,
	Zap,
} from "lucide-react";
import type * as React from "react";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { cn } from "../../lib";
import {
	type IColumnMetadata,
	MAX_COLUMN_ATTRIBUTE_CHARS,
	MAX_COLUMN_DESCRIPTION_CHARS,
	MAX_COLUMN_DISPLAY_NAME_CHARS,
	isColumnReadRestricted,
	normalizeColumnMetadata,
	sameColumnMetadata,
} from "../../lib/column-metadata";
import type { DataClassification } from "../../lib/data-classification";
import { asArray } from "../../lib/response-shape";
import {
	type IAddColumnPayload,
	type IIndexConfig,
	IIndexType,
	parseIndexType,
} from "../../state/backend-state/db-state";
import { AttributeTagsInput } from "./attribute-tags-input";
import { Badge } from "./badge";
import { Button } from "./button";
import {
	DataClassificationBadge,
	DataClassificationDot,
	DataSensitivityFields,
	PiiBadge,
	useDataClassificationCopy,
} from "./data-classification";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "./dialog";
import { Input } from "./input";
import { Label } from "./label";
import type { LanceField, LanceSchema } from "./lance-viewer";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "./table";
import {
	EDIT_COLUMN_TYPE_GROUPS,
	INDEX_TYPE_OPTIONS,
	IndexTypeHelp,
	IndexTypeSelect,
	addColumnSqlType,
	buildAddColumnExpression,
	validateColumnDefault,
	validateColumnName,
} from "./table-schema";
import { Textarea } from "./textarea";

export interface TableSchemaDialogProps {
	schema: LanceSchema | null;
	tableName?: string;
	rowCount?: number;
	onDropColumns?: (columns: string[]) => Promise<void>;
	onAddColumn?: (column: IAddColumnPayload) => Promise<void>;
	onAlterColumn?: (column: string, nullable: boolean) => Promise<void>;
	onSetPrimaryKey?: (column: string) => Promise<void>;
	/** Replaces a column's metadata. Without it the column details are read-only. */
	onSetColumnMetadata?: (
		column: string,
		metadata: IColumnMetadata,
	) => Promise<unknown>;
	/** Unlocks the read-attribute editor; only admins and owners may change it. */
	canManageColumnAccess?: boolean;
	/** Role attributes offered as read attributes. Unknown when omitted. */
	knownAttributes?: readonly string[];
	onBuildIndex?: (column: string, indexType: string) => Promise<void>;
	onGetIndices?: () => Promise<IIndexConfig[]>;
	onDropIndex?: (indexName: string) => Promise<void>;
}

type SchemaPane = "column" | "new" | "indices";
type IndexStatus = "idle" | "loading" | "ready" | "error";
type ColumnConfirm = "nullable" | "key" | "drop";
type ColumnAction = ColumnConfirm | "required" | "buildIndex" | "dropIndex";

export const describeField = (f: LanceField): string => {
	switch (f.kind) {
		case "vector":
			return `${f.kind}${f.dims ? `(${f.dims})` : ""}`;
		case "array":
			return `array<${
				typeof f.items === "string" ? f.items : (f.items?.kind ?? "unknown")
			}>`;
		default:
			return f.kind;
	}
};

const TEMPORAL_SUFFIX: Record<string, string> = {
	second: "s",
	millisecond: "ms",
	microsecond: "us",
	nanosecond: "ns",
};

export function fieldTypeLabel(field: LanceField): string {
	const suffix =
		field.kind === "date" && field.temporal
			? TEMPORAL_SUFFIX[field.temporal]
			: undefined;
	return suffix ? `timestamp(${suffix})` : describeField(field);
}

function columnIndex(indices: readonly IIndexConfig[], column: string) {
	return indices.find((index) => index.columns.includes(column));
}

function indexOption(index: IIndexConfig) {
	const type = parseIndexType(index.index_type);
	if (type === IIndexType.Auto) return undefined;
	return INDEX_TYPE_OPTIONS.find((option) => option.type === type);
}

const indexLabel = (index: IIndexConfig) =>
	indexOption(index)?.label ?? index.index_type;

/** Hosts toast their own failures and rethrow; the dialog only needs the outcome. */
async function attempt(task: () => Promise<unknown>): Promise<boolean> {
	try {
		await task();
		return true;
	} catch {
		return false;
	}
}

function useTableIndices(onGetIndices?: () => Promise<IIndexConfig[]>) {
	const [indices, setIndices] = useState<IIndexConfig[]>([]);
	const [status, setStatus] = useState<IndexStatus>("idle");
	const request = useRef(0);

	const reload = useCallback(async () => {
		if (!onGetIndices) return;
		const id = ++request.current;
		setStatus("loading");
		try {
			const next = await onGetIndices();
			if (id !== request.current) return;
			setIndices(asArray(next));
			setStatus("ready");
		} catch {
			if (id === request.current) setStatus("error");
		}
	}, [onGetIndices]);

	useEffect(() => {
		void reload();
	}, [reload]);

	return { indices, status, reload };
}

export function TableSchemaDialog(props: Readonly<TableSchemaDialogProps>) {
	const { t } = useTranslation("common");
	const [open, setOpen] = useState(false);
	const contentRef = useRef<HTMLDivElement>(null);
	return (
		<>
			<Button variant="outline" size="sm" onClick={() => setOpen(true)}>
				<Settings /> {t("schema", "Schema")}
			</Button>
			<Dialog open={open} onOpenChange={setOpen}>
				<DialogContent
					ref={contentRef}
					// Focusing the filter would pop the keyboard on touch devices.
					onOpenAutoFocus={(event) => {
						event.preventDefault();
						contentRef.current?.focus();
					}}
					className="h-[min(680px,calc(100dvh-2rem))] gap-0 overflow-hidden p-0 outline-none sm:max-w-5xl"
				>
					<SchemaWorkspace {...props} onDone={() => setOpen(false)} />
				</DialogContent>
			</Dialog>
		</>
	);
}

function SchemaWorkspace({
	onDone,
	...props
}: Readonly<TableSchemaDialogProps & { onDone: () => void }>) {
	const { t } = useTranslation("common");
	const { schema, onGetIndices, onAddColumn } = props;
	const fields = schema?.fields ?? [];
	const { indices, status, reload } = useTableIndices(onGetIndices);
	const [pane, setPane] = useState<SchemaPane>("column");
	const [columnName, setColumnName] = useState<string>();
	// Phones show the rail or the pane, never both.
	const [detailOnPhone, setDetailOnPhone] = useState(false);
	const selected =
		fields.find((field) => field.name === columnName) ?? fields[0];
	const editable = [
		props.onDropColumns,
		onAddColumn,
		props.onAlterColumn,
		props.onSetPrimaryKey,
		props.onSetColumnMetadata,
		props.onBuildIndex,
		props.onDropIndex,
	].some(Boolean);

	const openPane = (next: SchemaPane) => {
		setPane(next);
		setDetailOnPhone(true);
	};

	const openColumn = (name?: string) => {
		setColumnName(name);
		openPane("column");
	};

	const handleDropped = (name: string) => {
		const position = fields.findIndex((field) => field.name === name);
		openColumn((fields[position + 1] ?? fields[position - 1])?.name);
		void reload();
	};

	const facts = [
		typeof props.rowCount === "number"
			? t("countRows", {
					count: props.rowCount,
					defaultValue_one: "{{count}} row",
					defaultValue_other: "{{count}} rows",
				})
			: null,
		t("countColumns", {
			count: fields.length,
			defaultValue_one: "{{count}} column",
			defaultValue_other: "{{count}} columns",
		}),
		status === "ready"
			? t("countIndices", {
					count: indices.length,
					defaultValue_one: "{{count}} index",
					defaultValue_other: "{{count}} indices",
				})
			: null,
	]
		.filter(Boolean)
		.join(" · ");

	const renderPane = () => {
		if (pane === "new" && onAddColumn) {
			return (
				<NewColumnPane
					fields={fields}
					rowCount={props.rowCount}
					onAddColumn={onAddColumn}
					onAdded={openColumn}
					onCancel={() => setPane("column")}
				/>
			);
		}
		if (pane === "indices") {
			return (
				<IndicesPane
					fields={fields}
					indices={indices}
					status={status}
					canBuild={Boolean(props.onBuildIndex)}
					onRetry={reload}
					onOpenColumn={openColumn}
					onDropIndex={props.onDropIndex}
				/>
			);
		}
		if (!selected) {
			return (
				<p className="p-6 text-sm text-muted-foreground">
					{t("noSchemaLoadedYet", "No schema loaded yet.")}
				</p>
			);
		}
		return (
			<ColumnPane
				key={selected.name}
				field={selected}
				isLastColumn={fields.length === 1}
				primaryKey={schema?.primaryKey}
				index={columnIndex(indices, selected.name)}
				indexStatus={status}
				showIndex={Boolean(onGetIndices || props.onBuildIndex)}
				onAlterColumn={props.onAlterColumn}
				onSetPrimaryKey={props.onSetPrimaryKey}
				onBuildIndex={props.onBuildIndex}
				onDropIndex={props.onDropIndex}
				onDropColumns={props.onDropColumns}
				onIndicesChanged={reload}
				onDropped={handleDropped}
				details={
					<ColumnDetails
						field={selected}
						onSetColumnMetadata={props.onSetColumnMetadata}
						canManageColumnAccess={props.canManageColumnAccess ?? false}
						knownAttributes={props.knownAttributes}
					/>
				}
			/>
		);
	};

	return (
		<>
			<DialogHeader className="flex-row items-center gap-3 border-b px-5 py-4 pr-12 text-left">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-md border bg-muted/40">
					<Database className="size-4 text-muted-foreground" />
				</div>
				<div className="min-w-0 flex-1 space-y-1">
					<DialogTitle className="truncate text-base">
						{props.tableName ?? schema?.table}
					</DialogTitle>
					<DialogDescription className="truncate text-xs">
						{facts}
					</DialogDescription>
				</div>
			</DialogHeader>
			<div className="flex min-h-0 flex-1 flex-col sm:flex-row">
				<SchemaRail
					className={detailOnPhone ? "hidden sm:flex" : "flex"}
					fields={fields}
					indices={indices}
					primaryKey={schema?.primaryKey}
					selectedName={pane === "column" ? selected?.name : undefined}
					pane={pane}
					canAdd={Boolean(onAddColumn)}
					showIndices={Boolean(onGetIndices)}
					onOpenColumn={openColumn}
					onOpenPane={openPane}
				/>
				<div
					className={cn(
						"min-h-0 min-w-0 flex-1 overflow-y-auto",
						!detailOnPhone && "hidden sm:block",
					)}
				>
					<button
						type="button"
						onClick={() => setDetailOnPhone(false)}
						className="flex items-center gap-1 px-4 pt-3 text-sm text-muted-foreground outline-none hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50 sm:hidden"
					>
						<ChevronLeft className="size-4" />
						{t("columns", "Columns")}
					</button>
					{renderPane()}
				</div>
			</div>
			<div className="flex shrink-0 items-center gap-2.5 border-t px-5 py-3">
				{editable ? (
					<History className="size-4 shrink-0 text-muted-foreground" />
				) : (
					<Lock className="size-4 shrink-0 text-muted-foreground" />
				)}
				<p className="flex-1 text-xs text-muted-foreground">
					{editable
						? t(
								"schemaVersionNote",
								"Every change is saved as a new table version. You can roll back from History.",
							)
						: t(
								"schemaReadOnlyNote",
								"Read-only. Schema changes are turned off here.",
							)}
				</p>
				<Button variant="outline" size="sm" onClick={onDone}>
					{t("done", "Done")}
				</Button>
			</div>
		</>
	);
}

function SchemaRail({
	className,
	fields,
	indices,
	primaryKey,
	selectedName,
	pane,
	canAdd,
	showIndices,
	onOpenColumn,
	onOpenPane,
}: Readonly<{
	className?: string;
	fields: LanceField[];
	indices: IIndexConfig[];
	primaryKey?: string;
	selectedName?: string;
	pane: SchemaPane;
	canAdd: boolean;
	showIndices: boolean;
	onOpenColumn: (name: string) => void;
	onOpenPane: (pane: SchemaPane) => void;
}>) {
	const { t } = useTranslation("common");
	const [query, setQuery] = useState("");
	const needle = query.trim().toLowerCase();
	const matches = (field: LanceField) =>
		[field.name, field.columnMetadata?.display_name ?? ""].some((text) =>
			text.toLowerCase().includes(needle),
		);
	const visible = needle ? fields.filter(matches) : fields;

	return (
		<nav
			aria-label={t("columns", "Columns")}
			className={cn(
				"min-h-0 flex-1 flex-col bg-muted/20 sm:w-64 sm:flex-none sm:border-r",
				className,
			)}
		>
			<div className="p-3 pb-2">
				<div className="relative">
					<Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
					<Input
						type="search"
						value={query}
						onChange={(event) => setQuery(event.target.value)}
						placeholder={t("filterColumns", "Filter columns…")}
						aria-label={t("filterColumns", "Filter columns…")}
						className="h-8 pl-8"
					/>
				</div>
			</div>
			{canAdd && (
				<div className="px-2 pb-1">
					<RailButton active={pane === "new"} onClick={() => onOpenPane("new")}>
						<span className="flex size-6 shrink-0 items-center justify-center rounded-md border border-dashed border-muted-foreground/50">
							<Plus className="size-3.5" />
						</span>
						<span className="text-sm font-medium">
							{t("newColumn", "New column")}
						</span>
					</RailButton>
				</div>
			)}
			<div className="flex items-center justify-between px-4 pt-2 pb-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
				<span>{t("columns", "Columns")}</span>
				<span>{visible.length}</span>
			</div>
			<ul className="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-2 pb-2">
				{visible.map((field) => (
					<li key={field.name}>
						<RailButton
							active={field.name === selectedName}
							onClick={() => onOpenColumn(field.name)}
						>
							<span className="min-w-0 flex-1">
								<span className="block truncate font-mono text-[13px]">
									{field.name}
								</span>
								<span className="block truncate text-[11px] text-muted-foreground">
									{field.columnMetadata?.display_name &&
										`${field.columnMetadata.display_name} · `}
									{fieldTypeLabel(field)} ·{" "}
									{(field.nullable ?? true)
										? t("nullable", "Nullable")
										: t("required", "Required")}
								</span>
							</span>
							<ColumnMarkers metadata={field.columnMetadata} />
							{field.name === primaryKey && (
								<KeyRound
									role="img"
									aria-label={t("tableKeyBadge", "Key")}
									className="size-3.5 shrink-0 text-primary"
								/>
							)}
							{columnIndex(indices, field.name) && (
								<Zap
									role="img"
									aria-label={t("schemaIndexed", "Indexed")}
									className="size-3.5 shrink-0 text-primary"
								/>
							)}
						</RailButton>
					</li>
				))}
				{visible.length === 0 && (
					<li className="px-2 py-6 text-center text-xs text-muted-foreground">
						{t("schemaNoColumnsMatch", "No columns match")}
					</li>
				)}
			</ul>
			{showIndices && (
				<div className="border-t p-2">
					<RailButton
						active={pane === "indices"}
						onClick={() => onOpenPane("indices")}
					>
						<Zap className="size-4 shrink-0 text-primary" />
						<span className="flex-1 text-sm font-medium">
							{t("indices", "Indices")}
						</span>
						<span className="text-xs text-muted-foreground">
							{indices.length}
						</span>
					</RailButton>
				</div>
			)}
		</nav>
	);
}

function RailButton({
	active,
	className,
	...props
}: React.ComponentProps<"button"> & { active: boolean }) {
	return (
		<button
			type="button"
			aria-current={active || undefined}
			className={cn(
				"flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left outline-none transition-colors hover:bg-muted/60 focus-visible:ring-[3px] focus-visible:ring-ring/50",
				active && "bg-muted",
				className,
			)}
			{...props}
		/>
	);
}

function ColumnMarkers({
	metadata,
}: Readonly<{ metadata: IColumnMetadata | undefined }>) {
	const { t } = useTranslation("common");
	const levels = useDataClassificationCopy();
	const level = metadata?.classification;
	return (
		<>
			{level && (
				<span
					role="img"
					aria-label={levels[level].label}
					title={levels[level].label}
					className="inline-flex shrink-0"
				>
					<DataClassificationDot classification={level} />
				</span>
			)}
			{metadata?.pii && (
				<Fingerprint
					role="img"
					aria-label={t("piiLabel", "Contains personal data")}
					className="size-3.5 shrink-0 text-muted-foreground"
				/>
			)}
			{isColumnReadRestricted(metadata) && (
				<Lock
					role="img"
					aria-label={t("schemaReadRestricted", "Read-restricted")}
					className="size-3.5 shrink-0 text-muted-foreground"
				/>
			)}
		</>
	);
}

function ColumnPane({
	field,
	isLastColumn,
	primaryKey,
	index,
	indexStatus,
	showIndex,
	onAlterColumn,
	onSetPrimaryKey,
	onBuildIndex,
	onDropIndex,
	onDropColumns,
	onIndicesChanged,
	onDropped,
	details,
}: Readonly<{
	field: LanceField;
	isLastColumn: boolean;
	primaryKey?: string;
	index?: IIndexConfig;
	indexStatus: IndexStatus;
	showIndex: boolean;
	onAlterColumn?: (column: string, nullable: boolean) => Promise<void>;
	onSetPrimaryKey?: (column: string) => Promise<void>;
	onBuildIndex?: (column: string, indexType: string) => Promise<void>;
	onDropIndex?: (indexName: string) => Promise<void>;
	onDropColumns?: (columns: string[]) => Promise<void>;
	onIndicesChanged: () => Promise<void>;
	onDropped: (name: string) => void;
	/** Rendered between the column's properties and its drop control. */
	details?: React.ReactNode;
}>) {
	const { t } = useTranslation("common");
	const titleId = useId();
	const [confirm, setConfirm] = useState<ColumnConfirm | null>(null);
	const [busy, setBusy] = useState<ColumnAction | null>(null);
	const [indexType, setIndexType] = useState("auto");
	const nullable = field.nullable ?? true;
	const isKey = field.name === primaryKey;
	const canBecomeKey = !primaryKey && Boolean(field.keyEligible);
	const indexKind = field.indexKind ?? field.kind;
	const unsupportedIndex = indexKind === "unsupported-geometry";

	const run = async (action: ColumnAction, task: () => Promise<void>) => {
		setBusy(action);
		const ok = await attempt(task);
		setBusy(null);
		if (ok) setConfirm(null);
		return ok;
	};

	const allowNull = () =>
		onAlterColumn && run("nullable", () => onAlterColumn(field.name, true));
	// Lance refuses while any row holds NULL; the host reports that error.
	const makeRequired = () =>
		onAlterColumn && run("required", () => onAlterColumn(field.name, false));
	const setKey = () =>
		onSetPrimaryKey && run("key", () => onSetPrimaryKey(field.name));
	const buildIndex = async () => {
		if (!onBuildIndex) return;
		if (await run("buildIndex", () => onBuildIndex(field.name, indexType)))
			await onIndicesChanged();
	};
	const dropIndex = async () => {
		if (!onDropIndex || !index) return;
		if (await run("dropIndex", () => onDropIndex(index.name)))
			await onIndicesChanged();
	};
	const dropColumn = async () => {
		if (!onDropColumns || isKey || isLastColumn) return;
		if (await run("drop", () => onDropColumns([field.name])))
			onDropped(field.name);
	};

	const nullableAction = (() => {
		if (nullable) {
			if (!onAlterColumn) return null;
			return (
				<Button
					variant="outline"
					size="sm"
					disabled={busy !== null}
					onClick={() => void makeRequired()}
				>
					{busy === "required" && <Loader2 className="animate-spin" />}
					{t("schemaMakeRequired", "Make required")}
				</Button>
			);
		}
		if (isKey)
			return (
				<LockNote>{t("schemaKeyStaysRequired", "Key stays required")}</LockNote>
			);
		if (!onAlterColumn || confirm === "nullable") return null;
		return (
			<Button
				variant="outline"
				size="sm"
				disabled={busy !== null}
				onClick={() => setConfirm("nullable")}
			>
				{t("schemaAllowNull", "Allow NULL")}
			</Button>
		);
	})();

	const keyText = (() => {
		if (isKey)
			return t(
				"tableKeyCurrent",
				'"{{column}}" is the key. A key can\'t be changed or removed.',
				{ column: field.name },
			);
		if (canBecomeKey)
			return t(
				"tableKeyDescription",
				"Concurrent Upserts on the key column can't create duplicate rows. An Upsert makes its ID column the key when the table has none.",
			);
		return t(
			"tableKeyIneligible",
			"Only required text, 32/64-bit integer or binary columns can be the key.",
		);
	})();

	const keyAction = (() => {
		if (isKey) return <LockNote>{t("schemaPermanent", "Permanent")}</LockNote>;
		if (!onSetPrimaryKey || confirm === "key") return null;
		return (
			<Button
				variant="outline"
				size="sm"
				disabled={!canBecomeKey || busy !== null}
				onClick={() => setConfirm("key")}
			>
				<KeyRound /> {t("tableKeySetAction", "Set as key")}
			</Button>
		);
	})();

	const indexValue = (() => {
		if (index) {
			return (
				<div className="flex min-w-0 flex-col gap-1">
					<span className="flex min-w-0 items-center gap-2">
						<IndexChip>{indexLabel(index)}</IndexChip>
						<span className="truncate font-mono text-xs text-muted-foreground">
							{index.name}
						</span>
					</span>
					{indexOption(index)?.description && (
						<span className="text-xs text-muted-foreground">
							{indexOption(index)?.description}
						</span>
					)}
				</div>
			);
		}
		if (indexStatus === "loading" || indexStatus === "idle") {
			return (
				<span className="inline-flex items-center gap-2 text-xs text-muted-foreground">
					<Loader2 className="size-3.5 animate-spin" />
					{t("loading", "Loading…")}
				</span>
			);
		}
		if (indexStatus === "error") {
			return (
				<span className="text-xs text-muted-foreground">
					{t("schemaIndicesFailed", "Couldn't load the indices.")}
				</span>
			);
		}
		if (!onBuildIndex) {
			return (
				<span className="text-xs text-muted-foreground">
					{t("none", "None")}
				</span>
			);
		}
		return (
			<div className="flex min-w-0 flex-col gap-1.5">
				<IndexTypeSelect
					value={indexType}
					onChange={setIndexType}
					className="w-56 max-w-full"
					columnType={indexKind}
					disabled={busy !== null || unsupportedIndex}
				/>
				<IndexTypeHelp value={indexType} columnType={indexKind} />
			</div>
		);
	})();

	const indexAction = (() => {
		if (index && onDropIndex) {
			return (
				<Button
					variant="ghost"
					size="sm"
					className="text-destructive hover:text-destructive"
					disabled={busy !== null}
					onClick={() => void dropIndex()}
				>
					{busy === "dropIndex" && <Loader2 className="animate-spin" />}
					{t("dropIndex", "Drop index")}
				</Button>
			);
		}
		if (index || !onBuildIndex || indexStatus !== "ready") return null;
		return (
			<Button
				variant="outline"
				size="sm"
				disabled={busy !== null || unsupportedIndex}
				onClick={() => void buildIndex()}
			>
				{busy === "buildIndex" ? <Loader2 className="animate-spin" /> : <Zap />}
				{t("buildIndex", "Build index")}
			</Button>
		);
	})();

	return (
		<section aria-labelledby={titleId} className="flex flex-col gap-5 p-6">
			<h3
				id={titleId}
				className="flex min-w-0 items-center gap-2.5 font-mono text-xl font-medium"
			>
				<span className="truncate">{field.name}</span>
				{isKey && <KeyBadge />}
			</h3>

			<div className="divide-y rounded-lg border bg-card/40">
				<PropertyRow
					label={t("schemaTypeLabel", "Type")}
					hint={t("schemaTypeHint", "Set when the column is created")}
					value={<TypeChip>{fieldTypeLabel(field)}</TypeChip>}
					action={<LockNote>{t("schemaTypeFixed", "Can't change")}</LockNote>}
				/>
				<PropertyRow
					label={t("schemaEmptyValues", "Empty values")}
					hint={t("schemaEmptyValuesHint", "Whether rows can hold NULL")}
					value={<NullabilityValue nullable={nullable} />}
					action={nullableAction}
				>
					{confirm === "nullable" && (
						<ConfirmPanel
							title={t("schemaAllowNullTitle", "Allow NULL in {{name}}?", {
								name: field.name,
							})}
							description={[
								t(
									"schemaAllowNullBody",
									"Rows can then leave it empty. Making it required again only works while no row holds NULL.",
								),
								canBecomeKey && onSetPrimaryKey
									? t(
											"schemaAllowNullLosesKey",
											"While it's nullable, it can't be the table key.",
										)
									: null,
							]
								.filter(Boolean)
								.join(" ")}
							confirmLabel={t("schemaAllowNull", "Allow NULL")}
							busy={busy === "nullable"}
							onCancel={() => setConfirm(null)}
							onConfirm={() => void allowNull()}
						/>
					)}
				</PropertyRow>
				{(isKey || (!primaryKey && onSetPrimaryKey)) && (
					<PropertyRow
						label={t("schemaTableKey", "Table key")}
						hint={t("schemaTableKeyHint", "Stops duplicate rows from Upserts")}
						value={
							<span className="text-xs text-muted-foreground">{keyText}</span>
						}
						action={keyAction}
					>
						{confirm === "key" && (
							<ConfirmPanel
								title={t(
									"tableKeyConfirmTitle",
									'Make "{{column}}" the table key?',
									{ column: field.name },
								)}
								description={`${t(
									"tableKeyConfirmDescription",
									"The key can't be changed or removed afterwards. Concurrent Upserts on this column stop creating duplicate rows; Upserts on other columns are not protected.",
								)} ${t(
									"tableKeyConfirmDuplicates",
									"This fails if the column already holds duplicate values. Remove them first.",
								)}`}
								confirmLabel={t("tableKeySetAction", "Set as key")}
								busy={busy === "key"}
								onCancel={() => setConfirm(null)}
								onConfirm={() => void setKey()}
							/>
						)}
					</PropertyRow>
				)}
				{showIndex && (
					<PropertyRow
						label={t("schemaIndexLabel", "Index")}
						hint={t("schemaIndexHint", "Speeds up filters and search")}
						value={indexValue}
						action={indexAction}
					/>
				)}
			</div>

			{details}

			{onDropColumns && (isKey || isLastColumn) && (
				<div className="flex flex-col gap-3 rounded-lg border p-4 sm:flex-row sm:items-center">
					<div className="min-w-0 flex-1 space-y-1">
						<p className="text-sm font-medium">
							{t("schemaDropColumn", "Drop column")}
						</p>
						<p className="text-xs leading-relaxed text-muted-foreground">
							{isKey
								? t(
										"schemaKeyCannotBeDropped",
										"The table key can't be dropped. Upserts rely on it to keep rows unique.",
									)
								: t(
										"schemaLastColumnCannotBeDropped",
										"A table needs at least one column. Add a replacement before dropping this one.",
									)}
						</p>
					</div>
					<LockNote>
						{isKey
							? t("schemaPermanent", "Permanent")
							: t("schemaLastColumn", "Last column")}
					</LockNote>
				</div>
			)}
			{onDropColumns &&
				!isKey &&
				!isLastColumn &&
				(confirm === "drop" ? (
					<ConfirmPanel
						destructive
						className="m-0"
						title={t("schemaDropColumnTitle", "Drop {{name}}?", {
							name: field.name,
						})}
						description={dropDescription(t, index)}
						confirmLabel={t("schemaDropColumnNamed", "Drop {{name}}", {
							name: field.name,
						})}
						busy={busy === "drop"}
						onCancel={() => setConfirm(null)}
						onConfirm={() => void dropColumn()}
					/>
				) : (
					<div className="flex flex-col gap-3 rounded-lg border border-destructive/30 p-4 sm:flex-row sm:items-center">
						<div className="min-w-0 flex-1 space-y-1">
							<p className="text-sm font-medium">
								{t("schemaDropColumn", "Drop column")}
							</p>
							<p className="text-xs leading-relaxed text-muted-foreground">
								{dropDescription(t, index)}
							</p>
						</div>
						<Button
							variant="outline"
							size="sm"
							className="shrink-0 text-destructive hover:text-destructive"
							disabled={busy !== null}
							onClick={() => setConfirm("drop")}
						>
							<Trash2 /> {t("schemaDropColumn", "Drop column")}
						</Button>
					</div>
				))}
		</section>
	);
}

function dropDescription(
	t: ReturnType<typeof useTranslation>["t"],
	index?: IIndexConfig,
) {
	return index
		? t(
				"schemaDropColumnBodyIndexed",
				"Deletes the column and its {{index}} index. Flows that read it will fail. To undo, restore the previous version from History.",
				{ index: indexLabel(index) },
			)
		: t(
				"schemaDropColumnBody",
				"Deletes the column and its values. Flows that read it will fail. To undo, restore the previous version from History.",
			);
}

interface ColumnDetailsDraft {
	display_name: string;
	description: string;
	classification?: DataClassification;
	pii: boolean;
	read_attributes: string[];
}

function draftOf(metadata: IColumnMetadata): ColumnDetailsDraft {
	return {
		display_name: metadata.display_name ?? "",
		description: metadata.description ?? "",
		classification: metadata.classification,
		pii: metadata.pii === true,
		read_attributes: metadata.read_attributes ?? [],
	};
}

function ColumnDetails({
	field,
	onSetColumnMetadata,
	canManageColumnAccess,
	knownAttributes,
}: Readonly<{
	field: LanceField;
	onSetColumnMetadata?: (
		column: string,
		metadata: IColumnMetadata,
	) => Promise<unknown>;
	canManageColumnAccess: boolean;
	knownAttributes?: readonly string[];
}>) {
	const stored = normalizeColumnMetadata(field.columnMetadata);
	if (!onSetColumnMetadata) return <ColumnDetailsView metadata={stored} />;
	return (
		<ColumnDetailsForm
			// A stored change, from this form or another editor, restarts the draft from it.
			key={JSON.stringify(stored)}
			column={field.name}
			stored={stored}
			onSave={onSetColumnMetadata}
			canManageColumnAccess={canManageColumnAccess}
			knownAttributes={knownAttributes}
		/>
	);
}

function DetailsSection({
	status,
	children,
}: Readonly<{ status?: React.ReactNode; children: React.ReactNode }>) {
	const { t } = useTranslation("common");
	const titleId = useId();
	return (
		<section aria-labelledby={titleId} className="rounded-lg border bg-card/40">
			<div className="flex flex-wrap items-start gap-x-3 gap-y-1 border-b px-4 py-3.5">
				<div className="min-w-0 flex-1 space-y-0.5">
					<h4 id={titleId} className="text-sm font-medium">
						{t("details", "Details")}
					</h4>
					<p className="text-xs text-muted-foreground">
						{t(
							"schemaColumnDetailsHint",
							"What the column means, how sensitive it is and who can read it.",
						)}
					</p>
				</div>
				{status}
			</div>
			{children}
		</section>
	);
}

function AttributeChips({
	attributes,
}: Readonly<{ attributes: readonly string[] }>) {
	return (
		<span className="flex flex-wrap gap-1.5">
			{attributes.map((attribute) => (
				<span
					key={attribute}
					className="inline-flex items-center rounded-md border bg-muted/50 px-2 py-0.5 font-mono text-xs"
				>
					{attribute}
				</span>
			))}
		</span>
	);
}

function DetailRow({
	label,
	children,
}: Readonly<{ label: string; children: React.ReactNode }>) {
	return (
		<div className="grid gap-x-5 gap-y-1 px-4 py-3 sm:grid-cols-[11rem_minmax(0,1fr)] sm:items-baseline">
			<dt className="text-sm font-medium">{label}</dt>
			<dd className="min-w-0 wrap-break-word text-sm">{children}</dd>
		</div>
	);
}

function MutedValue({ children }: Readonly<{ children: React.ReactNode }>) {
	return <span className="text-muted-foreground">{children}</span>;
}

function ColumnDetailsView({
	metadata,
}: Readonly<{ metadata: IColumnMetadata }>) {
	const { t } = useTranslation("common");
	const levels = useDataClassificationCopy();
	const notSet = <MutedValue>{t("schemaNotSet", "Not set")}</MutedValue>;
	const readers = metadata.read_attributes ?? [];
	return (
		<DetailsSection>
			<dl className="divide-y">
				<DetailRow label={t("schemaDisplayName", "Display name")}>
					{metadata.display_name ?? notSet}
				</DetailRow>
				<DetailRow label={t("description", "Description")}>
					{metadata.description ? (
						<span className="whitespace-pre-line">{metadata.description}</span>
					) : (
						notSet
					)}
				</DetailRow>
				<DetailRow label={t("classification", "Classification")}>
					{metadata.classification ? (
						<DataClassificationBadge classification={metadata.classification} />
					) : (
						<MutedValue>{levels.unset.label}</MutedValue>
					)}
				</DetailRow>
				<DetailRow label={t("piiLabel", "Contains personal data")}>
					{metadata.pii ? (
						<PiiBadge />
					) : (
						<MutedValue>{t("no", "No")}</MutedValue>
					)}
				</DetailRow>
				<DetailRow label={t("schemaAllowedToRead", "Allowed to read")}>
					{readers.length > 0 ? (
						<AttributeChips attributes={readers} />
					) : (
						<MutedValue>
							{t(
								"schemaAllowedToReadEveryone",
								"Everyone who can read the table",
							)}
						</MutedValue>
					)}
				</DetailRow>
			</dl>
		</DetailsSection>
	);
}

function ColumnDetailsForm({
	column,
	stored,
	onSave,
	canManageColumnAccess,
	knownAttributes,
}: Readonly<{
	column: string;
	stored: IColumnMetadata;
	onSave: (column: string, metadata: IColumnMetadata) => Promise<unknown>;
	canManageColumnAccess: boolean;
	knownAttributes?: readonly string[];
}>) {
	const { t } = useTranslation("common");
	const ids = useId();
	const [baseline, setBaseline] = useState(stored);
	const [draft, setDraft] = useState(() => draftOf(stored));
	const [busy, setBusy] = useState(false);
	const next = normalizeColumnMetadata(draft);
	const dirty = !sameColumnMetadata(next, baseline);
	const update = (patch: Partial<ColumnDetailsDraft>) =>
		setDraft((current) => ({ ...current, ...patch }));

	const save = async (event: React.FormEvent) => {
		event.preventDefault();
		if (!dirty || busy) return;
		setBusy(true);
		const ok = await attempt(() => onSave(column, next));
		setBusy(false);
		if (ok) setBaseline(next);
	};

	return (
		<DetailsSection
			status={
				dirty && (
					<span className="text-xs text-muted-foreground">
						{t("unsavedChanges", "Unsaved changes")}
					</span>
				)
			}
		>
			<form noValidate onSubmit={(event) => void save(event)}>
				<div className="flex flex-col gap-5 p-4">
					<LabelledField
						id={`${ids}-display-name`}
						label={t("schemaDisplayName", "Display name")}
						hint={t(
							"schemaDisplayNameHint",
							"Shown in table headers instead of the column name.",
						)}
					>
						<Input
							id={`${ids}-display-name`}
							className="max-w-sm"
							placeholder={column}
							maxLength={MAX_COLUMN_DISPLAY_NAME_CHARS}
							value={draft.display_name}
							disabled={busy}
							aria-describedby={`${ids}-display-name-hint`}
							onChange={(event) => update({ display_name: event.target.value })}
						/>
					</LabelledField>
					<LabelledField
						id={`${ids}-description`}
						label={t("description", "Description")}
					>
						<Textarea
							id={`${ids}-description`}
							rows={3}
							maxLength={MAX_COLUMN_DESCRIPTION_CHARS}
							placeholder={t(
								"schemaColumnDescriptionPlaceholder",
								"What it holds, its unit or format, where it comes from…",
							)}
							value={draft.description}
							disabled={busy}
							onChange={(event) => update({ description: event.target.value })}
						/>
					</LabelledField>
					<DataSensitivityFields
						value={draft}
						disabled={busy}
						onChange={({ classification, pii }) =>
							update({ classification, pii: pii === true })
						}
					/>
					<ReadAttributesField
						id={`${ids}-readers`}
						value={draft.read_attributes}
						onChange={(read_attributes) => update({ read_attributes })}
						knownAttributes={knownAttributes}
						canManage={canManageColumnAccess}
						disabled={busy}
					/>
				</div>
				<div className="flex justify-end gap-2 border-t px-4 py-3">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						disabled={!dirty || busy}
						onClick={() => setDraft(draftOf(baseline))}
					>
						{t("cancel", "Cancel")}
					</Button>
					<Button type="submit" size="sm" disabled={!dirty || busy}>
						{busy && <Loader2 className="animate-spin" />}
						{t("save", "Save")}
					</Button>
				</div>
			</form>
		</DetailsSection>
	);
}

function LabelledField({
	id,
	label,
	hint,
	children,
}: Readonly<{
	id: string;
	label: string;
	hint?: string;
	children: React.ReactNode;
}>) {
	return (
		<div className="flex min-w-0 flex-col gap-1.5">
			<Label htmlFor={id}>{label}</Label>
			{children}
			{hint && (
				<p id={`${id}-hint`} className="text-xs text-muted-foreground">
					{hint}
				</p>
			)}
		</div>
	);
}

function ReadAttributesField({
	id,
	value,
	onChange,
	knownAttributes,
	canManage,
	disabled,
}: Readonly<{
	id: string;
	value: string[];
	onChange: (next: string[]) => void;
	knownAttributes?: readonly string[];
	canManage: boolean;
	disabled: boolean;
}>) {
	const { t } = useTranslation("common");
	const unheld = knownAttributes
		? value.filter((attribute) => !knownAttributes.includes(attribute))
		: [];
	return (
		<div className="flex min-w-0 flex-col gap-1.5">
			<Label htmlFor={id}>
				<Lock className="size-3.5 text-muted-foreground" />
				{t("schemaAllowedToRead", "Allowed to read")}
			</Label>
			<p id={`${id}-hint`} className="text-xs text-muted-foreground">
				{t(
					"schemaAllowedToReadHint",
					"Leave empty and everyone who can read the table sees this column. With role attributes, only admins and roles holding at least one of them see it.",
				)}
			</p>
			<AttributeTagsInput
				id={id}
				aria-describedby={`${id}-hint`}
				value={value}
				onChange={onChange}
				suggestions={knownAttributes}
				suggestionsLabel={t("schemaRoleAttributes", "Role attributes:")}
				placeholder={t("schemaReadAttributePlaceholder", "e.g. hr")}
				maxLength={MAX_COLUMN_ATTRIBUTE_CHARS}
				disabled={disabled || !canManage}
			/>
			{!canManage && (
				<p className="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Lock className="size-3.5 shrink-0" />
					{t(
						"schemaAllowedToReadAdminOnly",
						"Only admins and owners can change who may read a column.",
					)}
				</p>
			)}
			{unheld.length > 0 && (
				<p className="flex items-start gap-1.5 text-xs text-warning">
					<AlertTriangle className="mt-px size-3.5 shrink-0" />
					{t(
						"schemaUnheldReadAttributes",
						"No role holds {{attributes}} yet.",
						{
							attributes: unheld.join(", "),
						},
					)}
				</p>
			)}
		</div>
	);
}

function PropertyRow({
	label,
	hint,
	value,
	action,
	children,
}: Readonly<{
	label: string;
	hint: string;
	value: React.ReactNode;
	action?: React.ReactNode;
	children?: React.ReactNode;
}>) {
	return (
		<div>
			<div className="grid gap-x-5 gap-y-2 px-4 py-3.5 sm:grid-cols-[11rem_minmax(0,1fr)_auto] sm:items-center">
				<div className="space-y-0.5">
					<p className="text-sm font-medium">{label}</p>
					<p className="text-xs text-muted-foreground">{hint}</p>
				</div>
				<div className="min-w-0">{value}</div>
				<div className="flex sm:justify-end">{action}</div>
			</div>
			{children}
		</div>
	);
}

function ConfirmPanel({
	title,
	description,
	confirmLabel,
	busy,
	destructive = false,
	className,
	onCancel,
	onConfirm,
}: Readonly<{
	title: string;
	description: string;
	confirmLabel: string;
	busy: boolean;
	destructive?: boolean;
	className?: string;
	onCancel: () => void;
	onConfirm: () => void;
}>) {
	const { t } = useTranslation("common");
	const cancelRef = useRef<HTMLButtonElement>(null);
	useEffect(() => {
		cancelRef.current?.focus();
	}, []);

	return (
		<div
			className={cn(
				"mx-4 mb-4 flex flex-col gap-3 rounded-md border p-3.5 sm:flex-row sm:items-start",
				destructive
					? "border-destructive/40 bg-destructive/5"
					: "border-primary/30 bg-primary/5",
				className,
			)}
		>
			<AlertTriangle
				className={cn(
					"mt-0.5 size-4 shrink-0",
					destructive ? "text-destructive" : "text-primary",
				)}
			/>
			<div className="min-w-0 flex-1 space-y-1">
				<p className="text-sm font-medium">{title}</p>
				<p className="text-xs leading-relaxed text-muted-foreground">
					{description}
				</p>
			</div>
			<div className="flex shrink-0 gap-2">
				<Button
					ref={cancelRef}
					variant="ghost"
					size="sm"
					disabled={busy}
					onClick={onCancel}
				>
					{t("cancel", "Cancel")}
				</Button>
				<Button
					variant={destructive ? "destructive" : "default"}
					size="sm"
					disabled={busy}
					onClick={onConfirm}
				>
					{busy && <Loader2 className="animate-spin" />}
					{confirmLabel}
				</Button>
			</div>
		</div>
	);
}

function NullabilityValue({ nullable }: Readonly<{ nullable: boolean }>) {
	const { t } = useTranslation("common");
	return (
		<span className="flex flex-wrap items-center gap-2.5">
			<span
				className={cn(
					"inline-flex h-6 items-center rounded-full border px-2.5 text-xs font-medium",
					nullable
						? "border-primary/30 bg-primary/10 text-primary"
						: "bg-muted text-foreground",
				)}
			>
				{nullable ? t("nullable", "Nullable") : t("required", "Required")}
			</span>
			<span className="text-xs text-muted-foreground">
				{nullable
					? t("schemaNullableHint", "Rows may leave it empty")
					: t("schemaRequiredHint", "Every row has a value")}
			</span>
		</span>
	);
}

function KeyBadge() {
	const { t } = useTranslation("common");
	return (
		<Badge variant="secondary" className="shrink-0 gap-1 font-sans">
			<KeyRound className="size-3" />
			{t("tableKeyBadge", "Key")}
		</Badge>
	);
}

function LockNote({ children }: Readonly<{ children: React.ReactNode }>) {
	return (
		<span className="inline-flex items-center gap-1.5 whitespace-nowrap text-xs text-muted-foreground">
			<Lock className="size-3.5" />
			{children}
		</span>
	);
}

function TypeChip({ children }: Readonly<{ children: React.ReactNode }>) {
	return (
		<Badge variant="outline" className="font-mono text-[11px] font-normal">
			{children}
		</Badge>
	);
}

function IndexChip({ children }: Readonly<{ children: React.ReactNode }>) {
	return (
		<span className="inline-flex shrink-0 items-center gap-1 rounded-md border border-primary/30 bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
			<Zap className="size-3" />
			{children}
		</span>
	);
}

const ADD_COLUMN_TYPES = [
	...EDIT_COLUMN_TYPE_GROUPS.flatMap((group) => group.options),
	{ value: "vector", label: "Vector" },
];

const DEFAULT_PLACEHOLDERS: Record<string, string> = {
	string: "unknown",
	int32: "0",
	int64: "0",
	float32: "0.5",
	float64: "0.5",
	date32: "2026-01-31",
	timestamp: "2026-01-31 14:30:00",
};

function NewColumnPane({
	fields,
	rowCount,
	onAddColumn,
	onAdded,
	onCancel,
}: Readonly<{
	fields: LanceField[];
	rowCount?: number;
	onAddColumn: (column: IAddColumnPayload) => Promise<void>;
	onAdded: (name: string) => void;
	onCancel: () => void;
}>) {
	const { t } = useTranslation("common");
	const ids = useId();
	const nameRef = useRef<HTMLInputElement>(null);
	const [name, setName] = useState("");
	const [type, setType] = useState("string");
	const [fill, setFill] = useState("null");
	const [value, setValue] = useState("");
	const [dimensions, setDimensions] = useState("");
	const [submitted, setSubmitted] = useState(false);
	const [busy, setBusy] = useState(false);

	useEffect(() => {
		nameRef.current?.focus();
	}, []);

	const trimmedName = name.trim();
	const isBoolean = type === "boolean";
	const isVector = type === "vector";
	const fillValue =
		fill === "value" ? (isBoolean ? value || "true" : value) : "";
	const nameError =
		validateColumnName(name) ??
		(fields.some((field) => field.name === trimmedName)
			? t("schemaColumnExists", 'A column named "{{name}}" already exists', {
					name: trimmedName,
				})
			: null);
	const valueError = isVector
		? null
		: fill === "value" && !fillValue.trim()
			? t("schemaEnterAValue", "Enter a value")
			: validateColumnDefault(type, fillValue);
	const expression = buildAddColumnExpression(type, fillValue);
	const dimensionError =
		isVector &&
		(!/^\d+$/.test(dimensions) ||
			Number(dimensions) < 1 ||
			Number(dimensions) > 2147483647)
			? t(
					"schemaVectorDimensionsInvalid",
					"Enter a whole number between 1 and 2147483647.",
				)
			: null;
	const showNameError = Boolean(nameError) && (submitted || name.length > 0);
	const showValueError =
		Boolean(valueError) && (submitted || value.trim().length > 0);
	const showDimensionError =
		Boolean(dimensionError) && (submitted || dimensions.length > 0);
	const fillLegend = rowCount
		? t("schemaValueForExistingRows", {
				count: rowCount,
				defaultValue_one: "Value for the {{count}} existing row",
				defaultValue_other: "Value for the {{count}} existing rows",
			})
		: t("schemaValueForExistingRowsUnknown", "Value for existing rows");
	const defaultLabel = t("schemaDefaultValue", "Default value");

	const submit = async (event: React.FormEvent) => {
		event.preventDefault();
		setSubmitted(true);
		if (busy || nameError || valueError || dimensionError) return;
		let column: IAddColumnPayload;
		if (isVector) {
			column = {
				name: trimmedName,
				type: "vector",
				vector_size: Number(dimensions),
			};
		} else {
			if (!expression) return;
			column = { name: trimmedName, sql_expression: expression };
		}
		setBusy(true);
		const ok = await attempt(() => onAddColumn(column));
		setBusy(false);
		if (ok) onAdded(trimmedName);
	};

	return (
		<form
			noValidate
			onSubmit={(event) => void submit(event)}
			className="flex min-h-full flex-col"
		>
			<div className="flex flex-1 flex-col gap-6 p-6">
				<div className="space-y-1">
					<h3 className="text-lg font-semibold">
						{t("newColumn", "New column")}
					</h3>
					<p className="text-sm text-muted-foreground">
						{isVector
							? t(
									"schemaNewVectorColumnDescription",
									"Adds an empty vector column with the dimensions your embedding model requires.",
								)
							: t(
									"schemaNewColumnDescription",
									"Added as a nullable column. Existing rows get the value you choose below.",
								)}
					</p>
				</div>

				<div className="space-y-2">
					<Label htmlFor={`${ids}-name`}>{t("schemaColumnName", "Name")}</Label>
					<Input
						ref={nameRef}
						id={`${ids}-name`}
						className="max-w-sm font-mono"
						placeholder="column_name"
						autoComplete="off"
						spellCheck={false}
						value={name}
						onChange={(event) => setName(event.target.value)}
						aria-invalid={showNameError}
						aria-describedby={`${ids}-name-hint`}
					/>
					<p
						id={`${ids}-name-hint`}
						className={cn(
							"text-xs",
							showNameError ? "text-destructive" : "text-muted-foreground",
						)}
					>
						{showNameError
							? nameError
							: t(
									"schemaColumnNameHint",
									"Letters, numbers and underscores. It can't start with a number.",
								)}
					</p>
				</div>

				<fieldset className="space-y-2">
					<legend className="mb-2 text-sm font-medium">
						{t("schemaTypeLabel", "Type")}
					</legend>
					<RadioGroupPrimitive.Root
						aria-label={t("schemaTypeLabel", "Type")}
						value={type}
						onValueChange={(next) => {
							setType(next);
							setValue("");
						}}
						className="grid grid-cols-2 gap-2 sm:grid-cols-3"
					>
						{ADD_COLUMN_TYPES.map((option) => (
							<RadioGroupPrimitive.Item
								key={option.value}
								value={option.value}
								className="flex h-9 items-center justify-between gap-2 rounded-md border bg-background px-3 text-left outline-none transition-colors hover:border-foreground/30 focus-visible:ring-[3px] focus-visible:ring-ring/50 data-[state=checked]:border-primary data-[state=checked]:bg-primary/10"
							>
								<span className="text-sm font-medium">{option.label}</span>
								<span className="font-mono text-[11px] text-muted-foreground">
									{option.value === "vector"
										? "Float32"
										: addColumnSqlType(option.value)}
								</span>
							</RadioGroupPrimitive.Item>
						))}
					</RadioGroupPrimitive.Root>
				</fieldset>

				{isVector ? (
					<div className="space-y-2">
						<Label htmlFor={`${ids}-dimensions`}>
							{t("schemaVectorDimensions", "Dimensions")}
						</Label>
						<Input
							id={`${ids}-dimensions`}
							type="number"
							min={1}
							max={2147483647}
							step={1}
							className="max-w-xs font-mono"
							placeholder="1536"
							value={dimensions}
							onChange={(event) => setDimensions(event.target.value)}
							aria-invalid={showDimensionError}
							aria-describedby={`${ids}-dimensions-hint`}
						/>
						<p
							id={`${ids}-dimensions-hint`}
							className={cn(
								"text-xs",
								showDimensionError
									? "text-destructive"
									: "text-muted-foreground",
							)}
						>
							{showDimensionError
								? dimensionError
								: t(
										"schemaVectorDimensionsHint",
										"Use the dimension count from your embedding model. Existing rows start with NULL; generate new embeddings before searching this column.",
									)}
						</p>
					</div>
				) : (
					<fieldset className="space-y-2">
						<legend className="mb-2 text-sm font-medium">{fillLegend}</legend>
						<div className="flex flex-wrap items-center gap-3">
							<Segmented
								label={fillLegend}
								value={fill}
								onValueChange={setFill}
								options={[
									{ value: "null", label: "NULL" },
									{
										value: "value",
										label: t("schemaFillWithValue", "A value"),
									},
								]}
							/>
							{fill === "value" && isBoolean && (
								<Segmented
									label={defaultLabel}
									value={fillValue}
									onValueChange={setValue}
									options={[
										{ value: "true", label: "true" },
										{ value: "false", label: "false" },
									]}
								/>
							)}
							{fill === "value" && !isBoolean && (
								<Input
									className="max-w-xs font-mono"
									aria-label={defaultLabel}
									aria-invalid={showValueError}
									placeholder={DEFAULT_PLACEHOLDERS[type]}
									value={value}
									onChange={(event) => setValue(event.target.value)}
								/>
							)}
						</div>
						{showValueError && (
							<p className="text-xs text-destructive">{valueError}</p>
						)}
					</fieldset>
				)}
			</div>

			<div className="sticky bottom-0 flex flex-wrap items-center gap-3 border-t bg-background px-6 py-3">
				<span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
					{isVector ? t("schemaTypeLabel", "Type") : "SQL"}
				</span>
				<code className="min-w-0 flex-1 break-all font-mono text-xs">
					{isVector
						? `vector${!dimensionError ? `(${dimensions})` : ""}`
						: expression}
				</code>
				<div className="flex gap-2">
					<Button
						type="button"
						variant="ghost"
						disabled={busy}
						onClick={onCancel}
					>
						{t("cancel", "Cancel")}
					</Button>
					<Button type="submit" disabled={busy}>
						{busy && <Loader2 className="animate-spin" />}
						{t("addColumn", "Add Column")}
					</Button>
				</div>
			</div>
		</form>
	);
}

function Segmented({
	label,
	value,
	options,
	onValueChange,
}: Readonly<{
	label: string;
	value: string;
	options: readonly { value: string; label: string }[];
	onValueChange: (value: string) => void;
}>) {
	return (
		<RadioGroupPrimitive.Root
			aria-label={label}
			orientation="horizontal"
			value={value}
			onValueChange={onValueChange}
			className="inline-flex rounded-md border bg-muted/40 p-0.5"
		>
			{options.map((option) => (
				<RadioGroupPrimitive.Item
					key={option.value}
					value={option.value}
					className="h-7 rounded-[5px] px-3 text-xs font-medium text-muted-foreground outline-none transition-colors focus-visible:ring-[3px] focus-visible:ring-ring/50 data-[state=checked]:bg-background data-[state=checked]:text-foreground"
				>
					{option.label}
				</RadioGroupPrimitive.Item>
			))}
		</RadioGroupPrimitive.Root>
	);
}

function IndicesPane({
	fields,
	indices,
	status,
	canBuild,
	onRetry,
	onOpenColumn,
	onDropIndex,
}: Readonly<{
	fields: LanceField[];
	indices: IIndexConfig[];
	status: IndexStatus;
	canBuild: boolean;
	onRetry: () => Promise<void>;
	onOpenColumn: (name: string) => void;
	onDropIndex?: (indexName: string) => Promise<void>;
}>) {
	const { t } = useTranslation("common");
	const titleId = useId();
	const [dropping, setDropping] = useState<string | null>(null);
	const nested = indices.filter(
		(index) =>
			!index.columns.some((column) =>
				fields.some((field) => field.name === column),
			),
	);

	const drop = async (name: string) => {
		if (!onDropIndex) return;
		setDropping(name);
		if (await attempt(() => onDropIndex(name))) await onRetry();
		setDropping(null);
	};

	const indexCell = (index?: IIndexConfig) => {
		if (index) {
			return (
				<span className="flex min-w-0 items-center gap-2">
					<IndexChip>{indexLabel(index)}</IndexChip>
					<span className="truncate font-mono text-xs text-muted-foreground">
						{index.name}
					</span>
				</span>
			);
		}
		return (
			<span className="text-xs text-muted-foreground">
				{status === "ready" ? t("none", "None") : t("loading", "Loading…")}
			</span>
		);
	};

	const actionCell = (field: string, index?: IIndexConfig) => {
		if (index && onDropIndex) {
			return (
				<Button
					variant="ghost"
					size="sm"
					className="text-destructive hover:text-destructive"
					disabled={dropping !== null}
					aria-label={t("schemaDropIndexNamed", "Drop index {{name}}", {
						name: index.name,
					})}
					onClick={() => void drop(index.name)}
				>
					{dropping === index.name && <Loader2 className="animate-spin" />}
					{t("dropIndex", "Drop index")}
				</Button>
			);
		}
		if (index || !canBuild || status !== "ready") return null;
		return (
			<Button variant="ghost" size="sm" onClick={() => onOpenColumn(field)}>
				{t("schemaAddIndex", "Add index")}
			</Button>
		);
	};

	return (
		<section aria-labelledby={titleId} className="flex flex-col gap-5 p-6">
			<div className="space-y-1">
				<h3 id={titleId} className="text-lg font-semibold">
					{t("indices", "Indices")}
				</h3>
				<p className="text-sm text-muted-foreground">
					{t(
						"schemaIndicesDescription",
						"Indices speed up filters, text search and similarity queries. Building one reads every row.",
					)}
				</p>
			</div>
			{status === "error" ? (
				<div className="flex items-center gap-3 rounded-lg border border-destructive/40 p-4 text-sm">
					<AlertTriangle className="size-4 shrink-0 text-destructive" />
					<span className="flex-1">
						{t("schemaIndicesFailed", "Couldn't load the indices.")}
					</span>
					<Button variant="outline" size="sm" onClick={() => void onRetry()}>
						{t("retry", "Retry")}
					</Button>
				</div>
			) : (
				<div className="rounded-lg border">
					{/* Resets the base-layer table rules in global.css (cell borders, margins). */}
					<Table className="my-0">
						<TableHeader className="bg-muted/30">
							<TableRow className="hover:bg-transparent">
								<TableHead className="border-0 bg-transparent px-4">
									{t("column", "Column")}
								</TableHead>
								<TableHead className="border-0 bg-transparent px-4">
									{t("schemaTypeLabel", "Type")}
								</TableHead>
								<TableHead className="border-0 bg-transparent px-4">
									{t("schemaIndexLabel", "Index")}
								</TableHead>
								<TableHead className="border-0 bg-transparent px-4">
									<span className="sr-only">{t("actions", "Actions")}</span>
								</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{fields.map((field) => {
								const index = columnIndex(indices, field.name);
								return (
									<TableRow key={field.name}>
										<TableCell className="border-0 bg-transparent px-4">
											<button
												type="button"
												className="rounded-sm font-mono text-[13px] underline decoration-muted-foreground/40 underline-offset-4 outline-none hover:decoration-primary focus-visible:ring-[3px] focus-visible:ring-ring/50"
												onClick={() => onOpenColumn(field.name)}
											>
												{field.name}
											</button>
										</TableCell>
										<TableCell className="border-0 bg-transparent px-4">
											<TypeChip>{fieldTypeLabel(field)}</TypeChip>
										</TableCell>
										<TableCell className="border-0 bg-transparent max-w-64 px-4">
											{indexCell(index)}
										</TableCell>
										<TableCell className="border-0 bg-transparent px-4 text-right">
											{actionCell(field.name, index)}
										</TableCell>
									</TableRow>
								);
							})}
							{nested.map((index) => (
								<TableRow key={index.name}>
									<TableCell className="border-0 bg-transparent px-4 font-mono text-[13px]">
										{index.columns.join(", ")}
									</TableCell>
									<TableCell className="border-0 bg-transparent px-4" />
									<TableCell className="border-0 bg-transparent max-w-64 px-4">
										{indexCell(index)}
									</TableCell>
									<TableCell className="border-0 bg-transparent px-4 text-right">
										{actionCell(index.columns[0] ?? "", index)}
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</div>
			)}
		</section>
	);
}
