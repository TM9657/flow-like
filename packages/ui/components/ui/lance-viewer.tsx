import {
  type ColumnDef,
  type ColumnFiltersState,
  type ColumnOrderState,
  type SortingState,
  type VisibilityState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import {
  Columns3,
  Database,
  Download,
  GripVertical,
  Info,
  ListTree,
  Maximize2,
  Minimize2,
  MoreHorizontal,
  Save,
  Search,
  X
} from "lucide-react";
import * as React from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import Dexie, { type Table } from "dexie";
import { cn } from "../../lib";
import {
  Badge,
  Button,
  Checkbox,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Input,
  ScrollArea,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Separator,
  Textarea,
  Switch,
  Label,
  TextEditor,
} from "./";
import {
  Table as DataTable,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "./table";

export type LanceFieldKind =
  | "string"
  | "number"
  | "boolean"
  | "date"
  | "vector"
  | "array"
  | "object"
  | "unknown";

export interface LanceField {
  name: string;
  kind: LanceFieldKind;
  dims?: number;
  items?: LanceFieldKind | LanceField;
}

export interface LanceSchema {
  table: string;
  fields: LanceField[];
  primaryKey?: string;
}

export interface ArrowSchemaJSON {
  fields: any[];
  metadata?: Record<string, unknown>;
}

export interface LanceDBExplorerProps {
  arrowSchema?: ArrowSchemaJSON;
  onSwitchPage: (
    offset: number,
    limit: number,
  ) => Promise<Record<string, any>[] | { data: Record<string, any>[]; total?: number }>;
  total?: number;
  pageSizeOptions?: readonly number[];
  initialMode?: "table" | "vector";
  onSearch?: (args: {
    query: string;
    mode: "table" | "vector";
    vector?: number[];
    topK?: number;
    where?: Record<string, unknown>;
  }) => Promise<void> | void;
  className?: string;
  tableName?: string;
  children?: React.ReactNode;
}

interface TableSettings {
  id: string;
  tableName: string;
  columnVisibility: VisibilityState;
  columnOrder: string[];
  sorting: SortingState;
  pageSize: number;
}

class SettingsDB extends Dexie {
  tableSettings!: Table<TableSettings>;

  constructor() {
    super("LanceDBExplorerSettings");
    this.version(1).stores({
      tableSettings: "id, tableName"
    });
  }
}

const db = new SettingsDB();
const DEFAULT_PAGE_SIZE = 50;

const LanceDBExplorer: React.FC<LanceDBExplorerProps> = ({
  tableName = "table",
  children,
  arrowSchema,
  onSwitchPage,
  total: totalProp,
  pageSizeOptions = [25, 50, 100, 250],
  initialMode = "table",
  onSearch,
  className,
}) => {
  const [schema, setSchema] = useState<LanceSchema | null>(null);
  const [data, setData] = useState<Record<string, any>[]>([]);
  const [total, setTotal] = useState<number>(totalProp ?? 0);

  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState<number>(pageSizeOptions?.[0] ?? DEFAULT_PAGE_SIZE);

  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [columnOrder, setColumnOrder] = useState<ColumnOrderState>([]);
  const [sorting, setSorting] = useState<SortingState>([]);
  const [rowSelection, setRowSelection] = useState({});
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [globalQuery, setGlobalQuery] = useState("");

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [lastCount, setLastCount] = useState(0);
  const [fullscreen, setFullscreen] = useState(false);

  const settingsId = `${tableName}_settings`;

  const loadSettings = useCallback(async () => {
    try {
      const settings = await db.tableSettings.get(settingsId);
      if (settings) {
        setColumnVisibility(settings.columnVisibility);
        setColumnOrder(settings.columnOrder);
        setSorting(settings.sorting);
        setPageSize(settings.pageSize);
      }
    } catch (error) {
      console.warn("Failed to load settings:", error);
    }
  }, [settingsId]);

  const saveSettings = useCallback(async () => {
    try {
      await db.tableSettings.put({
        id: settingsId,
        tableName,
        columnVisibility,
        columnOrder,
        sorting,
        pageSize,
      });
    } catch (error) {
      console.warn("Failed to save settings:", error);
    }
  }, [settingsId, tableName, columnVisibility, columnOrder, sorting, pageSize]);

  useEffect(() => {
    if (arrowSchema?.fields?.length) {
      setSchema(arrowToLanceSchema(arrowSchema));
    } else {
      setSchema(null);
    }
    loadSettings();
  }, [arrowSchema, loadSettings]);

  useEffect(() => {
    saveSettings();
  }, [saveSettings]);

  useEffect(() => {
    if (typeof totalProp === "number") setTotal(totalProp);
  }, [totalProp]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await onSwitchPage((page - 1) * pageSize, pageSize);
      if (Array.isArray(res)) {
        setData(res);
        setLastCount(res.length);
        setTotal((prev) => {
          const count = res.length;
          const pageEnd = (page - 1) * pageSize + count;
          return count < pageSize ? pageEnd : Math.max(prev, 0);
        });
        if (res.length) {
          setSchema((prev) => prev ?? inferSchemaFromRows(res, tableName));
        }
      } else {
        const rows = res.data ?? [];
        setData(rows);
        setLastCount(rows.length);
        if (typeof res.total === "number") setTotal(res.total);
        if (rows.length) {
          setSchema((prev) => prev ?? inferSchemaFromRows(rows, tableName));
        }
      }
    } catch (e: any) {
      setError(e?.message ?? "Failed to load data");
    } finally {
      setLoading(false);
    }
  }, [onSwitchPage, page, pageSize, tableName]);

  useEffect(() => {
    load();
  }, [load]);

  const columns = useMemo<ColumnDef<Record<string, any>>[]>(() => {
    if (!schema) return [];
    const base: ColumnDef<Record<string, any>>[] = schema.fields.map((f) => buildColumnForField(f));

    base.unshift({
      id: "select",
      header: ({ table }) => (
        <Checkbox
          checked={table.getIsAllPageRowsSelected()}
          onCheckedChange={(v) => table.toggleAllPageRowsSelected(!!v)}
          aria-label="Select all"
        />
      ),
      cell: ({ row }) => (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(v) => row.toggleSelected(!!v)}
          aria-label="Select row"
        />
      ),
      enableSorting: false,
      enableHiding: false,
      size: 28,
    });

    return base;
  }, [schema]);

  const table = useReactTable({
    data,
    columns,
    state: {
      columnVisibility,
      columnOrder,
      sorting,
      rowSelection,
      columnFilters,
      globalFilter: globalQuery,
    },
    onColumnVisibilityChange: setColumnVisibility,
    onColumnOrderChange: setColumnOrder,
    onSortingChange: setSorting,
    onRowSelectionChange: setRowSelection,
    onColumnFiltersChange: setColumnFilters,
    onGlobalFilterChange: setGlobalQuery,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    manualPagination: true,
    pageCount: Math.ceil((total || 0) / pageSize) || -1,
    defaultColumn: { minSize: 80, maxSize: 600 },
    enableColumnResizing: true,
    columnResizeMode: "onChange",
  });

  const copySelectedAsCSV = useCallback(() => {
    const rows = table.getSelectedRowModel().rows;
    const cols = table
      .getAllLeafColumns()
      .filter((c) => c.id !== "select")
      .map((c) => c.id);
    const csv = [cols.join(",")].concat(
      rows.map((r) => cols.map((c) => stringifyCSV(r.getValue(c))).join(",")),
    );
    navigator.clipboard.writeText(csv.join("\n"));
  }, [table]);

  const currentFrom = (page - 1) * pageSize + 1;
  const currentTo = Math.min(page * pageSize, total || 0);
  const knowsTotal = typeof total === "number" && total > 0;
  const isLastPage = knowsTotal ? currentTo >= total : lastCount < pageSize;

  const containerCls = cn(
    "flex h-full w-full flex-col gap-3",
    fullscreen && "fixed inset-0 z-[60] bg-background p-4",
    className,
  );

  return (
    <div className={containerCls}>
      <div className="flex items-center gap-2 flex-shrink-0">
        <Database className="h-5 w-5" />
        <div className="text-sm text-muted-foreground">{tableName}</div>
        <Separator orientation="vertical" className="mx-1" />
        <div className="ml-auto flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setFullscreen((v) => !v)}
            title={fullscreen ? "Exit fullscreen" : "Fullscreen"}
          >
            {fullscreen ? (
              <>
                <Minimize2 className="h-4 w-4 mr-2" /> Exit
              </>
            ) : (
              <>
                <Maximize2 className="h-4 w-4 mr-2" /> Fullscreen
              </>
            )}
          </Button>
          <SchemaDialog schema={schema} tableName={tableName} />
          <ColumnVisibilityDropdown
            columns={table.getAllLeafColumns().map((c: any) => ({
              id: c.id,
              canHide: c.getCanHide(),
            }))}
            visibility={columnVisibility}
            onChange={setColumnVisibility}
          />
          {children}
        </div>
      </div>

      <Toolbar
        value={globalQuery}
        onValueChange={setGlobalQuery}
        onSearch={() => setPage(1)}
        onReset={() => {
          setGlobalQuery("");
          setPage(1);
        }}
      />

      <div className="flex flex-col flex-1 min-h-0 rounded-xl border bg-card">
        <div className="flex-1 w-full overflow-auto">
          <DataTable className="w-full">
            <TableHeader className="sticky top-0 bg-card z-10">
              {table.getHeaderGroups().map((headerGroup) => (
                <TableRow key={headerGroup.id}>
                  {headerGroup.headers.map((header) => (
                    <DraggableTableHead
                      key={header.id}
                      header={header}
                      table={table}
                    />
                  ))}
                </TableRow>
              ))}
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={columns.length} className="h-24 text-center text-muted-foreground">
                    Loading…
                  </TableCell>
                </TableRow>
              ) : table.getRowModel().rows?.length ? (
                table.getRowModel().rows.map((row) => (
                  <TableRow key={row.id} className="hover:bg-muted/30">
                    {row.getVisibleCells().map((cell) => (
                      <TableCell key={cell.id}>
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </TableCell>
                    ))}
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell colSpan={columns.length} className="h-24 text-center text-muted-foreground">
                    No results.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </DataTable>
        </div>

        <div className="flex items-center justify-between px-3 py-2 border-t bg-muted/20 flex-shrink-0">
          <div className="text-xs text-muted-foreground">
            {knowsTotal ? (
              <>
                Showing <b>{currentFrom}</b>–<b>{currentTo}</b> of{" "}
                <b>{total.toLocaleString()}</b>
              </>
            ) : (
              <>—</>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Select
              value={String(pageSize)}
              onValueChange={(v) => {
                setPageSize(Number(v));
                setPage(1);
              }}
            >
              <SelectTrigger className="h-8 w-[120px]">
                <SelectValue placeholder="Page size" />
              </SelectTrigger>
              <SelectContent>
                {pageSizeOptions.map((n) => (
                  <SelectItem key={n} value={String(n)}>
                    {n} / page
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page === 1}
            >
              Prev
            </Button>
            <div className="text-sm w-14 text-center">{page}</div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage((p) => p + 1)}
              disabled={isLastPage}
            >
              Next
            </Button>

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm">
                  <MoreHorizontal className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={copySelectedAsCSV}>
                  <Download className="h-4 w-4 mr-2" /> Copy selected as CSV
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>

      {error && (
        <div className="text-sm text-destructive flex items-center gap-2 flex-shrink-0">
          <Info className="h-4 w-4" /> {error}
        </div>
      )}
    </div>
  );
};

export default LanceDBExplorer;

const DraggableTableHead: React.FC<{ header: any; table: any }> = ({ header, table }) => {
  // Skip rendering for hidden columns to keep header in sync with body
  if (!header.column.getIsVisible()) return null;

  const { getState, setColumnOrder } = table;
  const { columnOrder } = getState();

  const [isDragging, setIsDragging] = useState(false);
  const [draggedOver, setDraggedOver] = useState<string | null>(null);

  const isDraggable = header.id !== "select";

  const columnOrderIds = useMemo(() => [
    ...columnOrder,
    ...table.getAllLeafColumns().map((d: any) => d.id).filter((id: string) => !columnOrder.includes(id)),
  ], [columnOrder, table]);

  const reorderColumn = useCallback((draggedColumnId: string, targetColumnId: string, position: "left" | "right") => {
    setColumnOrder(() => {
      const newColumnOrder = [...columnOrderIds];
      const draggedIndex = newColumnOrder.indexOf(draggedColumnId);
      const targetIndex = newColumnOrder.indexOf(targetColumnId);

      if (draggedIndex === -1 || targetIndex === -1) return newColumnOrder;

      newColumnOrder.splice(draggedIndex, 1);

      const insertIndex = position === "left" ? targetIndex : targetIndex + 1;
      newColumnOrder.splice(insertIndex, 0, draggedColumnId);

      return newColumnOrder;
    });
  }, [columnOrderIds, setColumnOrder]);

  const handleDragStart = useCallback((e: React.DragEvent) => {
    if (!isDraggable) return;
    setIsDragging(true);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", header.id);

    if (e.dataTransfer.setDragImage) {
      const dragImage = document.createElement("div");
      dragImage.textContent = header.id;
      dragImage.style.cssText = "position: absolute; top: -1000px; background: white; padding: 4px 8px; border: 1px solid #ccc; border-radius: 4px;";
      document.body.appendChild(dragImage);
      e.dataTransfer.setDragImage(dragImage, 0, 0);
      setTimeout(() => document.body.removeChild(dragImage), 0);
    }
  }, [isDraggable, header.id]);

  const handleDragEnd = useCallback(() => {
    setIsDragging(false);
    setDraggedOver(null);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    if (!isDraggable) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    setDraggedOver(header.id);
  }, [isDraggable, header.id]);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node)) {
      setDraggedOver(null);
    }
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const draggedColumnId = e.dataTransfer.getData("text/plain");
    const targetColumnId = header.id;

    if (draggedColumnId && draggedColumnId !== targetColumnId && isDraggable) {
      const rect = e.currentTarget.getBoundingClientRect();
      const position = e.clientX < rect.left + rect.width / 2 ? "left" : "right";
      reorderColumn(draggedColumnId, targetColumnId, position);
    }
    setDraggedOver(null);
  }, [header.id, isDraggable, reorderColumn]);

  return (
    <TableHead
      style={{ width: header.getSize() }}
      className={cn(
        "relative group border-b bg-muted/30 select-none",
        isDragging && "opacity-50",
        draggedOver === header.id && "bg-primary/10",
        isDraggable && "cursor-move"
      )}
      draggable={isDraggable}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="flex items-center gap-1">
        {isDraggable && (
          <GripVertical className="h-3 w-3 text-muted-foreground opacity-0 group-hover:opacity-100" />
        )}
        {header.isPlaceholder ? null : (
          <div
            className={cn(
              "flex select-none items-center gap-1 flex-1",
              header.column.getCanSort() && "cursor-pointer",
            )}
            onClick={header.column.getToggleSortingHandler()}
          >
            {flexRender(header.column.columnDef.header, header.getContext())}
            {{ asc: "↑", desc: "↓" }[header.column.getIsSorted() as string] ?? null}
          </div>
        )}
      </div>
    </TableHead>
  );
};

const ColumnVisibilityDropdown: React.FC<{
  columns: { id: string; canHide: boolean }[];
  visibility: VisibilityState;
  onChange: React.Dispatch<React.SetStateAction<VisibilityState>>;
}> = ({ columns, visibility, onChange }) => {
  const isVisible = useCallback(
    (id: string) => (visibility[id] ?? true) === true,
    [visibility]
  );

  const setVisible = useCallback(
    (id: string, v: boolean) => {
      onChange((prev) => ({ ...prev, [id]: v }));
    },
    [onChange]
  );

  const showAll = useCallback(() => {
    onChange((prev) => {
      const next = { ...prev };
      columns.forEach((c) => {
        if (c.canHide) next[c.id] = true;
      });
      return next;
    });
  }, [columns, onChange]);

  const hideAll = useCallback(() => {
    onChange((prev) => {
      const next = { ...prev };
      columns.forEach((c) => {
        if (c.canHide) next[c.id] = false;
      });
      return next;
    });
  }, [columns, onChange]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm">
          <Columns3 className="h-4 w-4 mr-2" /> Columns
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>Toggle columns</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={(e) => e.preventDefault()} onClick={showAll}>
          Show all
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={(e) => e.preventDefault()} onClick={hideAll}>
          Hide all
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        {columns
          .filter((c) => c.canHide)
          .map((c) => (
            <DropdownMenuCheckboxItem
              key={c.id}
              className="capitalize"
              checked={isVisible(c.id)}
              onCheckedChange={(v) => setVisible(c.id, !!v)}
              onSelect={(e) => e.preventDefault()}
            >
              {c.id}
            </DropdownMenuCheckboxItem>
          ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

const buildColumnForField = (f: LanceField): ColumnDef<Record<string, any>> => ({
  id: f.name,
  accessorFn: (row: Record<string, any>) => row[f.name],
  header: f.name,
  enableSorting: true,
  enableColumnFilter: true,
  cell: ({ getValue }) => <Cell value={getValue()} field={f} />,
});

const Cell: React.FC<{ value: any; field: LanceField }> = ({ value, field }) => {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editValue, setEditValue] = useState("");

  const openDialog = useCallback(() => {
    setEditValue(safeStringify(value, 2));
    setDialogOpen(true);
  }, [value]);

  const renderCellPreviewButton = () => {
    if (value == null) {
      return (
        <Button variant="ghost" size="sm" className="h-6 px-2 text-muted-foreground" onClick={openDialog}>
          NULL
        </Button>
      );
    }

    switch (field.kind) {
      case "boolean":
        return (
          <Button variant="ghost" size="sm" className="h-6 px-2" onClick={openDialog}>
            <Checkbox checked={!!value} disabled aria-label="bool" />
          </Button>
        );
      case "number":
        return (
          <Button variant="ghost" size="sm" className="h-6 px-2 tabular-nums justify-start" onClick={openDialog}>
            {value}
          </Button>
        );
      case "date":
        return (
          <Button variant="ghost" size="sm" className="h-6 px-2 tabular-nums justify-start" onClick={openDialog}>
            {formatDate(value)}
          </Button>
        );
      case "vector": {
        const arr = ensureNumericArray(value);
        const dims = field.dims ?? arr.length;
        return (
          <Button variant="ghost" size="sm" className="h-6 px-2 flex items-center gap-2" onClick={openDialog}>
            <Badge variant="outline">{dims}d</Badge>
            <Sparkline data={arr.slice(0, 64)} />
          </Button>
        );
      }
      case "array":
      case "object":
      case "unknown":
        return (
          <Button variant="ghost" size="sm" className="h-6 px-2" onClick={openDialog}>
            <ListTree className="h-3 w-3 mr-1" /> View
          </Button>
        );
      default:
        return (
          <Button variant="ghost" size="sm" className="h-6 px-2 justify-start max-w-[200px] truncate" onClick={openDialog}>
            {String(value)}
          </Button>
        );
    }
  };

  return (
    <>
      {renderCellPreviewButton()}
      <CellViewDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        value={value}
        valueStr={editValue}
        onValueChange={setEditValue}
        field={field}
      />
    </>
  );
};

const CellViewDialog: React.FC<{
  open: boolean;
  onOpenChange: (open: boolean) => void;
  value: any;
  valueStr: string;
  onValueChange: (value: string) => void;
  field: LanceField;
}> = ({ open, onOpenChange, value, valueStr, onValueChange, field }) => {
  const [localValue, setLocalValue] = useState(valueStr);
  const [isEditing, setIsEditing] = useState(false);

  useEffect(() => {
    setLocalValue(valueStr);
  }, [valueStr]);

  useEffect(() => {
    if (!open) {
      setIsEditing(false);
    }
  }, [open]);

  const handleSave = useCallback(() => {
    onValueChange(localValue);
    setIsEditing(false);
    onOpenChange(false);
  }, [localValue, onValueChange, onOpenChange]);

  const PreviewContent = useMemo(() => {
    if (value == null) {
      return <div className="text-sm text-muted-foreground">NULL</div>;
    }

    switch (field.kind) {
      case "boolean":
        return (
          <div className="flex items-center gap-2">
            <Checkbox checked={!!value} disabled aria-label="bool" />
            <span className="text-sm text-muted-foreground">{String(!!value)}</span>
          </div>
        );
      case "number":
        return <code className="text-sm">{String(value)}</code>;
      case "date":
        return <code className="text-sm">{formatDate(value)}</code>;
      case "vector": {
        const arr = ensureNumericArray(value);
        const dims = field.dims ?? arr.length;
        return (
          <div className="flex items-center gap-3">
            <Badge variant="outline">{dims}d</Badge>
            <Sparkline data={arr.slice(0, 128)} />
          </div>
        );
      }
      case "array":
      case "object":
      case "unknown":
        return (
          <pre className="whitespace-pre-wrap text-xs font-mono bg-muted rounded-md p-3">
            {safeStringify(value, 2)}
          </pre>
        );
      default: {
        if (typeof value === "string") {
          return <TextEditor
                      initialContent={value}
                      isMarkdown={true}
                    />;
        }
        return (
          <pre className="whitespace-pre-wrap text-sm bg-muted/50 rounded-md p-3">
            {String(value)}
          </pre>
        );
      }
    }
  }, [field.kind, field.dims, value]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-7xl w-full min-w-fit">
        <DialogHeader>
          <DialogTitle className="flex items-center justify-between">
            <span>{isEditing ? `Edit ${field.name}` : `Preview ${field.name}`}</span>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2">
                <Label htmlFor="edit-switch" className="text-xs text-muted-foreground">
                  Edit
                </Label>
                <Switch
                  id="edit-switch"
                  checked={isEditing}
                  onCheckedChange={setIsEditing}
                />
              </div>
              <Badge variant="outline">{field.kind}</Badge>
            </div>
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          {!isEditing ? (
            <div className="max-h-[60vh] overflow-auto pr-1">{PreviewContent}</div>
          ) : (
            <Textarea
              value={localValue}
              onChange={(e) => setLocalValue(e.target.value)}
              className="min-h-[300px] font-mono text-sm"
              placeholder="Enter value..."
            />
          )}
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              <X className="h-4 w-4 mr-2" /> Close
            </Button>
            {isEditing && (
              <Button onClick={handleSave}>
                <Save className="h-4 w-4 mr-2" /> Save
              </Button>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
};

const Sparkline: React.FC<{ data: number[] }> = ({ data }) => {
  const ref = React.useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const w = 80, h = 20;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);
    ctx.lineWidth = 1;
    ctx.beginPath();

    if (data.length === 0) return;

    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;

    data.forEach((v, i) => {
      const x = (i / Math.max(1, data.length - 1)) * (w - 2) + 1;
      const y = h - 1 - ((v - min) / range) * (h - 2);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });

    ctx.strokeStyle = getComputedStyle(document.documentElement).getPropertyValue("--primary");
    ctx.stroke();
  }, [data]);

  return <canvas ref={ref} aria-label="sparkline" />;
};

const Toolbar: React.FC<{
  value: string;
  onValueChange: (v: string) => void;
  onSearch: () => void;
  onReset: () => void;
}> = ({ value, onValueChange, onSearch, onReset }) => (
  <div className="flex flex-wrap items-center gap-2 flex-shrink-0">
    <div className="relative w-full sm:w-[380px]">
      <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
      <Input
        className="pl-8"
        placeholder="Search…"
        value={value}
        onChange={(e) => onValueChange(e.target.value)}
        onKeyDown={(e) => { if (e.key === "Enter") onSearch(); }}
      />
    </div>
    <Button variant="outline" size="sm" onClick={onSearch}>Apply</Button>
    <Button variant="ghost" size="sm" onClick={onReset}>Reset</Button>
  </div>
);

const SchemaDialog: React.FC<{ schema: LanceSchema | null; tableName?: string }> = ({ schema, tableName }) => {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
        <Info className="h-4 w-4 mr-2" /> Schema
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="w-full max-w-md">
          <DialogHeader>
            <DialogTitle>Schema Overview</DialogTitle>
          </DialogHeader>
          {schema ? (
            <div className="space-y-4">
              <div className="text-sm text-muted-foreground">
                Table: <b>{tableName}</b>
              </div>
              <Separator />
              <ScrollArea className="max-h-[70vh]">
                <div className="space-y-3 pr-2">
                  {schema.fields.map((f) => (
                    <div key={f.name} className="flex items-start justify-between gap-3 py-1">
                      <div className="space-y-1">
                        <div className="font-medium">{f.name}</div>
                        <div className="text-xs text-muted-foreground">{describeField(f)}</div>
                      </div>
                      {f.kind === "vector" && (
                        <Badge variant="secondary">{f.dims ?? "?"} dims</Badge>
                      )}
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </div>
          ) : (
            <div className="text-sm text-muted-foreground">No schema loaded yet.</div>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
};

const arrowToLanceSchema = (arrow: ArrowSchemaJSON): LanceSchema => ({
  table: (typeof arrow?.metadata?.["name"] === "string" ? String(arrow.metadata["name"]) : "table"),
  fields: (arrow?.fields ?? []).map(arrowFieldToLance),
});

const arrowFieldToLance = (f: any): LanceField => {
  const name = String(f?.name ?? "");
  const dt = f?.data_type;

  if (typeof dt === "string") {
    return { name, kind: arrowPrimitiveToKind(dt) };
  }

  if (dt && typeof dt === "object") {
    if (dt.FixedSizeList) {
      const [child, size] = dt.FixedSizeList as [any, number];
      const childType = child?.data_type;
      if (childType === "Float32" || childType === "Float64" || childType === "Float16") {
        return { name, kind: "vector", dims: Number(size) || undefined };
      }
      return { name, kind: "array", items: typeof childType === "string" ? arrowPrimitiveToKind(childType) : "unknown" };
    }
    if (dt.List) {
      const [child] = dt.List as [any];
      const childType = child?.data_type;
      return { name, kind: "array", items: typeof childType === "string" ? arrowPrimitiveToKind(childType) : "unknown" };
    }
    if (dt.Struct) {
      return { name, kind: "object" };
    }
    if (dt.Map) {
      return { name, kind: "object" };
    }
  }

  return { name, kind: "unknown" };
};

const arrowPrimitiveToKind = (dt: string): LanceFieldKind => {
  switch (dt) {
    case "Utf8":
    case "LargeUtf8":
    case "Binary":
    case "LargeBinary":
      return "string";
    case "Bool":
      return "boolean";
    case "Int8":
    case "Int16":
    case "Int32":
    case "Int64":
    case "UInt8":
    case "UInt16":
    case "UInt32":
    case "UInt64":
    case "Float16":
    case "Float32":
    case "Float64":
      return "number";
    case "Date32":
    case "Date64":
    case "Timestamp":
      return "date";
    default:
      return "unknown";
  }
};

const stringifyCSV = (v: any) => {
  if (v == null) return "";
  const s = typeof v === "string" ? v : safeStringify(v);
  if (s.includes(",") || s.includes("\n") || s.includes('"')) {
    return '"' + s.replaceAll('"', '""') + '"';
  }
  return s;
};

const safeStringify = (v: any, space?: number) => {
  try {
    return JSON.stringify(v, null, space);
  } catch {
    return String(v);
  }
};

const ensureNumericArray = (v: any): number[] => {
  if (Array.isArray(v)) return v.map(Number).filter((n) => Number.isFinite(n));
  if (typeof v === "string") return parseVector(v) ?? [];
  return [];
};

const parseVector = (text: string | undefined): number[] | undefined => {
  if (!text) return undefined;
  try {
    if (text.trim().startsWith("[")) {
      const arr = JSON.parse(text);
      return Array.isArray(arr) ? arr.map(Number).filter((n) => Number.isFinite(n)) : undefined;
    }
    return text
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter(Boolean)
      .map(Number)
      .filter((n) => Number.isFinite(n));
  } catch {
    return undefined;
  }
};

const formatDate = (v: any) => {
  try {
    const d = new Date(v);
    if (!isNaN(d.getTime())) return d.toISOString().replace("T", " ").slice(0, 19);
  } catch {}
  return String(v);
};

const describeField = (f: LanceField): string => {
  switch (f.kind) {
    case "vector":
      return `${f.kind}${f.dims ? `(${f.dims})` : ""}`;
    case "array":
      return `array<${typeof f.items === "string" ? f.items : (f.items as any)?.kind ?? "unknown"}>`;
    default:
      return f.kind;
  }
};

// Infer schema from values when no Arrow schema is provided
const inferSchemaFromRows = (rows: Record<string, any>[], tableName?: string): LanceSchema => {
  const sample = rows.slice(0, 100);
  const keys = new Set<string>();
  sample.forEach((r) => Object.keys(r ?? {}).forEach((k) => keys.add(k)));
  const fields: LanceField[] = Array.from(keys).map((name) => inferField(name, sample.map((r) => r?.[name])));
  return { table: tableName ?? "table", fields };
};

const inferField = (name: string, values: any[]): LanceField => {
  const nonNull = values.filter((v) => v !== null && v !== undefined);
  if (!nonNull.length) return { name, kind: "unknown" };

  // Vector: numeric arrays with consistent length
  if (nonNull.every((v) => Array.isArray(v) && v.every((n: any) => Number.isFinite(Number(n))))) {
    const len = nonNull[0].length;
    const consistent = nonNull.every((v) => v.length === len);
    if (len > 0 && consistent) return { name, kind: "vector", dims: len };
  }

  // Array
  if (nonNull.every((v) => Array.isArray(v))) {
    const flat = (nonNull as any[]).flatMap((a: any[]) => a);
    const itemKind = inferPrimitiveKind(flat);
    return { name, kind: "array", items: itemKind };
  }

  // Boolean
  if (nonNull.every((v) => typeof v === "boolean")) return { name, kind: "boolean" };

  // Number
  if (nonNull.every((v) => typeof v === "number" && Number.isFinite(v))) return { name, kind: "number" };

  // Date (Date objects or ISO-like strings)
  if (nonNull.every((v) => v instanceof Date || (typeof v === "string" && isISODateString(v)))) {
    return { name, kind: "date" };
  }

  // Object
  if (nonNull.every((v) => typeof v === "object" && v !== null && !Array.isArray(v))) {
    return { name, kind: "object" };
  }

  // String
  if (nonNull.every((v) => typeof v === "string")) return { name, kind: "string" };

  return { name, kind: "unknown" };
};

const inferPrimitiveKind = (vals: any[]): LanceFieldKind | LanceField => {
  const nonNull = vals.filter((v) => v !== null && v !== undefined);
  if (!nonNull.length) return "unknown";
  if (nonNull.every((v) => typeof v === "boolean")) return "boolean";
  if (nonNull.every((v) => typeof v === "number" && Number.isFinite(v))) return "number";
  if (nonNull.every((v) => typeof v === "string")) return "string";
  if (nonNull.every((v) => v instanceof Date || (typeof v === "string" && isISODateString(v)))) return "date";
  if (nonNull.every((v) => Array.isArray(v) && v.every((n: any) => Number.isFinite(Number(n))))) {
    const len = (nonNull[0] as any[]).length;
    const consistent = nonNull.every((v) => (v as any[]).length === len);
    if (len > 0 && consistent) return { name: "", kind: "vector", dims: len }; // unused name
  }
  return "unknown";
};

const isISODateString = (s: string): boolean => {
  return /^\d{4}-\d{2}-\d{2}(?:[ T]\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?Z?)?$/.test(s);
};