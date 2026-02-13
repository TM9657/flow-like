import { ChevronDown } from "lucide-react";
import { useCallback, useState } from "react";
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
	setValue,
}: Readonly<{
	pin: IPin;
	value: number[] | undefined | null;
	setValue: (value: any) => void;
}>) {
	const backend = useBackend();
	const [open, setOpen] = useState(false);

	const profileBits = useInvoke(
		backend.bitState.getProfileBits,
		backend.bitState,
		[],
		open,
	);

	const parsedValue = parseUint8ArrayToJson(value);

	const handleOpenChange = useCallback(
		(isOpen: boolean) => {
			setOpen(isOpen);
			if (isOpen) profileBits.refetch();
		},
		[profileBits],
	);

	return (
		<div className="flex flex-row items-center justify-start max-w-full ml-1 overflow-hidden">
			<Select
				open={open}
				onOpenChange={handleOpenChange}
				value={parsedValue}
				onValueChange={(v) => setValue(convertJsonToUint8Array(v))}
			>
				<SelectTrigger
					noChevron
					size="sm"
					className="w-fit! max-w-full! p-0 border-0 text-xs bg-card! text-start max-h-fit h-4 gap-0.5 flex-row items-center overflow-hidden"
				>
					<small className="text-start text-[10px] m-0! truncate">
						<BitRender backend={backend} bitId={parsedValue} />
					</small>
					<ChevronDown className="size-2 min-w-2 min-h-2 text-card-foreground mt-0.5 shrink-0" />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						<SelectLabel>{pin.friendly_name}</SelectLabel>
						{profileBits?.data?.map((bit) => {
							const bitId = `${bit.hub}:${bit.id}`;
							return (
								<SelectItem key={bitId} value={bitId}>
									{bit.meta?.en?.name ?? bit.id}
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
	const lastColonIndex = bitId?.lastIndexOf(":");
	const hub =
		lastColonIndex !== undefined && lastColonIndex > 0
			? bitId?.substring(0, lastColonIndex)
			: undefined;
	const id =
		lastColonIndex !== undefined && lastColonIndex > 0
			? bitId?.substring(lastColonIndex + 1)
			: bitId;

	const bit = useInvoke(
		backend.bitState.getBit,
		backend.bitState,
		[id!, hub],
		!!id,
	);

	if (!bitId) return <span className="truncate m-0">Select a bit</span>;
	if (bit.isFetching) return <span className="truncate m-0">Loading</span>;
	if (bit.error) return <span className="truncate m-0">Error loading bit</span>;

	return <span className="truncate m-0">{bit.data?.meta?.["en"]?.name}</span>;
}
