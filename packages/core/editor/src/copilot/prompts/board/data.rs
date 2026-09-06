//! Database and UI integration guidance for workflow authors.

/// Canonical data/database workflow guidance shared by board prompts.
pub const DATABASE_WORKFLOW_GUIDANCE: &str = r#"
## DATA AND DATABASE WORKFLOWS
Use Flow-Like's built-in database nodes as the default data architecture. Do NOT ask the user which
external vector database to use unless they explicitly request an external service. The built-in
database is LanceDB-backed and is opened with **Open Database** (`open_local_db`, FlowScript
`db::open`), which returns the database connection `Struct` directly. Database nodes live
in the `db` namespace: write `use db::*` once at the top and call them bare, or qualify each call.

### DATE/TIME TYPE CONTRACT
Treat every value that represents a real instant—such as `created_at`, `updated_at`, `scheduled_at`,
or an event time—as a FlowLike `Date` throughout the board. In FlowScript, use `Date` for the field
in interfaces and for function/event parameters, return values, and variables. Produce current
values with `datetime::now`, parse external text with `datetime::parse`, and pass the resulting
Date pin directly into `struct::set`. Never format or coerce it to `string` or an epoch number before
a database write merely because its JSON boundary representation is RFC3339.

The matching Lance schema handoff is exactly `type: "timestamp:ms:UTC"`. That is a native
millisecond UTC timestamp column, not a text column; it accepts the RFC3339 UTC value carried by a
FlowLike Date. Use `date32` only for a deliberately calendar-only value with no time or timezone.
When a temporal field is exchanged with a board as FlowLike `Date`, use `timestamp:ms:UTC` even if
the UI happens to show only its calendar portion; reserve `date32` for standalone calendar data that
is intentionally not a board Date.

An existing table's described schema remains authoritative. On writes, a legacy Utf8/LargeUtf8
column can continue receiving the RFC3339 JSON string carried by a Date pin. On reads, treat that
legacy column as raw text at the storage boundary: use `to_timestamp(column)` for temporal SQL
sorting/filtering and `datetime::parse` before passing the value to a Date consumer. Keep the
workflow's semantic variables, parameters, and returns typed `Date`; only the legacy raw column is
`string`. For a native `timestamp:ms:UTC` column, sort/filter it directly and pass its Date value
without reparsing.

Any view, list, dashboard, or lookup over persisted data MUST read the rows back through a real
read node (`db::filter`, `db::list`, the fts/vector/hybrid search nodes, or a DataFusion
`df::sqlQuery` over registered tables) in the same workflow. Opening the database alone reads
nothing, and rendering from in-memory state that was just written is a correctness bug: the flow
must work on a fresh run where memory is empty.

SETUP FUNCTION — populate shared references once:
Start the workflow with one `function setup() { ... }`, called first from the entry event, that
resolves every long-lived reference (database connections, embedding/LLM models) and stores each
in a top-level variable via its variable set node. Downstream functions read them with
`variable::get` instead of re-opening or re-loading per call, and the user adjusts everything in ONE
place.
- Embedding models load from a Bit, never from an invented id:
  `const bit = ai::loadBit({ bitId: "" })` — leave `bitId` as the empty string; the user selects
  the concrete bit on the board later — then `const model = ai::embedding::loadModel({ bit: bit })`
  and store `model` into a top-level variable.
- Databases: `db::open({ name: "..." })` stored into a variable the same way.

Inspect before you design: when `database_tool` is registered for a board specialist, use only its
read-only operations (`list_tables`, `describe_table`, and read-only `query`) to inspect schemas,
indices, row counts, and sample rows. Never call its create/insert/update/delete/delete_table/index/
optimize/schema operations from a board-specialist run; dropping a table is irreversible and is never
part of authoring a board. Those out-of-band data mutations belong to the Data Studio
specialist or outer orchestrator; report the needed schema as a handoff instead of performing it.
In a CREATE/ADD/BUILD board mutation, out-of-band database setup is
never a prerequisite for the first complete FlowScript submission. Use at most one table-list/schema
inspection, make one bounded, focused `get_declarations` lookup for the highest-leverage catalog
calls, call `plan_board_scope` exactly once after any usable declaration batch unless the host
already retained an accepted plan, and submit its active segment through `write_flowscript`
immediately. Do not chase omitted or unmatched searches or wait for every missing table before
retaining source. Check and commit the retained source while explicit schemas are pending. The
FlowScript may reference intended built-in table names and may implement the requested runtime
first-write behavior; it must not mutate app data through a support tool while constructing the
board.

### A DATABASE OR INDEX YOU COULD NOT SET UP NEVER STOPS THE BUILD
When a requested table does not exist, return a data-specialist handoff with explicit
`fields: [{name, type, nullable?, vector_size?}]`; use `type: "vector"` plus `vector_size` for
float32 embeddings. That handoff is a REPORT, not a gate.

Whenever out-of-band setup fails or is unavailable for ANY reason — a table that cannot be created,
an index that cannot be built, an optimize that is refused, an approval the user declined, any HTTP
error, or a `status: "partial"` with `code: "explicit_schema_create_not_deployed"` (often surfacing
as HTTP 405 on a local runtime) — the answer is always the same: BUILD THE WORKFLOW ANYWAY and let
it perform the setup at runtime. Never abandon, shrink or stub a board because a database, table or
index could not be prepared out of band; that is a support-tool limitation, not an unbuildable unit,
and the FlowScript for it is fully expressible. Never replace or postpone the workflow with a
database smoke test merely to make table creation pass.
One such result proves the capability mismatch for the current session: do not retry the HTTP
capability probe or wait for deployment in this run.
Record any remaining requested schemas as pending and finish/apply the board.

### THE WORKFLOW IS THE BETTER PLACE TO CREATE A TABLE
Runtime setup is not a consolation prize — for embedding/vector tables it is the RIGHT design.
The portable bootstrap is LAZY: LanceDB creates a table from its first write, so the first row IS
the schema. Writing one real row derives every column type from actual runtime values, including
the exact vector width of the embedding model the board loaded. An out-of-band `create_table` has
to GUESS that width, and a wrong `vector_size` produces a table every later embedding write rejects.

Design new-table workflows around that lazy first-write bootstrap by default, so a missing schema
endpoint costs zero extra steps:
1. `db::open({ name })` in `setup()`, stored in a variable next to the loaded embedding model.
2. Have the WORKFLOW upsert one COMPLETE first row via `db::upsert`/`db::batchUpsert` — every
   column present with a correctly typed value, embeddings produced by `ai::embedding::embedDocument`, and a
   zero-filled vector for vector columns that have nothing to embed yet. The table and its schema
   then exist for every later query.
3. Build indices IN THE FLOW with `db::buildIndex`, AFTER that first write — indexing a table that
   does not exist yet fails, so the order is load-bearing. Put index building at the end of the
   ingest path or in a separate maintenance/reindex event, never in a read path where it would
   rebuild on every query. `db::vectorSearch` works without an index; `db::ftsSearch` and
   `db::hybridSearch` need their `FULL TEXT` index built first.
4. `db::optimize` after large writes or index updates.

Recommended patterns:
- Persistent table / record store: `db::open` -> `db::insertOne` / `db::batchInsert` for
  fast append, or `db::upsert` / `db::batchUpsert` when there is a stable ID column.
- Big-data analytics: `db::open` -> `df::createSession` -> `df::registerLance` -> `df::sqlQuery`.
  DataFusion SQL works after sources are registered as tables in the session. For file/object data,
  use the DataFusion mount/register nodes for Parquet, CSV, JSON, data lakes, or external
  databases (`df::registerPostgres`, `df::registerMysql`, `df::registerSqlite`, `df::registerDuckdb`,
  `df::registerClickhouse`, BigQuery, Athena, Iceberg/Delta/Hudi), then query with `df::sqlQuery`.
- Vector/RAG ingest: load an embedding Bit with `ai::embedding::loadModel`, create vectors with
  `ai::embedding::embedDocument` for each document/chunk, then store rows containing text, metadata,
  IDs, and vector columns with `db::batchInsert` / `db::batchUpsert`.
- Uploaded document ingest: a file picker or chat attachment yields a `FlowPath`; that reference is
  not extracted text. For every requested file-read or file-store path, call a real extraction
  catalog operation such as `ai::processing::extractDocument({ file, extractImages? })` (node type
  `ai_processing_extract_document`) or its multi-document/AI variant, then consume the returned
  page content. `ui::getFileInputFiles` only obtains the selected file references. Never replace
  extraction with a filename, status message, empty string, or other placeholder literal. When
  extraction is requested, include one of these extraction nodes in the submitted FlowScript even
  if no file is available at authoring time; handle the missing-file case as a runtime branch.
- Vector search: embed the user's query with `ai::embedding::embedQuery`, then use `db::vectorSearch`
  with an optional SQL filter and an explicit limit.
- Keyword search: build a `FULL TEXT` index with `db::buildIndex` on the text column, then use
  `db::ftsSearch`.
- Hybrid search: build indexes for the vector column (`VECTOR` or `AUTO`) and text column
  (`FULL TEXT`), embed the query with `ai::embedding::embedQuery`, then call `db::hybridSearch` with both the
  search string and vector. Its `fields` input expects the vector column first and the FTS text
  column after that; keep `rerank` enabled unless the user asks otherwise.
- Indexing/maintenance: use `db::buildIndex` ("Build Index") for `VECTOR`, `FULL TEXT`, `BTREE`,
  `BITMAP`, `LABEL LIST`, or `AUTO`; use `db::listIndices` to inspect indices and
  `db::optimize` after large writes or index updates.

### DataFusion sessions (the analytics + dashboard-data path)
DataFusion is the right tool whenever a workflow needs SQL — aggregations, joins, ordering,
filtering, or shaping rows for a dashboard. The lifecycle is always the same:
1. `db::open({ name, userScoped, batchSize })` for each table you need.
2. `df::createSession({ sessionName: "default" })` ONCE — every other pin is an optional tuning
   default — then reuse the returned `session` for every register/query in that path. Do not
   create a new session per query or per helper; pass the session to helper functions as a
   `Struct` parameter instead.
3. `df::registerLance({ session, database, tableName })` (or a file/external register node) for each
   source. The `tableName` is the SQL identifier you then `SELECT ... FROM`.
4. `df::sqlQuery({ session, query })` returns THREE outputs from one call
   (`const { table, rows, rowCount } = sqlQuery({ … })`):
   - `table` — a `CSVTable` (columnar) made for analytics and charts/tables. Feed this straight
     into `ui::pushCsvToChart` (format `CSV`) for dashboard widgets.
   - `rows` — an array of row structs for `for (const row of rows)` iteration and per-row UI (set
     element text, instantiate widgets). Access fields as `row.<column>`.
   - `rowCount` — the integer result count, e.g. for a `${rowCount} results` badge.
   Build a SQL string with a template literal / `string::format` only for trusted fragments; runtime
   values go through `$placeholder` query params, never concatenated into SQL.

Look up exact FlowScript signatures with ONE bounded, focused `get_declarations` call before writing
these calls: put the highest-leverage searches in `queries` (never blank), e.g. `{"queries": ["open database",
"datafusion create session register lance", "sql query", "push csv to chart", "embedding",
"hybrid search build index"]}`. After any usable declaration response, call `plan_board_scope`
exactly once (unless the host already retained an accepted plan), then retain its active segment
immediately. Defer omitted or unmatched searches until compiler diagnostics identify a concrete
gap; use `catalog_search` only for read-only exploration, not to postpone the first write.
"#;

/// How a workflow drives A2UI pages/widgets (dashboards) and where to get real element references.
pub const DASHBOARD_A2UI_GUIDANCE: &str = r#"
## DASHBOARDS, PAGES, AND WIDGETS (A2UI)
A board renders interactive UI by calling `ui::*` nodes (legacy `a2ui*` names) that target elements
on the app's **pages** and instantiate its **widgets**. Write `use ui::*` at the top of a
dashboard board and call the members bare. A board does NOT contain those element ids — they live in separate
page/widget definitions — so you must look them up, not guess.

GROUND YOURSELF FIRST: before writing or editing ANY `ui::*` call, call `ui_inspect` (read-only, no
approval). `ui_inspect` with operation `list` returns every page (with `element_refs`), every
project widget (with `selector`), and every widget shipped by an installed package under
`package_widgets`; `page`/`widget` return the full detail for one. Never invent an `elementRef` or a
`widgetSelector` — if `ui_inspect` does not list it, it does not exist.

Reference conventions:
- An element reference is `"<page_id>/<element_id>"`, exactly as returned by `ui_inspect`.
- A widget selector is the widget's name (its `selector` from `ui_inspect`). A PACKAGE widget's
  selector is instead the `pkg:{package_id}/{widget_id}` string `ui_inspect` reports for it — pass
  that verbatim to `ui::instantiateWidget`; its `dyn*` input pins come from the widget's contract.

Common `ui::*` calls (confirm exact signatures with `get_declarations`):
- Read/write elements: `ui::setElementText({ elementRef, text })`,
  `ui::setMarkdownContent({ elementRef, markdown })`, `ui::setBadgeContent`,
  `ui::setElementValue`, and `ui::getElement({ elementRef }).element` /
  `ui::getElementValue({ elementRef }).value` to read current values (e.g. form inputs).
- Containers (grids/lists): clear with `ui::clearChildren({ containerRef: ui::getElement({ elementRef }).element })`,
  then add children with `ui::pushToContainer({ containerRef, elementRef, position: -1 })` or
  `ui::pushChild({ containerRef, childRef })`.
- Widgets: `ui::instantiateWidget({ widgetSelector, instanceId, dynPath<Field>: …, dynProp<Id>: …, fnRefs: [handlerEntry] })`
  returns `elementRef` to push into a container. The `dynPath*`/`dynProp*` input pins for a widget
  are listed by `ui_inspect` (operation `widget`). `fnRefs` entries must be `eventsWidgetAction`
  ENTRIES (not plain functions): declare one `eventsWidgetAction handlerName(widgetInstanceId: string, eventName: string, actionContext: Struct, inputValues: Struct) { … }`
  per widget action and pass the bare handler names. A handler serves as catch-all for the
  widget's actions; branch on the delivered `eventName`/`actionContext` inside the handler when
  one widget declares several actions.
- Charts (dashboard data): `ui::pushCsvToChart({ elementRef, library: "Nivo"|"Plotly", format: "CSV", table: <df::sqlQuery>.table, chartType: "Bar"|"Line"|"Pie"|… })`.
  The `table` pin accepts a DataFusion query result directly — this is the primary way to drive a
  dashboard chart from SQL. Use `format: "JSON"` with a `data` array when you already shaped the
  series yourself. Style with `ui::setNivoConfig` / `ui::setChartLayout`.
- Tables (dashboard data — often the most useful for SQL): `ui::writeCsvToTable({ elementRef, table: <df::sqlQuery>.table })`
  pushes a DataFusion result straight into a table element (or pass `csv` text). For incremental
  edits use `ui::updateTable` (set/append/replace rows). DataFusion's `table` output is built
  exactly for these table/chart pins, so prefer it over hand-iterating rows when filling a grid.
- Data-path updates: `ui::dataUpdate({ surfaceId, path, value })` is FORBIDDEN. Writing a surface
  data path does not change what the page renders; use the element setters and widget nodes above
  (see the a2ui page rules).
- Screen control: end a render path with `ui::showScreen()`; route with `ui::navigateTo({ route })`;
  read URL params with `ui::getQueryParam({ paramName }).value`.

### Interaction events PULL their own inputs
A page/widget action only INVOKES its handler — the dashboard never pushes element values into it.
NEVER declare a Generic Event with payload parameters (`payload`, `actionId`, `targetId`, `url`,
…) expecting the page to fill them from its inputs. Instead the handler body FETCHES the state it
needs from the page: `ui::getElementValue({ elementRef }).value` for inputs/selects,
`ui::getFileInputFiles` for uploads, `ui::getElement({ elementRef }).element` for anything else.
Compact correct shape — action invokes a named entry, the body reads the element, validates,
persists, then refreshes via an element setter:
```ts
use ui::*

addTarget() {
    const { value: raw } = getElementValue({ elementRef: "<page_id>/target-url-input" })
    const targetUrl = json::stringify({ value: raw })
    if (targetUrl != "") {
        const db = db::open({ name: "targets", userScoped: false, batchSize: 1000 })
        const id = random::cuid()
        let row = struct::make()
        row = row.set({ field: "id", value: id })
        row = row.set({ field: "url", value: targetUrl })
        db.upsert({ value: row, idRow: "id" })
        setElementValue({ elementRef: "<page_id>/target-url-input", value: "" })
        refreshTargetsTable()
    }
}
```
(`refreshTargetsTable` is a helper function that re-queries and calls `ui::writeCsvToTable`.)

Keep dashboards clean with functions/layers: put each page's onLoad logic in its own
`function pageLoad() { … }` (it becomes a Function layer), and factor repeated work — querying a
table, filling a container with widget instances — into small helper functions instead of one long
event block. See the dashboard examples below.
"#;

/// A2UI page contract: how board logic pushes values into a live UI page. Prevents two recurring
/// mistakes: using page/global state (a scratch store) to drive the screen, and using the generic
/// `a2uiDataUpdate` data-path node instead of an element setter or a widget instance. A leftover
/// `a2uiDataUpdate` does not block the commit; it returns an `FS_PROHIBITED_NODE` review note whose
/// directive sends the model back for another revision.
pub const A2UI_STATE_GUIDANCE: &str = r#"
## A2UI PAGES: UPDATING WHAT AN ELEMENT SHOWS
A board never pushes data into a page's data model. It writes to the ELEMENT with that element's
setter, or it instantiates/updates a WIDGET with typed inputs. There is no third option:

- Text/labels/status: `ui::setElementText` (Set Element Text), `ui::setMarkdownContent`,
  `ui::setBadgeContent`, `ui::setProgress`.
- Input values: `ui::setElementValue` (Set Element Value), `ui::setSelectValue`,
  `ui::setSliderValue`.
- Tables: `ui::writeCsvToTable` (Push CSV to Table) for full data, `ui::updateTable` for
  incremental row edits.
- Charts: `ui::pushCsvToChart` (Push Data to Chart).
- Package widgets: `ui::instantiateWidget` with one `dyn*` input per contract field, then
  `ui::pushChild` / `ui::pushToContainer`; `ui::widgetUpdateInputs` to patch a live instance.
Target elements with the `ui_inspect` element ref (`"<page_id>/<element_id>"`) directly or via
`ui::getElement({ elementRef }).element`.

### Rendering a list of records
Never assemble a data blob and write it at a path. Clear the container, loop the records, read each
field with `struct::get` (or a dot-path), instantiate one widget per record with those fields on
its generated `dyn*` input pins, and push each instance into the container:
```ts
use ui::*

function renderSources(rows: Struct[]) {
    const { element: grid } = getElement({ elementRef: "<page_id>/sources-list" })
    clearChildren({ containerRef: grid })
    for (const row of rows) {
        const instance = instantiateWidget({
            widgetSelector: "Knowledge Source Card",
            instanceId: row.get({ field: "id" }).value,
            dynPathDocument: row.get({ field: "document" }).value,
            dynPathChunkCount: row.get({ field: "chunk_count" }).value,
        })
        pushChild({ containerRef: grid, childRef: instance })
    }
}
```
The generated input pin names differ per widget — `ui_inspect` (operation `widget`) lists the exact
ones for the selected widget; never guess them. Re-rendering the same list repeats this loop;
changing one field on an already mounted instance uses `ui::widgetUpdateInputs` against that
instance's element ref.

- **Data Update** (`ui::dataUpdate`) is FORBIDDEN. Writing `$.data.<path>` does not change what the
  page renders — elements own their own state and widget instances read typed contract inputs, so
  neither observes the write. Every case it looks right for is one of the setters or widget nodes
  above. Each one left on the board returns an `FS_PROHIBITED_NODE` review note: the batch still
  commits, but the work is NOT done until you write a further revision that replaces it. Never
  report a board as finished while such a note stands.
- **Set Page State** (`ui::setPageState`) does NOT touch `$.data.*` bindings and will NOT update the
  screen. Page state is a separate per-page key/value store that widgets never read; its value only
  travels back to the board on the NEXT event, where **Get Page State** (`ui::getPageState`) reads
  it. Use it for cross-event scratch data scoped to a page. Its `key` is a plain identifier (e.g.
  `"lastQuery"`), never a `$.data...` path.
- **Set/Get Global State** behave like page state but shared across pages — same rule, not for
  display.

Rule of thumb: value must be visible now -> the setter for that element type, or a widget instance
carrying it as an input. Value must survive to a later event/handler -> page/global state. When
unsure which setter an element takes, call `get_declarations` for the names above and read the
signatures before writing.
"#;
