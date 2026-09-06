"use client";

import { useTranslation } from "@flow-like/locales";
import {
	Suspense,
	lazy,
	useEffect,
	useId,
	useMemo,
	useRef,
	useState,
} from "react";
import {
	GEOMETRY_KINDS,
	type GeometryKind,
	geometryKindFromSchema,
	geometryMarker,
	parseGeometryText,
} from "../../../lib/geometry";
import { IValueType } from "../../../lib/schema/flow/pin";
import type { IVariable } from "../../../lib/schema/flow/variable";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../lib/uint8";
import { Button } from "../../ui/button";
import { Input } from "../../ui/input";
import { Label } from "../../ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../ui/select";
import { Textarea } from "../../ui/textarea";

const GeometryPreview = lazy(() => import("../../ui/geometry-preview"));

export function GeometrySubtypeSelect({
	schema,
	refs,
	onChange,
	disabled,
}: Readonly<{
	schema?: string | null;
	refs?: Record<string, string>;
	onChange: (schema: string | null) => void;
	disabled?: boolean;
}>) {
	const { t } = useTranslation("flow");
	const id = useId();
	let kind = "any";
	let error: string | null = null;
	try {
		kind = geometryKindFromSchema(schema, refs) ?? "any";
	} catch (e) {
		error = String(e);
	}
	return (
		<div className="grid gap-2">
			<Label htmlFor={id}>{t("geometrySubtype", "Geometry subtype")}</Label>
			<Select
				value={kind}
				disabled={disabled}
				onValueChange={(kind) =>
					onChange(
						geometryMarker(kind === "any" ? null : (kind as GeometryKind)),
					)
				}
			>
				<SelectTrigger id={id}>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					<SelectItem value="any">
						{t("anyGeometry", "Any geometry")}
					</SelectItem>
					{GEOMETRY_KINDS.map((kind) => (
						<SelectItem key={kind} value={kind}>
							{kind}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
			{error && (
				<p role="alert" className="text-xs text-destructive">
					{error}
				</p>
			)}
		</div>
	);
}

export function GeometryValueInput({
	value,
	onChange,
	schema,
	refs,
	valueType = IValueType.Normal,
	disabled,
	allowUnset = true,
	preview = true,
	secret = false,
}: Readonly<{
	value: unknown;
	onChange: (value: unknown, valid: boolean) => void;
	schema?: string | null;
	refs?: Record<string, string>;
	valueType?: IValueType;
	disabled?: boolean;
	allowUnset?: boolean;
	preview?: boolean;
	secret?: boolean;
}>) {
	const { t } = useTranslation("flow");
	const id = useId();
	const [text, setText] = useState(() =>
		value == null ? "" : JSON.stringify(value, null, 2),
	);
	const [focused, setFocused] = useState(false);
	const [revealed, setRevealed] = useState(false);
	const previousValue = useRef(value);
	useEffect(() => {
		if (!focused && previousValue.current !== value) {
			setText(value == null ? "" : JSON.stringify(value, null, 2));
		}
		previousValue.current = value;
	}, [value, focused]);
	const parsed = useMemo(() => {
		try {
			return {
				value: parseGeometryText(text, { schema, refs, valueType, allowUnset }),
				error: null,
			};
		} catch (e) {
			return { value: null, error: e instanceof Error ? e.message : String(e) };
		}
	}, [text, schema, refs, valueType, allowUnset]);
	return (
		<div className="grid w-full gap-2">
			<Label htmlFor={id}>{t("geometryGeoJson", "GeoJSON geometry")}</Label>
			<p className="text-xs text-muted-foreground">
				{allowUnset
					? t(
							"geometryCoordinateOrder",
							"WGS 84 coordinates: longitude, latitude in degrees. Leave empty to unset.",
						)
					: t(
							"geometryRequiredCoordinateOrder",
							"WGS 84 coordinates: longitude, latitude in degrees. A geometry is required.",
						)}
			</p>
			{secret && (
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={() => setRevealed(!revealed)}
				>
					{revealed
						? t("hideGeometryValue", "Hide geometry value")
						: t("showGeometryValue", "Show geometry value")}
				</Button>
			)}
			{secret && !revealed ? (
				<Input
					id={id}
					type="password"
					value={text}
					readOnly
					disabled={disabled}
				/>
			) : (
				<Textarea
					id={id}
					disabled={disabled}
					value={text}
					rows={8}
					className="font-mono text-xs"
					spellCheck={false}
					aria-invalid={Boolean(parsed.error)}
					aria-describedby={parsed.error ? `${id}-error` : undefined}
					onFocus={() => setFocused(true)}
					onBlur={() => setFocused(false)}
					onChange={(event) => {
						const next = event.target.value;
						setText(next);
						try {
							onChange(
								parseGeometryText(next, {
									schema,
									refs,
									valueType,
									allowUnset,
								}),
								true,
							);
						} catch {
							onChange(undefined, false);
						}
					}}
				/>
			)}
			{parsed.error && (
				<p id={`${id}-error`} role="alert" className="text-xs text-destructive">
					{parsed.error}
				</p>
			)}
			{preview &&
				(!secret || revealed) &&
				!parsed.error &&
				parsed.value != null && (
					<Suspense fallback={null}>
						<GeometryPreview value={parsed.value} valueType={valueType} />
					</Suspense>
				)}
		</div>
	);
}

export function GeometryVariable({
	variable,
	onChange,
	disabled,
	refs,
}: Readonly<{
	variable: IVariable;
	onChange: (variable: IVariable) => void;
	disabled?: boolean;
	refs?: Record<string, string>;
}>) {
	return (
		<GeometryValueInput
			disabled={disabled}
			schema={variable.schema}
			refs={refs}
			secret={variable.secret}
			valueType={variable.value_type}
			value={parseUint8ArrayToJson(variable.default_value)}
			onChange={(value, valid) => {
				if (valid)
					onChange({
						...variable,
						default_value:
							value == null ? null : convertJsonToUint8Array(value),
					});
			}}
		/>
	);
}
