"use client";

import { useTranslation } from "@flow-like/locales";
import { MapPinIcon } from "lucide-react";
import { useMemo } from "react";
import { describeFlowGeometry } from "../../../../lib/geometry-display";
import type { IPin } from "../../../../lib/schema/flow/pin";
import { parseUint8ArrayToJson } from "../../../../lib/uint8";
import useFlowControlState from "../../../../state/flow-control-state";
import { GeometrySketch } from "../../../ui/geometry-cell";
import { VariableDescription } from "./default-text";

const stopPropagation = (event: { stopPropagation: () => void }) =>
	event.stopPropagation();

/** Compact on-node summary of a Geometry default; opens the pin editor modal. */
export function GeometryChip({
	nodeId,
	pin,
	value,
}: Readonly<{
	nodeId: string;
	pin: IPin;
	value: number[] | undefined | null;
}>) {
	const { t } = useTranslation("flow");
	const { editPin } = useFlowControlState();
	const decoded = useMemo(() => parseUint8ArrayToJson(value), [value]);
	const display = useMemo(() => describeFlowGeometry(decoded), [decoded]);
	const isConnected = (pin.connected_to?.length ?? 0) > 0;
	const unset = decoded === undefined || decoded === null;
	const label = unset
		? t("geometrySetValue", "Set geometry")
		: (display.kind ?? t("geometry", "Geometry"));
	return (
		<>
			<VariableDescription pin={pin} />
			{!isConnected && (
				<button
					type="button"
					className="nodrag flex h-3.5 max-w-full shrink items-center gap-0.5 overflow-hidden rounded-sm border border-orange-500/30 bg-orange-500/10 px-1 text-[0.5rem] leading-none text-foreground transition-colors hover:border-orange-500/60 hover:bg-orange-500/20"
					title={label}
					aria-label={t("geometryEditValue", "Edit geometry: {{label}}", {
						label,
					})}
					onMouseDown={stopPropagation}
					onPointerDown={stopPropagation}
					onClick={() => editPin(nodeId, pin)}
				>
					{unset ? (
						<MapPinIcon
							className="size-2.5 shrink-0 text-orange-500"
							aria-hidden
						/>
					) : (
						<GeometrySketch display={display} className="size-3" />
					)}
					<span className="truncate">{label}</span>
				</button>
			)}
		</>
	);
}
