import { describe, expect, test } from "bun:test";
import {
	buildToolActivity,
	getToolActivityLabel,
	toolActivityText,
} from "./tool-step-activity";

describe("tool activity context", () => {
	test("extracts a request destination from the provider's JSON preview", () => {
		const detail = buildToolActivity("http_request", {
			arguments_preview: JSON.stringify({
				url: "https://user:secret@example.com/docs/license?token=hidden#private",
				method: "get",
				headers: { Authorization: "Bearer private-key" },
				body: "private request body",
			}),
		});
		expect(detail).toEqual({
			kind: "tool_activity",
			category: "web",
			label: "Fetching a URL",
			method: "GET",
			sources: [
				{
					url: "https://example.com/docs/license",
					hostname: "example.com",
					path: "/docs/license",
					linkable: false,
				},
			],
		});
	});

	test("rejects executable and local URL schemes", () => {
		for (const url of [
			"javascript:alert(1)",
			"data:text/html,private",
			"file:///private/file",
		])
			expect(
				buildToolActivity("open_url", { arguments: { url } }).sources,
			).toBeUndefined();
	});

	test("deduplicates and bounds sources without preserving URL parameters", () => {
		const detail = buildToolActivity("internet_search", {
			input: { query: "license terms" },
			result_preview: JSON.stringify({
				sources: [
					{ url: "https://example.com/license?q=one" },
					{ url: "https://example.com/license?q=two" },
					{ url: "https://other.example/terms" },
					{ url: "javascript:alert(1)" },
				],
			}),
		});
		expect(detail.query).toBe("license terms");
		expect(detail.sources?.map((source) => source.url)).toEqual([
			"https://example.com/license",
			"https://other.example/terms",
		]);
	});

	test("retains only relevant resource fields for app, file and data tools", () => {
		expect(
			buildToolActivity("list_apps", { arguments: { query: "weather" } }).query,
		).toBe("weather");
		expect(
			buildToolActivity("call_app_event", {
				arguments: {
					app_name: "Weather",
					app_id: "app-1",
					instruction: "a long private instruction",
				},
			}).target,
		).toBe("Weather");
		expect(
			buildToolActivity("storage_tool", {
				arguments: {
					operation: "read_file",
					path: "reports/october.csv",
					content: "private document contents",
				},
			}),
		).toMatchObject({
			category: "files",
			label: "Reading a file",
			target: "reports/october.csv",
		});
		expect(
			buildToolActivity("database_tool", {
				arguments: {
					operation: "list_rows",
					table_name: "Orders",
					query: "select secret from credentials",
				},
			}),
		).toEqual({
			kind: "tool_activity",
			category: "database",
			label: "List rows",
			target: "Orders",
		});
	});

	test("reads HTTP results inside MCP text envelopes", () => {
		const detail = buildToolActivity("http_request", {
			result_preview: JSON.stringify({
				content: [
					{
						type: "text",
						text: JSON.stringify({
							http_status: 404,
							url: "https://example.com/missing?key=private",
							body: "private response text",
						}),
					},
				],
			}),
			result_summary: "Page was not found",
		});
		expect(detail.outcome).toBe("HTTP 404 · Page was not found");
		expect(detail.sources?.[0].url).toBe("https://example.com/missing");
		expect(JSON.stringify(detail)).not.toContain("private");
	});

	test("preserves operation labels and context when a completion only has status", () => {
		const previous = buildToolActivity("storage_tool", {
			arguments: { operation: "read_file", path: "data.csv" },
		});
		expect(
			buildToolActivity("storage_tool", { status: "completed" }, previous),
		).toEqual(previous);
	});

	test("keeps the requested URL when completion adds response sources", () => {
		const previous = buildToolActivity("open_url", {
			arguments: { url: "https://example.com/request" },
		});
		const detail = buildToolActivity(
			"open_url",
			{ result: { url: "https://example.com/response" } },
			previous,
		);
		expect(detail.sources?.map((source) => source.url)).toEqual([
			"https://example.com/request",
			"https://example.com/response",
		]);
	});

	test("reports app inventory counts and partial coverage from the real result shape", () => {
		expect(
			buildToolActivity("list_apps", {
				result_preview: JSON.stringify({
					status: "partial",
					returned: 3,
					total: 5,
					complete: false,
				}),
			}).outcome,
		).toBe("3 apps found · Partial inventory");
		expect(
			buildToolActivity("list_apps", {
				result: { returned: 1, complete: true },
			}).outcome,
		).toBe("1 app found");
	});

	test("describes mutating requests accurately", () => {
		expect(
			buildToolActivity("http_request", { arguments: { method: "post" } }),
		).toMatchObject({
			label: "Sending an HTTP request",
			method: "POST",
		});
	});

	test("handles absent or truncated provider previews without copying raw arguments", () => {
		for (const arguments_preview of [
			undefined,
			'{"headers":{"x-header":"private',
			"x".repeat(70_000),
		]) {
			expect(buildToolActivity("http_request", { arguments_preview })).toEqual({
				kind: "tool_activity",
				category: "web",
				label: "Making an HTTP request",
			});
		}
	});

	test("does not invent a method when arguments are unavailable or invalid", () => {
		for (const data of [
			{},
			{ arguments_preview: "{" },
			{ arguments: { method: "bad" } },
			{ arguments: { method: null } },
		]) {
			expect(buildToolActivity("http_request", data).method).toBeUndefined();
			expect(buildToolActivity("http_request", data).label).toBe(
				"Making an HTTP request",
			);
		}
		expect(buildToolActivity("http_request", { arguments: {} }).method).toBe(
			"GET",
		);
		const previous = buildToolActivity("http_request", {
			arguments: { method: "POST" },
		});
		expect(
			buildToolActivity("http_request", { status: "completed" }, previous)
				.method,
		).toBe("POST");
	});

	test("keeps stripped query destinations nonlinkable through completion", () => {
		const previous = buildToolActivity("http_request", {
			arguments: { url: "https://example.com/view?id=secret" },
		});
		const detail = buildToolActivity(
			"http_request",
			{ status: "completed" },
			previous,
		);
		expect(detail.sources).toEqual([
			{
				url: "https://example.com/view",
				hostname: "example.com",
				path: "/view",
				linkable: false,
			},
		]);
		expect(
			buildToolActivity("open_url", {
				arguments: { url: "https://example.com/license" },
			}).sources?.[0].linkable,
		).toBeUndefined();
	});

	test("humanizes known and unfamiliar tool labels", () => {
		expect(getToolActivityLabel("mcp__flowpilot__http_request")).toBe(
			"Making an HTTP request",
		);
		expect(getToolActivityLabel("custom_lookup")).toBe("Custom lookup");
		expect(getToolActivityLabel("")).toBe("Working");
	});

	test("keeps summaries short and strips raw JSON and credentials", () => {
		expect(
			toolActivityText('{"headers":{"Authorization":"secret"}}'),
		).toBeUndefined();
		expect(
			toolActivityText(
				"Fetching https://user:password@example.com/license?token=secret#private",
			),
		).toBe("Fetching https://example.com/license");
		expect(
			toolActivityText("Failed with Bearer secret and api_key=secret"),
		).toBe("Failed with Bearer [redacted] and [redacted]");
		expect(toolActivityText("A".repeat(1_000))?.length).toBeLessThanOrEqual(
			240,
		);
	});
});
