import { describe, expect, it } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { StudioBacktestMetric } from "../../../../state/backend-state/query-state";
import { ForecastMetrics } from "./forecast-metrics";

function metric(): StudioBacktestMetric {
	return {
		series_id: "north",
		target: "sales",
		observations: 4,
		mae: 0,
		rmse: 2,
		wape: 125,
		mape: 75,
	};
}

describe("forecast metrics", () => {
	it("shows measured percentage errors without converting them into an accuracy score", () => {
		const html = renderToStaticMarkup(<ForecastMetrics metrics={metric()} />);
		expect(html).toContain("125%");
		expect(html).toContain("75%");
		expect(html).not.toContain("-25%");
		expect(html).not.toContain("Unavailable");
		expect(html).toContain("4 observed values");
		expect(html).toContain("metric&#x27;s units");
		expect(html).toContain("Missing actual values are excluded");
	});
	it("preserves unavailable errors and explains zero-actual limits", () => {
		const html = renderToStaticMarkup(
			<ForecastMetrics
				metrics={{
					...metric(),
					observations: 0,
					mae: null,
					rmse: null,
					wape: null,
					mape: null,
				}}
				compact
			/>,
		);
		expect(html.match(/>Unavailable</g)).toHaveLength(4);
		expect(html).toContain("if any observed actual is zero");
		expect(html).toContain("when that total is zero");
		expect(html).not.toContain("0%");
	});
	it("does not invent WAPE for older snapshots", () => {
		const html = renderToStaticMarkup(
			<ForecastMetrics metrics={{ ...metric(), wape: undefined }} />,
		);
		expect(html.match(/>Unavailable</g)).toHaveLength(1);
		expect(html).toContain("WAPE was not included in this saved result");
	});
});
