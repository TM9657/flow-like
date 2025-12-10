import { Input } from "../../../components/ui/input";
import { MonacoTextEditor } from "../../../components/ui/monaco-text-editor";
import type { IVariable } from "../../../lib/schema/flow/variable";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../lib/uint8";

export function StringVariable({
	disabled,
	variable,
	onChange,
}: Readonly<{
	disabled?: boolean;
	variable: IVariable;
	onChange: (variable: IVariable) => void;
}>) {
	return (
		<div className="grid w-full items-center gap-1.5">
			{variable.secret ? (
				<Input
					autoComplete="off"
					spellCheck="false"
					autoCorrect="off"
					autoCapitalize="off"
					disabled={disabled}
					value={parseUint8ArrayToJson(variable.default_value)}
					onChange={(e) => {
						onChange({
							...variable,
							default_value: convertJsonToUint8Array(e.target.value),
						});
					}}
					type="password"
					id="default_value"
					placeholder="Default Value"
				/>
			) : (
				<MonacoTextEditor
					disabled={disabled}
					height="300px"
					value={parseUint8ArrayToJson(variable.default_value)}
					onChange={(newValue) => {
						onChange({
							...variable,
							default_value: convertJsonToUint8Array(newValue),
						});
					}}
					placeholder="Default Value"
				/>
			)}
		</div>
	);
}
