import {
	type ChartInput,
	hasRenderableNivoData,
	parseChartData,
	toNivoData,
	toPlotlyData,
} from "./chart-data-parser";
import { parsePartialJson } from "./partial-json";

type ChartLanguage = "nivo" | "plotly";
type JsonObject = Record<string, unknown>;

const object = (value: unknown): JsonObject | null =>
	value !== null && typeof value === "object" && !Array.isArray(value)
		? (value as JsonObject)
		: null;
const number = (value: unknown): value is number =>
	typeof value === "number" && Number.isFinite(value);
const label = (value: unknown): value is string | number =>
	(typeof value === "string" && value.length > 0) || number(value);

function hasData(input: ChartInput, language: ChartLanguage): boolean {
	try {
		if (input.mode === "csv") return Boolean(input.csvData?.rows.length);
		if (language === "nivo") {
			const { data } = toNivoData(input);
			return (
				hasRenderableNivoData(data) &&
				(Array.isArray(data) || Object.keys(object(data) ?? {}).length > 0)
			);
		}
		return toPlotlyData(input).data.some((entry) => {
			const trace = object(entry);
			return (
				trace &&
				Object.values(trace).some((value) =>
					Array.isArray(value) ? value.length > 0 : object(value) !== null,
				)
			);
		});
	} catch {
		return false;
	}
}

function partialNivo(root: JsonObject): ChartInput | null {
	const chartType = root.chartType;
	if (typeof chartType !== "string" || !Array.isArray(root.data)) return null;
	let data: unknown[] = [];
	const props: JsonObject = { chartType };
	if (chartType === "bar" || chartType === "radar") {
		const indexBy = root.indexBy;
		const keys = Array.isArray(root.keys)
			? root.keys.filter((key): key is string => typeof key === "string")
			: [];
		if (typeof indexBy !== "string" || keys.length === 0) return null;
		props.indexBy = indexBy;
		props.keys = keys;
		data = root.data.filter((row) => {
			const item = object(row);
			return (
				item && label(item[indexBy]) && keys.every((key) => number(item[key]))
			);
		});
	} else if (["line", "scatter", "bump", "areaBump"].includes(chartType)) {
		data = root.data.flatMap((entry) => {
			const series = object(entry);
			if (!series || !label(series.id) || !Array.isArray(series.data))
				return [];
			const points = series.data.filter((entry) => {
				const point = object(entry);
				return point && label(point.x) && (number(point.y) || point.y === null);
			});
			return points.length > 0 ? [{ id: series.id, data: points }] : [];
		});
	} else if (chartType === "pie" || chartType === "funnel") {
		data = root.data.filter((entry) => {
			const item = object(entry);
			return item && label(item.id) && number(item.value);
		});
	}
	if (data.length === 0) return null;
	if (root.layout === "horizontal" || root.layout === "vertical")
		props.layout = root.layout;
	// Optional nested settings can still be incomplete. Renderer defaults apply
	// until the complete JSON payload supplies those settings.
	return { mode: "json", config: {}, jsonData: { ...props, data } };
}

function partialPlotly(root: JsonObject): ChartInput | null {
	if (!Array.isArray(root.data)) return null;
	const data = root.data.flatMap((entry) => {
		const trace = object(entry);
		if (!trace || typeof trace.type !== "string") return [];
		const result: JsonObject = { type: trace.type };
		for (const key of ["name", "mode", "orientation", "stackgroup", "fill"])
			if (typeof trace[key] === "string") result[key] = trace[key];
		if (trace.type === "bar" || trace.type === "scatter") {
			if (!Array.isArray(trace.x) || !Array.isArray(trace.y)) return [];
			const count = Math.min(trace.x.length, trace.y.length);
			const pairs = Array.from({ length: count }, (_, index) => [
				(trace.x as unknown[])[index],
				(trace.y as unknown[])[index],
			]).filter(
				([x, y]) => (label(x) || x === null) && (label(y) || y === null),
			);
			if (!pairs.some(([x, y]) => label(x) && label(y))) return [];
			result.x = pairs.map(([x]) => x);
			result.y = pairs.map(([, y]) => y);
		} else if (trace.type === "pie") {
			if (!Array.isArray(trace.labels) || !Array.isArray(trace.values))
				return [];
			const count = Math.min(trace.labels.length, trace.values.length);
			const pairs = Array.from({ length: count }, (_, index) => [
				(trace.labels as unknown[])[index],
				(trace.values as unknown[])[index],
			]).filter(([name, value]) => label(name) && number(value));
			if (pairs.length === 0) return [];
			result.labels = pairs.map(([name]) => name);
			result.values = pairs.map(([, value]) => value);
		} else return [];
		return [result];
	});
	return data.length > 0
		? { mode: "json", config: {}, jsonData: { data } }
		: null;
}

/** Commit complete data values while the surrounding code fence is growing. */
export function parseStreamingChartData(
	content: string,
	language: ChartLanguage,
): ChartInput | null {
	const trimmed = content.trimStart();
	if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
		try {
			const complete = parseChartData(content, language);
			return hasData(complete, language) ? complete : null;
		} catch {
			const root = object(
				parsePartialJson(content, { allowPartialArrayItems: true }),
			);
			return root
				? language === "nivo"
					? partialNivo(root)
					: partialPlotly(root)
				: null;
		}
	}

	// The last CSV value may be an unfinished number. It becomes a value only
	// after its newline arrives; the settled parser includes the final row.
	const lastNewline = content.lastIndexOf("\n");
	if (lastNewline < 0) return null;
	const completeLines = content.slice(0, lastNewline + 1);
	const separator = completeLines.match(/(?:^|\n)---\r?\n/);
	const csv = separator
		? completeLines.slice((separator.index ?? 0) + separator[0].length)
		: completeLines;
	const lines = csv.split(/\r?\n/).filter((line) => line.trim());
	const headers = lines[0]?.split(",");
	if (
		!headers ||
		headers.length < 2 ||
		headers.some((header) => !header.trim())
	)
		return null;
	const rows = lines.slice(1).filter((line) => {
		const cells = line.split(",").map((cell) => cell.trim());
		return (
			cells.length === headers.length &&
			cells.every(Boolean) &&
			cells.slice(1).every((cell) => Number.isFinite(Number(cell)))
		);
	});
	if (rows.length === 0) return null;
	const prefix = separator
		? completeLines.slice(0, (separator.index ?? 0) + separator[0].length)
		: "";
	try {
		return parseChartData(`${prefix}${lines[0]}\n${rows.join("\n")}`, language);
	} catch {
		return null;
	}
}

export interface ChartSnapshot {
	source: string;
	language: ChartLanguage;
	input: ChartInput | null;
	key: string;
}

/** Keep the mounted chart stable between committed rows and reset it on edits. */
export function advanceChartSnapshot(
	previous: ChartSnapshot | null,
	content: string,
	language: ChartLanguage,
	isStreaming: boolean,
): ChartSnapshot {
	const prior =
		previous?.language === language && content.startsWith(previous.source)
			? previous
			: null;
	let input: ChartInput | null = null;
	try {
		input = isStreaming
			? parseStreamingChartData(content, language)
			: parseChartData(content, language);
	} catch {
		// Invalid settled data uses the source/debug fallback, never an older chart.
	}
	if (!input && isStreaming) input = prior?.input ?? null;
	const key = input ? JSON.stringify(input) : "";
	return {
		source: content,
		language,
		input: key === prior?.key ? prior.input : input,
		key,
	};
}

export function streamingChartHeight(
	content: string,
	input: ChartInput | null,
): number {
	const configured =
		input?.config.height ??
		Number(/(?:^|\n)height:\s*(\d+(?:\.\d+)?)\s*\r?\n/.exec(content)?.[1]);
	return Number.isFinite(configured) && configured > 0 ? configured : 350;
}
