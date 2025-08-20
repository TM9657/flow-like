import {
  type ColumnDef,
  type ColumnFiltersState,
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
  Info,
  ListTree,
  Maximize2,
  Minimize2,
  MoreHorizontal,
  Search
} from "lucide-react";
import * as React from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
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
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger
} from "./";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "./table";

/**
 * LanceDBExplorer — Arrow schema + offset/limit paging.
 */
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
  arrowSchema: ArrowSchemaJSON;
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

const DEFAULT_PAGE_SIZE = 50;

const LanceDBExplorer: React.FC<LanceDBExplorerProps> = ({
  tableName,
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
  const [rowSelection, setRowSelection] = useState({});
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [globalQuery, setGlobalQuery] = useState("");

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [lastCount, setLastCount] = useState(0);
  const [fullscreen, setFullscreen] = useState(false);

  useEffect(() => {
    if (arrowSchema) setSchema(arrowToLanceSchema(arrowSchema));
  }, [arrowSchema]);

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
      } else {
        const rows = res.data ?? [];
        setData(rows);
        setLastCount(rows.length);
        if (typeof res.total === "number") setTotal(res.total);
      }
    } catch (e: any) {
      setError(e?.message ?? "Failed to load data");
    } finally {
      setLoading(false);
    }
  }, [onSwitchPage, page, pageSize]);

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
      rowSelection,
      columnFilters,
      globalFilter: globalQuery,
    },
    onColumnVisibilityChange: setColumnVisibility,
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

  // Container classes (fullscreen uses absolute positioning)
  const containerCls = cn(
    "flex min-h-0 flex-col gap-3",
    fullscreen &&
      "absolute inset-0 z-[60] rounded-none bg-background/95 backdrop-blur",
    className,
  );

  return (
    <TooltipProvider>
      <div className={containerCls}>
        <header className="flex items-center gap-2">
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
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm">
                  <Columns3 className="h-4 w-4 mr-2" /> Columns
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuLabel>Toggle columns</DropdownMenuLabel>
                <DropdownMenuSeparator />
                {table
                  .getAllLeafColumns()
                  .filter((c) => c.getCanHide())
                  .map((column) => (
                    <DropdownMenuCheckboxItem
                      key={column.id}
                      className="capitalize"
                      checked={column.getIsVisible()}
                      onCheckedChange={(v) => column.toggleVisibility(!!v)}
                    >
                      {column.id}
                    </DropdownMenuCheckboxItem>
                  ))}
              </DropdownMenuContent>
            </DropdownMenu>
            {children}
          </div>
        </header>

        <Toolbar
          value={globalQuery}
          onValueChange={(v) => setGlobalQuery(v)}
          onSearch={async () => {
            setPage(1);
          }}
          onReset={async () => {
            setGlobalQuery("");
            setPage(1);
          }}
        />

        <div className="rounded-xl border bg-card flex min-h-0 flex-1 flex-col">
          <div className="relative min-h-0 flex-1 overflow-auto">
            <Table>
              {/* Sticky header per th for reliable stickiness inside scroller */}
              <TableHeader className="bg-muted/30">
                {table.getHeaderGroups().map((headerGroup) => (
                  <TableRow key={headerGroup.id}>
                    {headerGroup.headers.map((header) => (
                      <TableHead
                        key={header.id}
                        style={{ width: header.getSize() }}
                        className={cn(
                          "sticky top-0 z-10 bg-card/80 backdrop-blur supports-[backdrop-filter]:bg-card/60",
                          "border-b",
                        )}
                      >
                        {header.isPlaceholder ? null : (
                          <div
                            className={cn(
                              "flex select-none items-center gap-1",
                              header.column.getCanSort() && "cursor-pointer",
                            )}
                            onClick={header.column.getToggleSortingHandler()}
                          >
                            {flexRender(header.column.columnDef.header, header.getContext())}
                            {{ asc: "↑", desc: "↓" }[header.column.getIsSorted() as string] ?? null}
                          </div>
                        )}
                      </TableHead>
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
            </Table>
          </div>

          <div className="flex items-center justify-between px-3 py-2 border-t bg-muted/20">
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
          <div className="text-sm text-destructive flex items-center gap-2">
            <Info className="h-4 w-4" /> {error}
          </div>
        )}
      </div>
    </TooltipProvider>
  );
};

export default LanceDBExplorer;

/*** Helpers ***/
function buildColumnForField(f: LanceField): ColumnDef<Record<string, any>> {
  const id = f.name;
  const accessor = (row: Record<string, any>) => row[id];

  const common: Partial<ColumnDef<Record<string, any>>> = {
    id,
    accessorFn: accessor,
    header: id,
    enableSorting: true,
    cell: ({ getValue }) => <Cell value={getValue()} field={f} />,
  };

  return common as ColumnDef<Record<string, any>>;
}

function Cell({ value, field }: Readonly<{ value: any; field: LanceField }>) {
  if (value == null) return <span className="text-muted-foreground">NULL</span>;

  switch (field.kind) {
    case "boolean":
      return <Checkbox checked={!!value} disabled aria-label="bool" />;
    case "number":
      return <span className="tabular-nums">{value}</span>;
    case "date":
      return <span className="tabular-nums">{formatDate(value)}</span>;
    case "vector": {
      const arr = ensureNumericArray(value);
      const dims = field.dims ?? arr.length;
      return (
        <div className="flex items-center gap-2">
          <Badge variant="outline">{dims}d</Badge>
          <Sparkline data={arr.slice(0, 64)} />
        </div>
      );
    }
    case "array":
    case "object":
    case "unknown":
      return <JsonPreview value={value} />;
    default:
      return (
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="line-clamp-1 max-w-[28rem] inline-block align-middle">
              {String(value)}
            </span>
          </TooltipTrigger>
          <TooltipContent className="max-w-xs break-words">
            {String(value)}
          </TooltipContent>
        </Tooltip>
      );
  }
}

function JsonPreview({ value }: Readonly<{ value: any }>) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="ghost" size="sm" className="h-7 px-2" onClick={() => setOpen(true)}>
        <ListTree className="h-4 w-4 mr-1" /> View
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>Value</DialogTitle>
          </DialogHeader>
          <ScrollArea className="max-h-[70vh]">
            <pre className="text-sm whitespace-pre-wrap break-words p-2 bg-muted rounded-md">
              {safeStringify(value, 2)}
            </pre>
          </ScrollArea>
        </DialogContent>
      </Dialog>
    </>
  );
}

function Sparkline({ data }: Readonly<{ data: number[] }>) {
  const ref = React.useRef<HTMLCanvasElement | null>(null);
  useEffect(() => {
    const canvas = ref.current; if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = 80, h = 20;
    canvas.width = w * dpr; canvas.height = h * dpr; canvas.style.width = `${w}px`; canvas.style.height = `${h}px`;
    const ctx = canvas.getContext("2d"); if (!ctx) return;
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
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    });
    ctx.strokeStyle = getComputedStyle(document.documentElement).getPropertyValue("--primary");
    ctx.stroke();
  }, [data]);
  return <canvas ref={ref} aria-label="sparkline" />;
}

function Toolbar({
  value,
  onValueChange,
  onSearch,
  onReset,
}: Readonly<{
  value: string;
  onValueChange: (v: string) => void;
  onSearch: () => void | Promise<void>;
  onReset: () => void | Promise<void>;
}>) {
  return (
    <div className="flex flex-wrap items-center gap-2">
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
}

function SchemaDialog({ schema, tableName }: Readonly<{ schema: LanceSchema | null, tableName?: string }>) {
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
}

/*** Arrow -> Lance mapping ***/
function arrowToLanceSchema(arrow: ArrowSchemaJSON): LanceSchema {
  const fields: LanceField[] = (arrow?.fields ?? []).map(arrowFieldToLance);
  return {
    table: (typeof arrow?.metadata?.["name"] === "string" ? String(arrow.metadata["name"]) : "table"),
    fields,
  };
}

function arrowFieldToLance(f: any): LanceField {
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
}

function arrowPrimitiveToKind(dt: string): LanceFieldKind {
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
}

/*** Utility ***/
function stringifyCSV(v: any) {
  if (v == null) return "";
  const s = typeof v === "string" ? v : safeStringify(v);
  if (s.includes(",") || s.includes("\n") || s.includes('"')) {
    return '"' + s.replaceAll('"', '""') + '"';
  }
  return s;
}

function safeStringify(v: any, space?: number) {
  try { return JSON.stringify(v, null, space); } catch { return String(v); }
}

function safeParse(text: string) {
  try { return JSON.parse(text); } catch { return undefined; }
}

function ensureNumericArray(v: any): number[] {
  if (Array.isArray(v)) return v.map(Number).filter((n) => Number.isFinite(n));
  if (typeof v === "string") return parseVector(v) ?? [];
  return [];
}

function parseVector(text: string | undefined): number[] | undefined {
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
}

function formatDate(v: any) {
  try {
    const d = new Date(v);
    if (!isNaN(d.getTime())) return d.toISOString().replace("T", " ").slice(0, 19);
  } catch {}
  return String(v);
}

function describeField(f: LanceField): string {
  switch (f.kind) {
    case "vector": return `${f.kind}${f.dims ? `(${f.dims})` : ""}`;
    case "array": return `array<${typeof f.items === "string" ? f.items : (f.items as any)?.kind ?? "unknown"}>`;
    default: return f.kind;
  }
}