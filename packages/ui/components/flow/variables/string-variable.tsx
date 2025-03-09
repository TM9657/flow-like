import { Input } from "../../../components/ui/input";
import { Textarea } from "../../../components/ui/textarea";
import { type IVariable } from "../../../lib/schema/flow/variable";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../lib/uint8";

export function StringVariable({
	variable,
	onChange,
}: Readonly<{ variable: IVariable; onChange: (variable: IVariable) => void }>) {
	return (
		<div className="grid w-full max-w-sm items-center gap-1.5">
			{variable.secret ? (
				<Input
					value={parseUint8ArrayToJson(variable.default_value)}
					onChange={(e) => {
						onChange({
							...variable,
							default_value: convertJsonToUint8Array(e.target.value),
						});
					}}
					type={variable.secret ? "password" : "text"}
					id="default_value"
					placeholder="Default Value"
				/>
			) : (
				<Textarea
					rows={6}
					value={parseUint8ArrayToJson(variable.default_value)}
					onChange={(e) => {
						onChange({
							...variable,
							default_value: convertJsonToUint8Array(e.target.value),
						});
					}}
					id="default_value"
					placeholder="Default Value"
				/>
			)}
		</div>
	);
}
