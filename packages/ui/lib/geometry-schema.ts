import {
	GEOMETRY_KINDS,
	type GeometryKind,
	geometryMarker,
	normalizeGeometryValue,
} from "./geometry";
import { IValueType } from "./schema/flow/pin";

type Schema = Record<string, unknown>;
const object = (value: unknown): value is Schema =>
	!!value && typeof value === "object" && !Array.isArray(value);

export function resolveGeometryFieldSchema(
	schema: unknown,
	root: unknown = schema,
	seen = new Set<string>(),
): Schema | undefined {
	if (!object(schema)) return undefined;
	if (typeof schema.$ref === "string" && schema.$ref.startsWith("#/")) {
		if (seen.has(schema.$ref)) return undefined;
		seen.add(schema.$ref);
		let target = root;
		for (const segment of schema.$ref.slice(2).split("/")) {
			target = object(target)
				? target[segment.replace(/~1/g, "/").replace(/~0/g, "~")]
				: undefined;
		}
		return resolveGeometryFieldSchema(target, root, seen);
	}
	if (schema["x-flow-like-type"] === "geometry") return schema;
	const union = schema.anyOf ?? schema.oneOf ?? schema.allOf;
	if (Array.isArray(union)) {
		for (const branch of union) {
			const resolved = resolveGeometryFieldSchema(branch, root, new Set(seen));
			if (resolved && resolved.type !== "null") return resolved;
		}
	}
	return schema;
}

/** Expanded interface schemas retain an explicit annotation; ordinary objects stay Struct. */
export function geometrySchemaField(
	schema: unknown,
	root: unknown = schema,
	depth = 0,
): { schema: string | null; valueType: IValueType } | undefined {
	if (depth > 32) return undefined;
	const resolved = resolveGeometryFieldSchema(schema, root);
	if (!resolved) return undefined;
	if (resolved["x-flow-like-type"] === "geometry") {
		const kind = resolved["x-geometry"];
		if (kind !== undefined && !GEOMETRY_KINDS.includes(kind as GeometryKind))
			return undefined;
		return {
			schema: geometryMarker(kind as GeometryKind | undefined),
			valueType: IValueType.Normal,
		};
	}
	if (resolved.type === "array") {
		const item = geometrySchemaField(resolved.items, root, depth + 1);
		if (item?.valueType === IValueType.Normal)
			return {
				...item,
				valueType: resolved.uniqueItems ? IValueType.HashSet : IValueType.Array,
			};
	}
	if (resolved.type === "object" && object(resolved.additionalProperties)) {
		const item = geometrySchemaField(
			resolved.additionalProperties,
			root,
			depth + 1,
		);
		if (item?.valueType === IValueType.Normal)
			return { ...item, valueType: IValueType.HashMap };
	}
	return undefined;
}

/** Validates annotated Geometry fields without applying Geometry rules to other Struct data. */
export function normalizeSchemaGeometryValues(
	value: unknown,
	schema: unknown,
	root: unknown = schema,
	allowUnset = true,
	depth = 0,
): unknown {
	if (depth > 32) return value;
	const field = geometrySchemaField(schema, root);
	if (field) return normalizeGeometryValue(value, { ...field, allowUnset });
	const resolved = resolveGeometryFieldSchema(schema, root);
	if (!resolved || value == null) return value;
	if (Array.isArray(value) && resolved.items)
		return value.map((item) =>
			normalizeSchemaGeometryValues(
				item,
				resolved.items,
				root,
				allowUnset,
				depth + 1,
			),
		);
	if (object(value)) {
		const result = { ...value };
		const properties = object(resolved.properties) ? resolved.properties : {};
		for (const [key, property] of Object.entries(properties)) {
			if (
				!(key in result) &&
				(allowUnset ||
					!Array.isArray(resolved.required) ||
					!resolved.required.includes(key))
			)
				continue;
			result[key] = normalizeSchemaGeometryValues(
				result[key],
				property,
				root,
				allowUnset ||
					!Array.isArray(resolved.required) ||
					!resolved.required.includes(key),
				depth + 1,
			);
		}
		if (object(resolved.additionalProperties))
			for (const key of Object.keys(result)) {
				if (!(key in properties))
					result[key] = normalizeSchemaGeometryValues(
						result[key],
						resolved.additionalProperties,
						root,
						allowUnset,
						depth + 1,
					);
			}
		return result;
	}
	return value;
}
