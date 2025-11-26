import { type IPin, IPinType } from "../../../../lib/schema/flow/pin";
import { AutoResizeText } from "../../auto-resize-text";

export function VariableDescription({ pin }: Readonly<{ pin: IPin }>) {
	return (
		<small
			className={`w-fit max-w-full block truncate ${pin.pin_type === IPinType.Input ? "text-start ml-1" : "text-end mr-1"}`}
		>
			<AutoResizeText text={pin.friendly_name} maxChars={20} />
		</small>
	);
}
