import type { IToolActivityDetail } from "../../components/interfaces/chat-default/chat-db";

const MAX_CONTEXT_CHARS = 240;
const MAX_PREVIEW_CHARS = 64 * 1024;
const MAX_SOURCES = 4;

const TOOL_ACTIVITIES: Record<
	string,
	[IToolActivityDetail["category"], string]
> = {
	http_request: ["web", "Making an HTTP request"],
	open_url: ["web", "Reading a webpage"],
	web_fetch: ["web", "Reading a webpage"],
	fetch_url: ["web", "Reading a webpage"],
	archive_lookup: ["web", "Checking the web archive"],
	internet_search: ["search", "Searching the web"],
	web_search: ["search", "Searching the web"],
	research_agent: ["search", "Researching sources"],
	list_apps: ["apps", "Checking available apps"],
	search_apps: ["apps", "Searching apps"],
	describe_app_interface: ["apps", "Inspecting an app interface"],
	call_app_chat: ["apps", "Asking an app"],
	call_app_event: ["apps", "Running an app action"],
	open_app_chat: ["apps", "Opening an app chat"],
	open_app_page: ["apps", "Opening an app page"],
	create_app: ["apps", "Creating an app"],
	fork_app: ["apps", "Copying an app"],
	storage_tool: ["files", "Working with files"],
	read_file: ["files", "Reading a file"],
	list_files: ["files", "Listing files"],
	database_tool: ["database", "Working with data"],
	forecast_tool: ["database", "Working with forecasts"],
	query: ["database", "Querying data"],
	data_studio_agent: ["database", "Working with app data"],
	flowpilot_board: ["workflow", "Working on a workflow"],
	flowpilot_widget: ["workflow", "Building an interface"],
	flowpilot_home: ["workflow", "Updating Home"],
	project_scout: ["apps", "Inspecting the project"],
	edit_flowscript: ["workflow", "Editing a workflow"],
	write_flowscript: ["workflow", "Writing a workflow"],
	patch_flowscript: ["workflow", "Updating a workflow"],
	check_flowscript: ["workflow", "Checking a workflow"],
	test_flowscript: ["workflow", "Testing a workflow"],
	commit_flowscript: ["workflow", "Saving a workflow"],
	run_board_tests: ["workflow", "Testing a workflow"],
	execute_event: ["workflow", "Running an event"],
	catalog_search: ["search", "Searching the node catalog"],
	get_declarations: ["search", "Looking up node definitions"],
	search_by_pin: ["search", "Finding compatible nodes"],
	ask_user: ["tool", "Requesting your input"],
};

function toolKey(name: string): string {
	return (
		name
			.split(/__|[./]/)
			.at(-1)
			?.toLowerCase() ?? name.toLowerCase()
	);
}

export function getToolActivityLabel(name: string): string {
	const key = toolKey(name);
	if (TOOL_ACTIVITIES[key]) return TOOL_ACTIVITIES[key][1];
	const label = key.replace(/[_-]+/g, " ").trim();
	return label ? label[0].toUpperCase() + label.slice(1) : "Working";
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
	let parsed = value;
	if (typeof value === "string" && value.length <= MAX_PREVIEW_CHARS) {
		try {
			parsed = JSON.parse(value);
		} catch {
			return undefined;
		}
	}
	return parsed && typeof parsed === "object" && !Array.isArray(parsed)
		? (parsed as Record<string, unknown>)
		: undefined;
}

function sourceFromUrl(value: unknown) {
	if (typeof value !== "string" || value.length > 8192) return undefined;
	try {
		const url = new URL(value);
		if (url.protocol !== "https:" && url.protocol !== "http:") return undefined;
		const hadQuery = !!url.search;
		url.username = "";
		url.password = "";
		url.search = "";
		url.hash = "";
		return {
			url: url.href,
			hostname: url.hostname,
			...(url.pathname !== "/" ? { path: url.pathname } : {}),
			...(hadQuery ? { linkable: false } : {}),
		};
	} catch {
		return undefined;
	}
}

/** Keep display text short and remove credentials and URL parameters from summaries. */
export function toolActivityText(value: unknown): string | undefined {
	if (typeof value !== "string") return undefined;
	const text = value.trim();
	// SDK fallback summaries can be a complete JSON argument preview.
	if (!text || /^[{[]/.test(text)) return undefined;
	const cleaned = text
		.replace(
			/https?:\/\/[^\s<>"']+/gi,
			(url) => sourceFromUrl(url)?.url ?? "[URL]",
		)
		.replace(/\b(Bearer|Basic)\s+[^\s,;]+/gi, "$1 [redacted]")
		.replace(
			/\b(?:sk-|ghp_|github_pat_|xox[baprs]-|AKIA|AIza)[\w-]+/g,
			"[redacted]",
		)
		.replace(
			/\b(?:password|passwd|secret|token|api[_-]?key|authorization|cookie)\s*[:=]\s*(?:"[^"]*"|'[^']*'|[^\s,;]+)/gi,
			"[redacted]",
		)
		.replace(/\s+/g, " ");
	return cleaned.length > MAX_CONTEXT_CHARS
		? `${cleaned.slice(0, MAX_CONTEXT_CHARS - 1)}…`
		: cleaned;
}

function firstText(records: Record<string, unknown>[], keys: string[]) {
	for (const record of records) {
		for (const key of keys) {
			const value = toolActivityText(record[key]);
			if (value) return value;
		}
	}
	return undefined;
}

function collectSources(records: Record<string, unknown>[]) {
	const sources: NonNullable<IToolActivityDetail["sources"]> = [];
	const add = (value: unknown, linkable?: unknown) => {
		const source = sourceFromUrl(value);
		if (source && linkable === false) source.linkable = false;
		const existing = source && sources.find((item) => item.url === source.url);
		if (existing && source?.linkable === false) existing.linkable = false;
		if (source && !existing) sources.push(source);
	};
	for (const record of records) {
		for (const key of ["url", "uri", "href", "requested_url", "source_url"])
			add(record[key]);
		for (const key of ["urls", "sources", "results"]) {
			const entries = record[key];
			if (!Array.isArray(entries)) continue;
			for (const entry of entries.slice(0, MAX_SOURCES)) {
				if (typeof entry === "string") add(entry);
				else {
					const source = asRecord(entry);
					add(source?.url ?? source?.href ?? source?.uri, source?.linkable);
				}
			}
		}
	}
	return sources.slice(0, MAX_SOURCES);
}

/** Extract only display context. Raw arguments, request bodies and tool results stay out. */
export function buildToolActivity(
	name: string,
	data: unknown,
	previous?: IToolActivityDetail,
): IToolActivityDetail {
	const key = toolKey(name);
	const category = TOOL_ACTIVITIES[key]?.[0] ?? previous?.category ?? "tool";
	const record = asRecord(data) ?? {};
	const args = asRecord(
		record.arguments ??
			record.input ??
			record.parameters ??
			record.arguments_preview,
	);
	const result = asRecord(
		record.result ?? record.output ?? record.result_preview,
	);
	const context = [args, record].filter(
		(item): item is Record<string, unknown> => !!item,
	);
	const results = result ? [result] : [];
	// MCP providers may wrap a JSON tool result in a text content block.
	if (Array.isArray(result?.content)) {
		for (const block of result.content.slice(0, MAX_SOURCES)) {
			const content = asRecord(asRecord(block)?.text);
			if (content) results.push(content);
		}
	}
	const sources = collectSources([
		...(previous?.sources ? [{ sources: previous.sources }] : []),
		...context,
		...results,
	]);
	const query = ["search", "apps"].includes(category)
		? firstText(context, ["query", "search_query", "searchQuery", "q"])
		: undefined;
	const targetKeys =
		category === "files"
			? ["path", "file_path", "filePath", "filename", "prefix"]
			: category === "database"
				? [
						"table_name",
						"tableName",
						"table",
						"database_name",
						"app_name",
						"appName",
					]
				: category === "apps"
					? [
							"app_name",
							"appName",
							"name",
							"event_name",
							"eventName",
							"app_id",
							"appId",
						]
					: category === "workflow"
						? [
								"board_name",
								"boardName",
								"workflow_name",
								"page_name",
								"route",
								"app_name",
								"appName",
							]
						: [];
	const target = firstText(context, targetKeys);
	const rawMethod =
		key === "http_request"
			? firstText([...context, ...results], ["method"])?.toUpperCase()
			: undefined;
	const method =
		rawMethod && /^(GET|HEAD|POST|PUT|PATCH|DELETE|OPTIONS)$/.test(rawMethod)
			? rawMethod
			: key === "http_request"
				? (previous?.method ??
					(args && !("method" in args) && !("method" in record)
						? "GET"
						: undefined))
				: undefined;
	const operation = ["database", "files"].includes(category)
		? firstText(context, ["operation"])
		: undefined;
	const forecastLabels: Record<string, string> = {
		list_models: "Checking forecasting models",
		run: "Running a forecast",
		list: "Listing saved predictions",
		get: "Opening a saved prediction",
		save: "Saving a prediction",
	};
	const label =
		key === "forecast_tool"
			? (forecastLabels[operation ?? ""] ??
				previous?.label ??
				"Working with forecasts")
			: key === "http_request"
				? method
					? ["GET", "HEAD"].includes(method)
						? "Fetching a URL"
						: "Sending an HTTP request"
					: "Making an HTTP request"
				: operation && /^[a-z_]+$/i.test(operation)
					? getToolActivityLabel(operation)
					: (previous?.label ?? getToolActivityLabel(name));
	const httpStatus = [...results, record]
		.map((item) => item.http_status)
		.find(
			(status) =>
				typeof status === "number" &&
				Number.isInteger(status) &&
				status >= 100 &&
				status <= 599,
		);
	const summary = toolActivityText(record.result_summary);
	const usefulSummary =
		summary && !/^(ok|done|completed|success)$/i.test(summary)
			? summary
			: undefined;
	const appInventory =
		key === "list_apps"
			? results.find(
					(item) =>
						typeof item.returned === "number" &&
						Number.isInteger(item.returned) &&
						item.returned >= 0,
				)
			: undefined;
	const appsOutcome = appInventory
		? `${appInventory.returned} app${appInventory.returned === 1 ? "" : "s"} found${appInventory.complete === false ? " · Partial inventory" : ""}`
		: undefined;
	const outcome = httpStatus
		? usefulSummary && !usefulSummary.includes(`HTTP ${httpStatus}`)
			? `HTTP ${httpStatus} · ${usefulSummary}`
			: `HTTP ${httpStatus}`
		: (appsOutcome ?? usefulSummary);
	return {
		...previous,
		kind: "tool_activity",
		category,
		label,
		...(target ? { target } : {}),
		...(query ? { query } : {}),
		...(method ? { method } : {}),
		...(sources.length ? { sources } : {}),
		...(outcome ? { outcome } : {}),
	};
}
