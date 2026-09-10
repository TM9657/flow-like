const MAX_FIELDS = 256;
const MAX_NODES = 2_048;
const MAX_DEPTH = 16;
const TIME_UNITS = new Set([
	"Second",
	"Millisecond",
	"Microsecond",
	"Nanosecond",
]);
const VARIANTS = [
	"Struct",
	"List",
	"LargeList",
	"ListView",
	"LargeListView",
	"FixedSizeList",
	"Map",
	"Dictionary",
	"Timestamp",
	"Time32",
	"Time64",
	"Duration",
	"Interval",
	"FixedSizeBinary",
	"Decimal32",
	"Decimal64",
	"Decimal128",
	"Decimal256",
	"Union",
	"RunEndEncoded",
];

function object(value: unknown): Record<string, unknown> | undefined {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: undefined;
}

function integer(value: unknown, min: number, max: number): value is number {
	return (
		typeof value === "number" &&
		Number.isInteger(value) &&
		value >= min &&
		value <= max
	);
}

/** Project Arrow's serde type structure without carrying nested Field metadata or defaults. */
export function projectWorkspaceTableFields(fields: unknown[]): {
	fields: unknown[];
	truncated: boolean;
} {
	let truncated = false;
	let visited = 0;
	const active = new Set<object>();
	const omitted = (): undefined => {
		truncated = true;
		return undefined;
	};
	const enter = (value: unknown, depth: number) => {
		if (++visited > MAX_NODES || depth > MAX_DEPTH) return false;
		return !value || typeof value !== "object" || !active.has(value);
	};
	const shortString = (value: unknown, limit: number) =>
		typeof value === "string" && value.length <= limit ? value : omitted();
	const fieldList = (value: unknown, depth: number): unknown[] | undefined => {
		if (!Array.isArray(value)) return omitted();
		if (value.length > MAX_FIELDS) truncated = true;
		const result: unknown[] = [];
		for (const child of value.slice(0, MAX_FIELDS)) {
			if (visited >= MAX_NODES) {
				truncated = true;
				break;
			}
			const projected = field(child, depth);
			if (projected) result.push(projected);
		}
		return result;
	};
	const field = (
		value: unknown,
		depth: number,
	): Record<string, unknown> | undefined => {
		const raw = object(value);
		if (!raw || !enter(raw, depth)) return omitted();
		active.add(raw);
		try {
			const result: Record<string, unknown> = {};
			if (raw.name !== undefined) {
				const name = shortString(raw.name, 256);
				if (name !== undefined) result.name = name;
			}
			if (typeof raw.nullable === "boolean") result.nullable = raw.nullable;
			else if (raw.nullable !== undefined) omitted();
			for (const key of ["data_type", "type"]) {
				if (raw[key] === undefined) continue;
				const projected = type(raw[key], depth + 1);
				if (projected !== undefined) result[key] = projected;
			}
			if (raw.data_type === undefined && raw.type === undefined) omitted();
			return Object.keys(result).length ? result : omitted();
		} finally {
			active.delete(raw);
		}
	};
	const type = (value: unknown, depth: number): unknown => {
		if (!enter(value, depth)) return omitted();
		if (typeof value === "string") return shortString(value, 128);
		const raw = object(value);
		if (!raw) return omitted();
		const variants = VARIANTS.filter((variant) => Object.hasOwn(raw, variant));
		if (variants.length !== 1) return omitted();
		const variant = variants[0];
		const parameter = raw[variant];
		active.add(raw);
		try {
			let projected: unknown;
			if (variant === "Struct") projected = fieldList(parameter, depth + 1);
			else if (
				["List", "LargeList", "ListView", "LargeListView"].includes(variant)
			)
				projected = field(parameter, depth + 1);
			else if (["Time32", "Time64", "Duration"].includes(variant)) {
				if (typeof parameter === "string" && TIME_UNITS.has(parameter))
					projected = parameter;
			} else if (variant === "Interval") {
				if (
					typeof parameter === "string" &&
					["YearMonth", "DayTime", "MonthDayNano"].includes(parameter)
				)
					projected = parameter;
			} else if (variant === "FixedSizeBinary") {
				if (integer(parameter, 0, 2_147_483_647)) projected = parameter;
			} else if (Array.isArray(parameter) && parameter.length === 2) {
				const [first, second] = parameter;
				if (variant === "Timestamp") {
					if (
						typeof first === "string" &&
						TIME_UNITS.has(first) &&
						(second === null ||
							(typeof second === "string" && second.length <= 128))
					)
						projected = [first, second];
				} else if (variant.startsWith("Decimal")) {
					if (integer(first, 0, 255) && integer(second, -128, 127))
						projected = [first, second];
				} else if (variant === "FixedSizeList" || variant === "Map") {
					if (
						variant === "Map"
							? typeof second === "boolean"
							: integer(second, 0, 2_147_483_647)
					) {
						const child = field(first, depth + 1);
						if (child) projected = [child, second];
					}
				} else if (variant === "Dictionary" || variant === "RunEndEncoded") {
					const project = variant === "Dictionary" ? type : field;
					const left = project(first, depth + 1);
					const right = project(second, depth + 1);
					if (left !== undefined && right !== undefined)
						projected = [left, right];
				} else if (
					variant === "Union" &&
					Array.isArray(first) &&
					(second === "Dense" || second === "Sparse")
				) {
					const members: unknown[] = [];
					if (first.length > MAX_FIELDS) truncated = true;
					for (const member of first.slice(0, MAX_FIELDS)) {
						if (visited >= MAX_NODES) {
							truncated = true;
							break;
						}
						if (
							!Array.isArray(member) ||
							member.length !== 2 ||
							!integer(member[0], -128, 127)
						) {
							omitted();
							continue;
						}
						const child = field(member[1], depth + 1);
						if (child) members.push([member[0], child]);
					}
					projected = [members, second];
				}
			}
			return projected === undefined ? omitted() : { [variant]: projected };
		} finally {
			active.delete(raw);
		}
	};
	return { fields: fieldList(fields, 0) ?? [], truncated };
}
