import { type IPin } from "../../../../lib/schema/flow/pin";
import { VariableDescription } from "./default-text";
import { Checkbox } from "../../../../components/ui/checkbox";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../../lib/uint8";

export function BooleanVariable({
	pin,
	value,
	setValue,
}: Readonly<{
	pin: IPin;
	value: number[] | undefined | null;
	setValue: (value: any) => void;
}>) {
	return (
		<>
			<VariableDescription pin={pin} />
			<Checkbox
				checked={parseUint8ArrayToJson(value) ?? false}
				onCheckedChange={(checked) => {
					setValue(convertJsonToUint8Array(checked));
				}}
				className="scale-50"
			/>
		</>
	);
}
