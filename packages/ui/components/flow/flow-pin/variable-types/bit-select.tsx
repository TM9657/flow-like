import { ChevronDown } from "lucide-react";
import { type IBackendState, useBackend, useInvoke } from "../../../..";
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

export function BitVariable({
	pin,
	value,
	appId,
	setValue,
}: Readonly<{
	pin: IPin;
	value: number[] | undefined | null;
	appId: string;
	setValue: (value: any) => void;
}>) {
	const backend = useBackend();
	const app = useInvoke(
		backend.appState.getApp,
		backend.appState,
		[appId],
		!!appId,
	);

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
						<BitRender backend={backend} bitId={parseUint8ArrayToJson(value)} />
					</small>
					<ChevronDown className="size-2 min-w-2 min-h-2 text-card-foreground mt-0.5 shrink-0" />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						<SelectLabel>{pin.friendly_name}</SelectLabel>
						{app?.data?.bits?.map((option) => {
							return (
								<SelectItem key={option} value={option}>
									<BitRender backend={backend} bitId={option} />
								</SelectItem>
							);
						})}
					</SelectGroup>
				</SelectContent>
			</Select>
		</div>
	);
}

function BitRender({
	backend,
	bitId,
}: Readonly<{ backend: IBackendState; bitId?: string }>) {
	const bit = useInvoke(
		backend.bitState.getBit,
		backend.bitState,
		[bitId!],
		!!bitId,
	);

	if (!bitId) return <span className="truncate m-0">Select a bit</span>;
	if (bit.isFetching) return <span className="truncate m-0">Loading</span>;
	if (bit.error) return <span className="truncate m-0">Error loading bit</span>;

	return <span className="truncate m-0">{bit.data?.meta?.["en"]?.name}</span>;
}
