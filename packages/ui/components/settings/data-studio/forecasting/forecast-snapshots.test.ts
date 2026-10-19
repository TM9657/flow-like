import { describe, expect, it } from "bun:test";
import {
	publicWebOrigin,
	resolveHubWebOrigin,
} from "../../../../lib/public-web-origin";
import type {
	SavedStudioForecast,
	StudioForecastPayload,
} from "../../../../state/backend-state/query-state";
import {
	captureForecastRequest,
	forecastNeedsWebOrigin,
	forecastRequestFingerprint,
	forecastShareLink,
	previewForecastTargets,
	restoreForecastSetup,
	withoutSeriesTarget,
} from "./forecast-snapshots";

const request = (): StudioForecastPayload => ({
	profile_id: "alice-profile",
	model_bit_id: "forecast-model",
	sql: 'SELECT * FROM "sales" WHERE region = $region',
	params: { region: "north", filter: { threshold: 2, include: [1, 4] } },
	mapping: {
		timestamp: "day",
		series_id: "store_id",
		targets: ["sales"],
		past_covariates: ["visitors"],
		known_covariates: ["promotion"],
		frequency_ms: 86400000,
	},
	horizon: 24,
	quantile_levels: [0.2, 0.5, 0.8],
	future_sql: 'SELECT * FROM "promotions" WHERE region = $region',
	future_params: { region: "north" },
	backtest_horizon: 12,
	backtest_offset: 7,
	budget: {
		maximum_rows: 20000,
		maximum_bytes: 40000000,
		maximum_tensor_elements: 1000000,
	},
});

describe("saved forecast requests", () => {
	it("captures the completed run without retaining the user's profile or draft references", () => {
		const draft = request();
		const saved = captureForecastRequest(draft);
		const original = structuredClone(saved);
		draft.horizon = 36;
		draft.mapping.targets.push("returns");
		draft.quantile_levels?.push(0.9);
		(draft.params?.filter as { include: number[] }).include.push(7);
		expect(saved).toEqual(original);
		expect("profile_id" in saved).toBe(false);
		expect(saved.horizon).toBe(24);
		expect(forecastRequestFingerprint(saved)).not.toBe(
			forecastRequestFingerprint(draft),
		);
	});

	it("restores an inactive model, SQL parameters, custom quantiles and covariates without a preview", () => {
		const saved = captureForecastRequest(request());
		const restored = restoreForecastSetup(saved);
		expect(restored.request).toEqual(saved);
		expect(restored.request.model_bit_id).toBe("forecast-model");
		expect(restored.request.backtest_offset).toBe(7);
		expect(restored.covariates).toEqual({
			visitors: { enabled: true, role: "past" },
			promotion: { enabled: true, role: "known" },
		});
		expect(restored.extras).toEqual({
			params: saved.params,
			future_params: saved.future_params,
			quantile_levels: [0.2, 0.5, 0.8],
			budget: saved.budget,
		});
		const reopened = {
			...restored.request,
			...restored.extras,
			profile_id: "bob-profile",
		};
		expect(forecastRequestFingerprint(reopened)).toBe(
			forecastRequestFingerprint(saved),
		);
		restored.request.mapping.targets.push("returns");
		expect(saved.mapping.targets).toEqual(["sales"]);
	});

	it("treats native serialized defaults and omitted fields as the same setup", () => {
		const minimal: SavedStudioForecast["request"] = {
			model_bit_id: "model",
			sql: "SELECT * FROM sales",
			mapping: { timestamp: "day", targets: ["sales"] },
			horizon: 24,
		};
		const serialized = JSON.parse(
			JSON.stringify({
				...minimal,
				params: {},
				future_params: {},
				future_sql: null,
				quantile_levels: [],
				backtest_horizon: 0,
				backtest_offset: 0,
				mapping: {
					...minimal.mapping,
					series_id: null,
					frequency_ms: null,
					past_covariates: [],
					known_covariates: [],
				},
				budget: {
					maximum_rows: 100000,
					maximum_bytes: 256 * 1024 * 1024,
					maximum_tensor_elements: 32 * 1024 * 1024,
				},
			}),
		);
		expect(forecastRequestFingerprint(minimal)).toBe(
			forecastRequestFingerprint(serialized),
		);
		expect(
			forecastRequestFingerprint({ ...minimal, profile_id: "another-profile" }),
		).toBe(forecastRequestFingerprint(minimal));
		expect(
			forecastRequestFingerprint(restoreForecastSetup(serialized).request),
		).toBe(forecastRequestFingerprint(minimal));
	});

	it("ignores object key order while detecting parameter, quantile and budget changes", () => {
		const original = request();
		const reordered = request();
		reordered.params = {
			filter: { include: [1, 4], threshold: 2 },
			region: "north",
		};
		expect(forecastRequestFingerprint(reordered)).toBe(
			forecastRequestFingerprint(original),
		);
		for (const change of [
			{ params: { region: "south" } },
			{ future_params: { region: "south" } },
			{ quantile_levels: [0.1, 0.5, 0.9] },
			{ budget: { maximum_rows: 100 } },
			{ backtest_offset: 3 },
		])
			expect(forecastRequestFingerprint({ ...original, ...change })).not.toBe(
				forecastRequestFingerprint(original),
			);
	});
});

describe("saved forecast links and mappings", () => {
	it("replaces packaged desktop origins with the configured Hub web app", () => {
		for (const location of [
			"tauri://localhost/library/config/explore?id=project&scope=user",
			"http://tauri.localhost/library/config/explore?id=project&scope=user",
			"https://tauri.localhost/library/config/explore?id=project&scope=user",
		]) {
			expect(forecastNeedsWebOrigin(location)).toBe(true);
			expect(() => forecastShareLink(location, "saved", false)).toThrow(
				"public web app address",
			);
			const link = new URL(
				forecastShareLink(
					location,
					"saved",
					true,
					"https://studio.example.com",
				),
			);
			expect(link.origin).toBe("https://studio.example.com");
			expect(link.pathname).toBe("/library/config/explore");
			expect(link.searchParams.get("id")).toBe("project");
			expect(link.searchParams.get("forecast")).toBe("saved");
			expect(link.searchParams.get("scope")).toBe("user");
		}
	});
	it("resolves hosted and custom Hub app origins without a desktop navigation scheme", async () => {
		expect(await resolveHubWebOrigin("https://api.flow-like.com")).toBe(
			"https://app.flow-like.com",
		);
		const calls: string[] = [];
		const fetchHub = async (input: string) => {
			calls.push(String(input));
			return Response.json({ app: "studio.example.com" });
		};
		expect(
			await resolveHubWebOrigin("https://api.example.com/", fetchHub),
		).toBe("https://studio.example.com");
		expect(calls).toEqual(["https://api.example.com/api/v1"]);
		expect(publicWebOrigin("flow-like://store")).toBeUndefined();
		expect(
			publicWebOrigin("https://user:password@studio.example.com"),
		).toBeUndefined();
		expect(publicWebOrigin("https://studio.example.com/path")).toBeUndefined();
		await expect(
			resolveHubWebOrigin("https://api.example.com", async () =>
				Response.json({ app: null }),
			),
		).rejects.toThrow("no public HTTPS app address");
	});
	it("opens the project prediction instead of an unrelated table or overlay", () => {
		const link = new URL(
			forecastShareLink(
				"https://app.flow-like.com/data?app=project&scope=user&table=sales&overlay=old&view=query#chart",
				"saved / one",
				false,
			),
		);
		expect(link.pathname).toBe("/data");
		expect(link.searchParams.get("app")).toBe("project");
		expect(link.searchParams.get("view")).toBe("forecast");
		expect(link.searchParams.get("forecast")).toBe("saved / one");
		expect(link.searchParams.has("scope")).toBe(false);
		expect(link.searchParams.has("table")).toBe(false);
		expect(link.searchParams.has("overlay")).toBe(false);
		expect(link.hash).toBe("");
	});
	it("keeps personal predictions in personal scope", () => {
		const link = new URL(
			forecastShareLink(
				"https://app.flow-like.com/data?app=project",
				"personal",
				true,
			),
		);
		expect(link.searchParams.get("scope")).toBe("user");
		expect(link.searchParams.get("forecast")).toBe("personal");
	});
	it("removes a numeric series ID from targets and does not restore it on preview reload", () => {
		const targets = ["store_id", "sales"];
		expect(withoutSeriesTarget(targets, "store_id")).toEqual(["sales"]);
		expect(targets).toEqual(["store_id", "sales"]);
		expect(
			previewForecastTargets(
				["store_id"],
				["store_id", "day", "sales"],
				"day",
				"store_id",
			),
		).toEqual(["sales"]);
		expect(
			previewForecastTargets([], ["store_id", "day"], "day", "store_id"),
		).toEqual([]);
		expect(
			previewForecastTargets(
				["returns", "sales"],
				["store_id", "sales", "returns"],
				"day",
				"store_id",
			),
		).toEqual(["returns", "sales"]);
	});
});

it("retains timestamp interpretation in saved requests and invalidates a run when it changes", () => {
	const original: SavedStudioForecast["request"] = {
		model_bit_id: "model",
		sql: "SELECT * FROM history",
		mapping: {
			timestamp: "time",
			timestamp_format: "day_first",
			targets: ["sales"],
		},
		horizon: 12,
	};
	const restored = restoreForecastSetup(original);
	expect(restored.request.mapping.timestamp_format).toBe("day_first");
	expect(forecastRequestFingerprint(original)).not.toBe(
		forecastRequestFingerprint({
			...original,
			mapping: { ...original.mapping, timestamp_format: "month_first" },
		}),
	);
	expect(
		restoreForecastSetup({
			...original,
			mapping: { timestamp: "time", targets: ["sales"] },
		}).request.mapping.timestamp_format,
	).toBeUndefined();
});
