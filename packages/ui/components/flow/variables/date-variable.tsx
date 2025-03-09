import { Calendar } from "../../../components/ui/calendar";
import type { ISystemTime } from "../../../lib/schema/flow/board";
import type { IVariable } from "../../../lib/schema/flow/variable";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../lib/uint8";

export function DateVariable({
	variable,
	onChange,
}: Readonly<{ variable: IVariable; onChange: (variable: IVariable) => void }>) {
	return (
		<div className="flex items-center space-x-2">
			<Calendar
				mode="single"
				selected={
					parseUint8ArrayToJson(variable.default_value)?.nanos_since_epoch
						? new Date(
								parseUint8ArrayToJson(variable.default_value)
									.nanos_since_epoch / 1000000,
							)
						: new Date()
				}
				onSelect={(date) => {
					if (!date) return;
					const rustDate: ISystemTime = {
						secs_since_epoch: date.getTime() / 1000,
						nanos_since_epoch: date.getTime() * 1000000,
					};
					onChange({
						...variable,
						default_value: convertJsonToUint8Array(rustDate),
					});
				}}
				className="rounded-md border"
			/>
		</div>
	);
}
