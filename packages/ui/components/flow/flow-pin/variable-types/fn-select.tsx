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

export function FnVariable({
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
		<div className="flex flex-row items-center justify-start ml-1">
			<Select
				defaultValue={parseUint8ArrayToJson(value)}
				value={parseUint8ArrayToJson(value)}
				onValueChange={(value) => setValue(convertJsonToUint8Array(value))}
				onOpenChange={async () => {
					// const nodes = flow.getNodes();
				}}
			>
				<SelectTrigger
					noChevron
					size="sm"
					className="!w-fit !max-w-fit p-0 border-0 text-xs !bg-card text-nowrap text-start max-h-fit h-4 gap-0.5 flex-row items-center"
				>
					<small className="text-nowrap text-start text-[10px] !m-0 w-fit">
						{!board.data && "Loading..."}
						{board.data &&
							(board?.data?.nodes?.[parseUint8ArrayToJson(value)]
								?.friendly_name ??
								"No Function Selected")}
					</small>
					<ChevronDown className="size-2 min-w-2 min-h-2 text-card-foreground" />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						<SelectLabel>{pin.friendly_name}</SelectLabel>
						{Object.values(board?.data?.nodes ?? {})
							?.filter((node) => node.start)
							.map((node) => {
								return (
									<SelectItem key={node.id} value={node.id}>
										{node.friendly_name ?? node.name}
									</SelectItem>
								);
							})}
					</SelectGroup>
				</SelectContent>
			</Select>
		</div>
	);
}
