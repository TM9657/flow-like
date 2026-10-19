import { describe, expect, test } from "vitest";
import type { BoundValue, ForecastChartComponent } from "../types";
import { resolveForecastChartProps } from "./forecast-chart-props";

function resolve(value: BoundValue): unknown {
	if ("literalString" in value) return value.literalString;
	if ("literalBool" in value) return value.literalBool;
	if ("literalNumber" in value) return value.literalNumber;
	if ("path" in value) {
		const data: Record<string, unknown> = {
			"$.saved": "prediction-a",
			"$.personal": true,
		};
		return data[value.path];
	}
	return undefined;
}

const component: ForecastChartComponent = {
	id: "saved-outlook",
	type: "forecastChart",
	forecastId: { path: "$.saved" },
};

describe("saved forecast component bindings", () => {
	test("uses explicit, runtime then action-context project identity", () => {
		expect(
			resolveForecastChartProps(component, resolve, "runtime", "context").appId,
		).toBe("runtime");
		expect(
			resolveForecastChartProps(component, resolve, undefined, "context").appId,
		).toBe("context");
		expect(
			resolveForecastChartProps(
				{ ...component, appId: { literalString: "owning-project" } },
				resolve,
				"runtime",
			).appId,
		).toBe("owning-project");
		expect(
			resolveForecastChartProps(
				{ ...component, appId: { literalString: " " } },
				resolve,
				"runtime",
			).appId,
		).toBe("runtime");
	});

	test("resolves saved selection and scope without treating string values as booleans", () => {
		const props = resolveForecastChartProps(
			{
				...component,
				userScoped: { path: "$.personal" },
				view: { literalString: "evaluation" },
				target: { literalString: "sales" },
				seriesId: { literalString: "region-a" },
				showMetrics: { literalBool: false },
			},
			resolve,
			"project",
		);
		expect(props).toMatchObject({
			appId: "project",
			forecastId: "prediction-a",
			userScoped: true,
			view: "evaluation",
			target: "sales",
			seriesId: "region-a",
			showMetrics: false,
		});
		const malformed = resolveForecastChartProps(
			{
				...component,
				userScoped: { literalString: "false" },
				view: { literalString: "unknown" },
				showMetrics: { literalString: "true" },
			},
			resolve,
		);
		expect(malformed.userScoped).toBe(false);
		expect(malformed.view).toBe("forecast");
		expect(malformed.showMetrics).toBeUndefined();
	});

	test("bounds numeric chart height and leaves missing bindings unselected", () => {
		for (const [input, expected] of [
			[20, 120],
			[320, 320],
			[5000, 1200],
		]) {
			expect(
				resolveForecastChartProps(
					{ ...component, height: { literalNumber: input } },
					resolve,
				).height,
			).toBe(expected);
		}
		expect(
			resolveForecastChartProps(
				{ ...component, height: { literalNumber: Number.NaN } },
				resolve,
			).height,
		).toBeUndefined();
		expect(
			resolveForecastChartProps(
				{ ...component, forecastId: { path: "$.missing" } },
				resolve,
			).forecastId,
		).toBe("");
	});
});
