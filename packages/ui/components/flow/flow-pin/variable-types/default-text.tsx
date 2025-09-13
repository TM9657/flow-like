import { type IPin, IPinType } from "../../../../lib/schema/flow/pin";

export function VariableDescription({ pin }: Readonly<{ pin: IPin }>) {
	return (
		<small
			className={`w-fit text-nowrap ${pin.pin_type === IPinType.Input ? "text-start ml-1" : "translate-x-[-95%] ml-2"}`}
		>
			{pin.friendly_name}
		</small>
	);
}
