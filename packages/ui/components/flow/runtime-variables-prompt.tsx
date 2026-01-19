import {
	AlertCircleIcon,
	CheckIcon,
	EyeIcon,
	EyeOffIcon,
	KeyIcon,
	SaveIcon,
} from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Badge } from "../ui/badge";
import type { IVariable } from "../../lib/schema/flow/board";
import { convertJsonToUint8Array, parseUint8ArrayToJson } from "../../lib/uint8";
import type { RuntimeVariableValue } from "../../state/runtime-variables-context";

export interface RuntimeVariablesPromptProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	variables: IVariable[];
	existingValues: Map<string, RuntimeVariableValue>;
	onSave: (values: RuntimeVariableValue[]) => Promise<void>;
	onCancel: () => void;
}

/**
 * A dialog that prompts the user to configure missing runtime variables
 * before executing a flow.
 */
export function RuntimeVariablesPrompt({
	open,
	onOpenChange,
	variables,
	existingValues,
	onSave,
	onCancel,
}: RuntimeVariablesPromptProps) {
	const [values, setValues] = useState<Map<string, string>>(() => {
		const map = new Map<string, string>();
		for (const variable of variables) {
			const existing = existingValues.get(variable.id);
			if (existing?.value) {
				try {
					const decoded = parseUint8ArrayToJson(existing.value);
					map.set(
						variable.id,
						typeof decoded === "string" ? decoded : JSON.stringify(decoded),
					);
				} catch {
					map.set(variable.id, "");
				}
			} else if (variable.default_value) {
				try {
					const decoded = parseUint8ArrayToJson(variable.default_value);
					map.set(
						variable.id,
						typeof decoded === "string" ? decoded : JSON.stringify(decoded),
					);
				} catch {
					map.set(variable.id, "");
				}
			} else {
				map.set(variable.id, "");
			}
		}
		return map;
	});
	const [showPasswords, setShowPasswords] = useState<Set<string>>(new Set());
	const [isSaving, setIsSaving] = useState(false);

	const missingVariables = useMemo(() => {
		return variables.filter((v) => {
			const value = values.get(v.id);
			return !value || value.trim() === "";
		});
	}, [variables, values]);

	const canSave = missingVariables.length === 0;

	const handleSave = useCallback(async () => {
		if (!canSave) return;
		setIsSaving(true);
		try {
			const result: RuntimeVariableValue[] = [];
			for (const variable of variables) {
				const value = values.get(variable.id) ?? "";
				const encoded = convertJsonToUint8Array(value);
				if (encoded) {
					result.push({
						variableId: variable.id,
						value: encoded,
					});
				}
			}
			await onSave(result);
		} finally {
			setIsSaving(false);
		}
	}, [canSave, variables, values, onSave]);

	const toggleShowPassword = useCallback((variableId: string) => {
		setShowPasswords((prev) => {
			const next = new Set(prev);
			if (next.has(variableId)) {
				next.delete(variableId);
			} else {
				next.add(variableId);
			}
			return next;
		});
	}, []);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-lg max-h-[80vh] overflow-y-auto">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<KeyIcon className="w-5 h-5" />
						Configure Runtime Variables
					</DialogTitle>
					<DialogDescription>
						This flow requires runtime variables to be configured before
						execution. These values are stored locally and never uploaded.
					</DialogDescription>
				</DialogHeader>

				<div className="space-y-4 py-4">
					{variables.map((variable) => {
						const value = values.get(variable.id) ?? "";
						const isSecret = variable.secret;
						const showPassword = showPasswords.has(variable.id);
						const isConfigured = value.trim() !== "";

						return (
							<Card key={variable.id} className="p-4">
								<div className="flex flex-col gap-2">
									<div className="flex items-center justify-between">
										<Label
											htmlFor={`runtime-var-${variable.id}`}
											className="text-sm font-medium flex items-center gap-2"
										>
											{variable.name}
											{isSecret && (
												<Badge variant="secondary" className="text-xs gap-1">
													<KeyIcon className="w-3 h-3" />
													Secret
												</Badge>
											)}
										</Label>
										{isConfigured && (
											<CheckIcon className="w-4 h-4 text-green-500" />
										)}
									</div>

									{variable.description && (
										<p className="text-xs text-muted-foreground">
											{variable.description}
										</p>
									)}

									<div className="relative">
										<Input
											id={`runtime-var-${variable.id}`}
											type={isSecret && !showPassword ? "password" : "text"}
											value={value}
											onChange={(e) => {
												setValues((prev) => {
													const next = new Map(prev);
													next.set(variable.id, e.target.value);
													return next;
												});
											}}
											placeholder={
												isSecret ? "Enter secret value..." : "Enter value..."
											}
											className="pr-10"
										/>
										{isSecret && (
											<Button
												variant="ghost"
												size="icon"
												className="absolute right-1 top-1/2 -translate-y-1/2 h-7 w-7"
												onClick={() => toggleShowPassword(variable.id)}
												type="button"
											>
												{showPassword ? (
													<EyeOffIcon className="w-4 h-4" />
												) : (
													<EyeIcon className="w-4 h-4" />
												)}
											</Button>
										)}
									</div>
								</div>
							</Card>
						);
					})}
				</div>

				{missingVariables.length > 0 && (
					<div className="flex items-center gap-2 text-amber-500 text-sm">
						<AlertCircleIcon className="w-4 h-4" />
						{missingVariables.length} variable
						{missingVariables.length !== 1 ? "s" : ""} still need
						{missingVariables.length === 1 ? "s" : ""} to be configured
					</div>
				)}

				<DialogFooter className="gap-2">
					<Button variant="outline" onClick={onCancel}>
						Cancel
					</Button>
					<Button
						onClick={handleSave}
						disabled={!canSave || isSaving}
						className="gap-2"
					>
						<SaveIcon className="w-4 h-4" />
						Save & Continue
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
