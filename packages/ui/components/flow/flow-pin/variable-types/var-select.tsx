import { ChevronDown } from "lucide-react";
import { useBackend, useInvoke } from "../../../..";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
} from "../../../../components/ui/select";
import type { IPin } from "../../../../lib/schema/flow/pin";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../../lib/uint8";

export function VarVariable({
	pin,
	value,
	boardId,
	appId,
	setValue,
}: Readonly<{
	pin: IPin;
	value: number[] | undefined | null;
	boardId: string;
	appId: string;
	setValue: (value: any) => void;
}>) {
	const backend = useBackend();
	const board = useInvoke(backend.boardState.getBoard, backend.boardState, [
		appId,
		boardId,
	]);

	return (
		<div className="flex flex-row items-center justify-start max-w-full ml-1 overflow-hidden">
			<Select
				defaultValue={parseUint8ArrayToJson(value)}
				value={parseUint8ArrayToJson(value)}
				onValueChange={(value) => setValue(convertJsonToUint8Array(value))}
			>
				<SelectTrigger
					noChevron
					size="sm"
					className="w-fit! max-w-full! p-0 border-0 text-xs bg-card! text-start max-h-fit h-4 gap-0.5 flex-row items-center overflow-hidden"
				>
					<small className="text-start text-[10px] m-0! truncate">
						{!board.data && "Loading..."}
						{board.data &&
							(board?.data?.variables?.[parseUint8ArrayToJson(value)]?.name ??
								"No Variable Selected")}
					</small>
					<ChevronDown className="size-2 min-w-2 min-h-2 text-card-foreground shrink-0" />
				</SelectTrigger>
				<SelectContent className="bg-background">
					<SelectGroup>
						<SelectLabel>{pin.friendly_name}</SelectLabel>
						{Object.values(board?.data?.variables ?? {})?.map((variable) => {
							return (
								<SelectItem key={variable.id} value={variable.id}>
									{variable.name}
								</SelectItem>
							);
						})}
					</SelectGroup>
				</SelectContent>
			</Select>
		</div>
	);
}
