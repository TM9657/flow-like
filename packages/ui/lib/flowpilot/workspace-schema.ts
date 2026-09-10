const MAX_SCHEMA_BYTES = 65_536;
const MAX_SCHEMA_NODES = 128;
const MAX_SCHEMA_DEPTH = 8;
const encoder = new TextEncoder();
const JSON_TYPES = new Set([
	"array",
	"boolean",
	"integer",
	"null",
	"number",
	"object",
	"string",
]);
const OMITTED_ANNOTATIONS = new Set([
	"default",
	"examples",
	"example",
	"$comment",
	"$schema",
	"readOnly",
	"writeOnly",
	"deprecated",
]);

export interface WorkspaceSchemaProjection {
	schema: unknown;
	truncated: boolean;
}

/**
 * Keep schema structure for discovery without exposing defaults or example values.
 * Local JSON Pointer refs are rewritten into reachable definitions. This projection
 * is reference context; omitted validation constraints set truncated.
 */
export function projectWorkspaceSchema(
	value: unknown,
): WorkspaceSchemaProjection {
	if (value === undefined || value === null)
		return { schema: null, truncated: false };
	let source: unknown;
	try {
		const serialized =
			typeof value === "string" ? value : JSON.stringify(value);
		if (
			typeof serialized !== "string" ||
			serialized.length > MAX_SCHEMA_BYTES ||
			encoder.encode(serialized).length > MAX_SCHEMA_BYTES
		) {
			return { schema: null, truncated: true };
		}
		source = JSON.parse(serialized);
	} catch {
		return { schema: null, truncated: true };
	}

	let truncated = false;
	let visited = 0;
	const references = new Map<string, string>();
	const definitions: Record<string, unknown> = Object.create(null);
	const object = (item: unknown): item is Record<string, unknown> =>
		item !== null && typeof item === "object" && !Array.isArray(item);
	const resolveReference = (reference: string): unknown => {
		if (reference === "#") return source;
		if (!reference.startsWith("#/")) return undefined;
		try {
			const pointer = decodeURIComponent(reference.slice(2));
			let resolved = source;
			let position: "schema" | "map" | "array" = "schema";
			for (const segment of pointer.split("/")) {
				if (/~(?:[^01]|$)/.test(segment)) return undefined;
				const key = segment.replace(/~1/g, "/").replace(/~0/g, "~");
				if (
					(!object(resolved) && !Array.isArray(resolved)) ||
					!Object.hasOwn(resolved, key)
				) {
					return undefined;
				}
				if (position === "schema") {
					if (
						[
							"properties",
							"$defs",
							"definitions",
							"patternProperties",
							"dependentSchemas",
						].includes(key)
					)
						position = "map";
					else if (["allOf", "anyOf", "oneOf", "prefixItems"].includes(key))
						position = "array";
					else if (
						[
							"items",
							"additionalProperties",
							"contains",
							"not",
							"if",
							"then",
							"else",
							"propertyNames",
							"unevaluatedItems",
							"unevaluatedProperties",
						].includes(key)
					) {
						position = Array.isArray((resolved as Record<string, unknown>)[key])
							? "array"
							: "schema";
					} else return undefined;
				} else if (position === "array") {
					if (!/^(0|[1-9]\d*)$/.test(key)) return undefined;
					position = "schema";
				} else position = "schema";
				resolved = (resolved as Record<string, unknown>)[key];
			}
			return position === "schema" ? resolved : undefined;
		} catch {
			return undefined;
		}
	};
	const project = (item: unknown, depth: number): unknown => {
		if (visited >= MAX_SCHEMA_NODES || depth > MAX_SCHEMA_DEPTH) {
			truncated = true;
			return {};
		}
		visited++;
		if (typeof item === "boolean") return item;
		if (!object(item)) {
			truncated = true;
			return {};
		}
		const projected: Record<string, unknown> = Object.create(null);
		const projectList = (items: unknown[]) => {
			const result: unknown[] = [];
			for (const child of items) {
				if (visited >= MAX_SCHEMA_NODES) {
					truncated = true;
					break;
				}
				result.push(project(child, depth + 1));
			}
			return result;
		};
		for (const [key, field] of Object.entries(item)) {
			if (
				OMITTED_ANNOTATIONS.has(key) ||
				key === "$defs" ||
				key === "definitions"
			)
				continue;
			if (key === "type") {
				if (typeof field === "string" && JSON_TYPES.has(field))
					projected.type = field;
				else if (Array.isArray(field)) {
					const valid = field.filter(
						(type) => typeof type === "string" && JSON_TYPES.has(type),
					);
					if (valid.length !== field.length || valid.length > JSON_TYPES.size)
						truncated = true;
					projected.type = [...new Set(valid)].slice(0, JSON_TYPES.size);
				} else truncated = true;
			} else if (key === "title" || key === "description" || key === "format") {
				if (typeof field === "string") projected[key] = field;
				else truncated = true;
			} else if (key === "required") {
				if (Array.isArray(field)) {
					const names = field.filter((name) => typeof name === "string");
					if (names.length !== field.length || names.length > MAX_SCHEMA_NODES)
						truncated = true;
					projected.required = names.slice(0, MAX_SCHEMA_NODES);
				} else truncated = true;
			} else if (key === "properties") {
				if (!object(field)) {
					truncated = true;
					continue;
				}
				const properties: Record<string, unknown> = Object.create(null);
				for (const [name, child] of Object.entries(field)) {
					if (visited >= MAX_SCHEMA_NODES) {
						truncated = true;
						break;
					}
					properties[name] = project(child, depth + 1);
				}
				projected.properties = properties;
			} else if (key === "items" || key === "additionalProperties") {
				projected[key] =
					key === "items" && Array.isArray(field)
						? projectList(field)
						: project(field, depth + 1);
			} else if (key === "allOf" || key === "anyOf" || key === "oneOf") {
				if (Array.isArray(field)) projected[key] = projectList(field);
				else truncated = true;
			} else if (key === "$ref") {
				if (typeof field !== "string") {
					truncated = true;
					continue;
				}
				const target = resolveReference(field);
				if (!object(target) && typeof target !== "boolean") {
					truncated = true;
					continue;
				}
				let name = references.get(field);
				if (!name) {
					name = `ref_${references.size}`;
					references.set(field, name);
					definitions[name] = project(target, depth + 1);
				}
				projected.$ref = `#/$defs/${name}`;
			} else truncated = true;
		}
		return projected;
	};
	const schema = project(source, 0);
	if (object(schema) && Object.keys(definitions).length > 0)
		schema.$defs = definitions;
	return { schema, truncated };
}
