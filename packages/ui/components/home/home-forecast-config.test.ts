import { describe, expect, it } from "bun:test";
import { HOME_WIDGET_PRESETS, createHomeWidget } from "./catalog";
import {
	normalizeHomeForecastConfig,
	updateHomeForecastSource,
} from "./home-forecast-config";
import { minimumHomeWidgetRows, normalizeHomeLayout } from "./home-layout";
import { parseHomeLayoutJson, serializeHomeLayout } from "./home-layout-json";

describe("home saved forecasts", () => {
	it("offers a data widget sized for the saved chart", () => {
		const preset = HOME_WIDGET_PRESETS.find(
			(item) => item.id === "saved-forecast",
		);
		expect(preset?.category).toBe("data");
		const widget = createHomeWidget("saved-forecast");
		expect(widget.type).toBe("saved-forecast");
		expect(widget.size).toEqual({ columns: 8, rows: 5 });
		expect(minimumHomeWidgetRows(widget)).toBe(4);
		expect(normalizeHomeForecastConfig(widget.config)).toEqual({
			appId: "",
			forecastId: "",
			scope: "project",
			view: "forecast",
		});
	});

	it("keeps only the saved reference and supported display options", () => {
		expect(
			normalizeHomeForecastConfig({
				appId: "sales-app",
				forecastId: "saved-run",
				scope: "personal",
				view: "evaluation",
				target: " Sales ",
				seriesId: "store-7",
				result: { history: ["private"] },
				created_by: "owner",
			}),
		).toEqual({
			appId: "sales-app",
			forecastId: "saved-run",
			scope: "personal",
			view: "evaluation",
			target: " Sales ",
			seriesId: "store-7",
		});
		expect(
			normalizeHomeForecastConfig({
				appId: 3,
				forecastId: {},
				scope: "other",
				view: false,
				target: [],
				seriesId: null,
			}),
		).toEqual({
			appId: "",
			forecastId: "",
			scope: "project",
			view: "forecast",
		});
	});

	it("clears prediction-specific choices when app, scope or prediction changes", () => {
		const config = normalizeHomeForecastConfig({
			appId: "sales-app",
			forecastId: "saved-run",
			scope: "project",
			view: "evaluation",
			target: "sales",
			seriesId: "store-7",
		});
		expect(updateHomeForecastSource(config, { appId: "another-app" })).toEqual({
			appId: "another-app",
			forecastId: "",
			scope: "project",
			view: "forecast",
		});
		expect(updateHomeForecastSource(config, { scope: "personal" })).toEqual({
			appId: "sales-app",
			forecastId: "",
			scope: "personal",
			view: "forecast",
		});
		expect(updateHomeForecastSource(config, { forecastId: "new-run" })).toEqual(
			{
				appId: "sales-app",
				forecastId: "new-run",
				scope: "project",
				view: "forecast",
			},
		);
		expect(updateHomeForecastSource(config, { appId: "sales-app" })).toEqual(
			config,
		);
		expect(config.forecastId).toBe("saved-run");
	});

	it("round-trips the selection through persisted home layouts", () => {
		const widget = createHomeWidget("saved-forecast");
		widget.config = {
			...normalizeHomeForecastConfig({
				appId: "app",
				forecastId: "saved",
				scope: "personal",
				view: "evaluation",
				target: "revenue",
				seriesId: "north",
			}),
		};
		const layout = normalizeHomeLayout({ version: 1, widgets: [widget] });
		expect(layout).not.toBeNull();
		if (!layout)
			throw new Error("Expected the saved forecast layout to normalize");
		const parsed = parseHomeLayoutJson(serializeHomeLayout(layout));
		expect(parsed.ok).toBe(true);
		if (parsed.ok)
			expect(parsed.layout.widgets[0].config).toEqual(widget.config);
		expect(Object.keys(widget.config).sort()).toEqual([
			"appId",
			"forecastId",
			"scope",
			"seriesId",
			"target",
			"view",
		]);
	});
});
