import { type IPin, IPinType } from "../../../../lib/schema/flow/pin";

export function VariableDescription({ pin }: Readonly<{ pin: IPin }>) {
	return (
		<small
			className={`text-nowrap ${pin.pin_type === IPinType.Input ? "text-start" : "text-end"}`}
		>
			{pin.friendly_name}
		</small>
	);
}
