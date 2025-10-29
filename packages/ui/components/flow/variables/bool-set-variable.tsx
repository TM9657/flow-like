import { PlusCircleIcon, XIcon } from "lucide-react";
import { useState } from "react";
import type { IVariable } from "../../../lib/schema/flow/variable";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../lib/uint8";
import { Button, Label, Separator, Switch } from "../../ui";

export function BoolSetVariable({
	disabled,
	variable,
	onChange,
}: Readonly<{
	disabled?: boolean;
	variable: IVariable;
	onChange: (variable: IVariable) => void;
}>) {
	const [newValue, setNewValue] = useState(false);

	const currentArray = Array.isArray(
		parseUint8ArrayToJson(variable.default_value),
	)
		? (parseUint8ArrayToJson(variable.default_value) as boolean[])
		: [];

	const addValue = () => {
		if (disabled || newValue === undefined) return;
		const updated = [...currentArray, newValue];
		onChange({
			...variable,
			default_value: convertJsonToUint8Array(Array.from(new Set(updated))),
		});
		setNewValue(false);
	};

	const removeAt = (idx: number) => {
		if (disabled || idx < 0 || idx >= currentArray.length) return;
		const updated = currentArray.slice();
		updated.splice(idx, 1);
		onChange({
			...variable,
			default_value: convertJsonToUint8Array(Array.from(new Set(updated))),
		});
	};

	return (
		<div className="flex flex-col gap-3 w-full min-w-0">
			<div className="flex gap-2 items-center justify-between">
				<div className="flex gap-2 items-center">
					<Switch
						disabled={disabled}
						checked={newValue}
						onCheckedChange={setNewValue}
						id="new_value"
					/>
					<Label htmlFor="new_value" className="cursor-pointer">
						{newValue ? "True" : "False"}
					</Label>
				</div>
				<Button
					disabled={disabled}
					size="icon"
					variant="default"
					onClick={addValue}
					className="shrink-0"
				>
					<PlusCircleIcon className="w-4 h-4" />
				</Button>
			</div>

			{currentArray.length > 0 && (
				<>
					<Separator />
					<div className="flex flex-col gap-2 rounded-md border p-3">
						{currentArray.map((val, idx) => (
							<div
								key={`${variable.name}-${idx}`}
								className="group flex items-center gap-2 rounded-md bg-secondary px-3 py-2"
							>
								<Switch
									disabled={disabled}
									checked={val}
									onCheckedChange={(v) => {
										const updated = currentArray.slice();
										updated[idx] = v;
										onChange({
											...variable,
											default_value: convertJsonToUint8Array(
												Array.from(new Set(updated)),
											),
										});
									}}
									id={`item-${idx}`}
									className="scale-75"
								/>
								<Label
									htmlFor={`item-${idx}`}
									className="cursor-pointer text-xs"
								>
									{val ? "True" : "False"}
								</Label>
								<Button
									size="icon"
									variant="ghost"
									onClick={() => removeAt(idx)}
									disabled={disabled}
									className="h-5 w-5 shrink-0 rounded-sm hover:bg-destructive hover:text-destructive-foreground ml-auto"
								>
									<XIcon className="h-3 w-3" />
								</Button>
							</div>
						))}
					</div>
				</>
			)}
		</div>
	);
}
