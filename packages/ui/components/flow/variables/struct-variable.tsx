"use client";

import { BracesIcon, FormInputIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "../../../components/ui/button";
import { Checkbox } from "../../../components/ui/checkbox";
import { Input } from "../../../components/ui/input";
import { Label } from "../../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../../components/ui/select";
import { Textarea } from "../../../components/ui/textarea";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../../../components/ui/tooltip";
import type { IVariable } from "../../../lib/schema/flow/variable";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../lib/uint8";
import { cn } from "../../../lib/utils";

interface SchemaProperty {
	type?: string;
	description?: string;
	default?: unknown;
	enum?: string[];
	items?: SchemaProperty;
	properties?: Record<string, SchemaProperty>;
	required?: string[];
}

interface JsonSchema {
	type?: string;
	properties?: Record<string, SchemaProperty>;
	required?: string[];
	description?: string;
}

const EMPTY_STRING_HASH = "16248035215404677707";

const resolveRef = (
	value: string | undefined | null,
	refs: Record<string, string> | undefined,
): string => {
	if (!value) return "";
	if (value === EMPTY_STRING_HASH) return "";
	const resolved = refs?.[value];
	return resolved ?? value;
};

const parseSchema = (
	schemaStr: string | null | undefined,
	refs: Record<string, string> | undefined,
): JsonSchema | null => {
	if (!schemaStr) return null;
	const resolved = resolveRef(schemaStr, refs);
	if (!resolved) return null;
	try {
		return JSON.parse(resolved) as JsonSchema;
	} catch {
		return null;
	}
};

const getDefaultFromSchema = (schema: JsonSchema): Record<string, unknown> => {
	const result: Record<string, unknown> = {};
	if (!schema.properties) return result;
	for (const [key, prop] of Object.entries(schema.properties)) {
		if (prop.default !== undefined) {
			result[key] = prop.default;
		} else if (prop.type === "string") {
			result[key] = "";
		} else if (prop.type === "number" || prop.type === "integer") {
			result[key] = 0;
		} else if (prop.type === "boolean") {
			result[key] = false;
		} else if (prop.type === "array") {
			result[key] = [];
		} else if (prop.type === "object") {
			result[key] = {};
		}
	}
	return result;
};

export function StructVariable({
	disabled,
	variable,
	onChange,
	refs,
}: Readonly<{
	disabled?: boolean;
	variable: IVariable;
	onChange: (variable: IVariable) => void;
	refs?: Record<string, string>;
}>) {
	const schema = useMemo(
		() => parseSchema(variable.schema, refs),
		[variable.schema, refs],
	);

	const hasSchema = schema !== null && schema.properties !== undefined;

	const [useJsonMode, setUseJsonMode] = useState(!hasSchema);
	const [jsonValue, setJsonValue] = useState<string>(() => {
		const parsed = parseUint8ArrayToJson(variable.default_value);
		return typeof parsed === "object" ? JSON.stringify(parsed, null, 2) : "{}";
	});
	const [jsonError, setJsonError] = useState<string | null>(null);
	const [isFocused, setIsFocused] = useState(false);

	const [formValues, setFormValues] = useState<Record<string, unknown>>(() => {
		const parsed = parseUint8ArrayToJson(variable.default_value);
		if (typeof parsed === "object" && parsed !== null) {
			return parsed as Record<string, unknown>;
		}
		if (hasSchema) {
			return getDefaultFromSchema(schema);
		}
		return {};
	});

	// Re-initialize form values and mode when schema changes
	useEffect(() => {
		const parsed = parseUint8ArrayToJson(variable.default_value);
		if (hasSchema) {
			setUseJsonMode(false);
			const defaults = getDefaultFromSchema(schema!);
			if (typeof parsed === "object" && parsed !== null) {
				setFormValues({ ...defaults, ...parsed });
			} else {
				setFormValues(defaults);
			}
		} else {
			setUseJsonMode(true);
			if (typeof parsed === "object" && parsed !== null) {
				setFormValues(parsed as Record<string, unknown>);
			} else {
				setFormValues({});
			}
		}
	}, [variable.schema, refs]);

	// Sync JSON value when switching modes
	useEffect(() => {
		if (useJsonMode) {
			setJsonValue(JSON.stringify(formValues, null, 2));
		}
	}, [useJsonMode]);

	// Update variable when form values change (non-JSON mode)
	useEffect(() => {
		if (useJsonMode) return;
		onChange({
			...variable,
			default_value: convertJsonToUint8Array(formValues),
		});
	}, [formValues]);

	const handleJsonChange = useCallback(
		(newJson: string) => {
			setJsonValue(newJson);
			try {
				const parsed = JSON.parse(newJson);
				setJsonError(null);
				onChange({
					...variable,
					default_value: convertJsonToUint8Array(parsed),
				});
			} catch (e) {
				setJsonError("Invalid JSON");
			}
		},
		[onChange, variable],
	);

	const handleFieldChange = useCallback((fieldName: string, value: unknown) => {
		setFormValues((prev) => ({ ...prev, [fieldName]: value }));
	}, []);

	const renderSchemaField = useCallback(
		(fieldName: string, prop: SchemaProperty, required: boolean) => {
			const value = formValues[fieldName];
			const label = `${fieldName}${required ? " *" : ""}`;

			if (prop.enum && prop.enum.length > 0) {
				return (
					<div key={fieldName} className="space-y-1">
						<Label className="text-xs">{label}</Label>
						<Select
							disabled={disabled}
							value={String(value ?? "")}
							onValueChange={(v) => handleFieldChange(fieldName, v)}
						>
							<SelectTrigger className="h-8">
								<SelectValue placeholder={`Select ${fieldName}`} />
							</SelectTrigger>
							<SelectContent>
								{prop.enum.map((option) => (
									<SelectItem key={option} value={option}>
										{option}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						{prop.description && (
							<p className="text-xs text-muted-foreground">
								{prop.description}
							</p>
						)}
					</div>
				);
			}

			switch (prop.type) {
				case "boolean":
					return (
						<div key={fieldName} className="flex items-center space-x-2 py-1">
							<Checkbox
								disabled={disabled}
								id={`struct-${fieldName}`}
								checked={Boolean(value)}
								onCheckedChange={(checked) =>
									handleFieldChange(fieldName, checked)
								}
							/>
							<Label
								htmlFor={`struct-${fieldName}`}
								className="text-xs cursor-pointer"
							>
								{label}
							</Label>
							{prop.description && (
								<span className="text-xs text-muted-foreground ml-2">
									{prop.description}
								</span>
							)}
						</div>
					);

				case "integer":
					return (
						<div key={fieldName} className="space-y-1">
							<Label className="text-xs">{label}</Label>
							<Input
								disabled={disabled}
								type="number"
								step="1"
								className="h-8"
								value={String(value ?? "")}
								onChange={(e) =>
									handleFieldChange(
										fieldName,
										e.target.value ? Number.parseInt(e.target.value, 10) : "",
									)
								}
								placeholder={prop.description || `Enter ${fieldName}`}
							/>
						</div>
					);

				case "number":
					return (
						<div key={fieldName} className="space-y-1">
							<Label className="text-xs">{label}</Label>
							<Input
								disabled={disabled}
								type="number"
								step="0.1"
								className="h-8"
								value={String(value ?? "")}
								onChange={(e) =>
									handleFieldChange(
										fieldName,
										e.target.value ? Number.parseFloat(e.target.value) : "",
									)
								}
								placeholder={prop.description || `Enter ${fieldName}`}
							/>
						</div>
					);

				case "array":
				case "object":
					return (
						<div key={fieldName} className="space-y-1">
							<Label className="text-xs">{label}</Label>
							<Textarea
								disabled={disabled}
								className="font-mono text-xs h-20"
								value={
									typeof value === "object"
										? JSON.stringify(value, null, 2)
										: String(value ?? "")
								}
								onChange={(e) => {
									try {
										const parsed = JSON.parse(e.target.value);
										handleFieldChange(fieldName, parsed);
									} catch {
										// Keep raw text for partial edits
									}
								}}
								placeholder={prop.description || `Enter ${fieldName} as JSON`}
							/>
						</div>
					);

				default:
					return (
						<div key={fieldName} className="space-y-1">
							<Label className="text-xs">{label}</Label>
							<Input
								disabled={disabled}
								type="text"
								className="h-8"
								value={String(value ?? "")}
								onChange={(e) => handleFieldChange(fieldName, e.target.value)}
								placeholder={prop.description || `Enter ${fieldName}`}
							/>
						</div>
					);
			}
		},
		[formValues, handleFieldChange, disabled],
	);

	return (
		<div className="grid w-full items-center gap-2">
			{hasSchema && (
				<div className="flex items-center justify-end gap-2">
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant={useJsonMode ? "outline" : "secondary"}
								size="sm"
								className="h-7 px-2 gap-1"
								onClick={() => {
									if (useJsonMode) {
										// Switching to form mode - parse JSON into form values
										try {
											const parsed = JSON.parse(jsonValue);
											setFormValues(parsed);
											setJsonError(null);
										} catch {
											// Keep current form values if JSON is invalid
										}
									}
									setUseJsonMode(false);
								}}
							>
								<FormInputIcon className="w-3 h-3" />
								<span className="text-xs">Form</span>
							</Button>
						</TooltipTrigger>
						<TooltipContent>Edit using generated form</TooltipContent>
					</Tooltip>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant={useJsonMode ? "secondary" : "outline"}
								size="sm"
								className="h-7 px-2 gap-1"
								onClick={() => {
									setJsonValue(JSON.stringify(formValues, null, 2));
									setUseJsonMode(true);
								}}
							>
								<BracesIcon className="w-3 h-3" />
								<span className="text-xs">JSON</span>
							</Button>
						</TooltipTrigger>
						<TooltipContent>Edit raw JSON</TooltipContent>
					</Tooltip>
				</div>
			)}

			{useJsonMode || !hasSchema ? (
				<div className="space-y-1">
					<div
						className={cn(
							"relative w-full rounded-md border bg-transparent transition-all duration-200",
							"border-input dark:bg-input/30",
							isFocused && "border-ring ring-ring/50 ring-[3px]",
							jsonError && "border-destructive",
							disabled && "opacity-50 cursor-not-allowed",
						)}
					>
						<textarea
							disabled={disabled}
							value={jsonValue}
							onChange={(e) => handleJsonChange(e.target.value)}
							onFocus={() => setIsFocused(true)}
							onBlur={() => setIsFocused(false)}
							placeholder='{"key": "value"}'
							autoComplete="off"
							spellCheck="false"
							autoCorrect="off"
							autoCapitalize="off"
							rows={8}
							className={cn(
								"w-full resize-none bg-transparent px-3 py-2 text-sm outline-none",
								"font-mono leading-[22px]",
								"placeholder:text-muted-foreground",
							)}
						/>
					</div>
					{jsonError && <p className="text-xs text-destructive">{jsonError}</p>}
					{!hasSchema && (
						<p className="text-xs text-muted-foreground">
							No schema defined. Add a schema to enable form mode.
						</p>
					)}
				</div>
			) : (
				<div className="space-y-3 border rounded-md p-3">
					{schema.description && (
						<p className="text-xs text-muted-foreground mb-2">
							{schema.description}
						</p>
					)}
					{Object.entries(schema.properties || {}).map(([fieldName, prop]) =>
						renderSchemaField(
							fieldName,
							prop,
							schema.required?.includes(fieldName) ?? false,
						),
					)}
				</div>
			)}
		</div>
	);
}
