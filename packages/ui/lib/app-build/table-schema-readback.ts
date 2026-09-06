import type { z } from "zod";
import type { tableColumnSchema } from "./contract";

type Column = z.infer<typeof tableColumnSchema>;

export function schemaFields(raw: unknown): Record<string, unknown>[] {
	if (!raw || typeof raw !== "object") return [];
	const fields = Array.isArray(raw)
		? raw
		: (raw as Record<string, unknown>).fields;
	return Array.isArray(fields)
		? fields.filter(
				(field): field is Record<string, unknown> =>
					!!field && typeof field === "object" && !Array.isArray(field),
			)
		: [];
}

function scalarType(raw: unknown): string | undefined {
	if (typeof raw !== "string") return undefined;
	const aliases: Record<string, string> = {
		utf8: "string",
		largeutf8: "string",
		bool: "boolean",
	};
	return aliases[raw.toLowerCase()] ?? raw.toLowerCase();
}

function typeMatches(raw: unknown, expected: Column): boolean {
	if (
		expected.type !== "vector" &&
		scalarType(raw) === expected.type.toLowerCase()
	)
		return true;
	if (!raw || typeof raw !== "object" || Array.isArray(raw)) return false;
	const arrow = raw as Record<string, unknown>;
	if (expected.type === "timestamp:ms:UTC") {
		const timestamp = arrow.Timestamp;
		return (
			Array.isArray(timestamp) &&
			timestamp.length === 2 &&
			timestamp[0] === "Millisecond" &&
			timestamp[1] === "UTC"
		);
	}
	if (expected.type === "vector") {
		const vector = arrow.FixedSizeList;
		if (
			!Array.isArray(vector) ||
			vector.length !== 2 ||
			vector[1] !== expected.vector_size
		)
			return false;
		const child = vector[0];
		if (!child || typeof child !== "object" || Array.isArray(child))
			return false;
		return scalarType(child.data_type ?? child.type) === "float32";
	}
	return false;
}

/** Match Arrow's typed structure exactly; text in a field name cannot prove a vector width. */
export function tableSchemaIssues(
	fields: Record<string, unknown>[],
	columns: readonly Column[],
): string[] {
	const issues: string[] = [];
	for (const column of columns) {
		const field = fields.find((field) => field.name === column.name);
		if (!field) {
			issues.push(`Missing table column '${column.name}'.`);
			continue;
		}
		if (!typeMatches(field.data_type ?? field.type, column))
			issues.push(`Column '${column.name}' has a different type.`);
		if (column.nullable !== undefined && field.nullable !== column.nullable)
			issues.push(`Column '${column.name}' has different nullability.`);
	}
	return issues;
}
