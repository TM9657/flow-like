"use client";

import { VariableIcon } from "lucide-react";
import { type FC, memo, useEffect, useState } from "react";
import { Button } from "../../../components/ui/button";
import { IValueType } from "../../../lib";
import {
	type IPin,
	IPinType,
	IVariableType,
} from "../../../lib/schema/flow/pin";
import useFlowControlState from "../../../state/flow-control-state";
import { BitVariable } from "./variable-types/bit-select";
import { BooleanVariable } from "./variable-types/boolean-variable";
import { VariableDescription } from "./variable-types/default-text";
import { ElementSelect } from "./variable-types/element-select";
import { EnumVariable } from "./variable-types/enum-variable";
import { FnVariable } from "./variable-types/fn-select";
import { VarVariable } from "./variable-types/var-select";

interface PinEditProps {
	readonly nodeId: string;
	readonly pin: IPin;
	readonly defaultValue: any;
	readonly appId: string;
	readonly boardId: string;
	readonly changeDefaultValue: (value: any) => void;
	readonly saveDefaultValue: (value: any) => Promise<void>;
}

export const PinEdit: FC<PinEditProps> = memo(function PinEdit({
	nodeId,
	pin,
	defaultValue,
	appId,
	boardId,
	changeDefaultValue,
	saveDefaultValue,
}: PinEditProps) {
	const [cachedDefaultValue, setCachedDefaultValue] = useState(defaultValue);

	useEffect(() => {
		changeDefaultValue(cachedDefaultValue);
	}, [cachedDefaultValue, changeDefaultValue]);

	if (pin.pin_type === IPinType.Output)
		return <VariableDescription pin={pin} />;
	if (pin.depends_on.length > 0) return <VariableDescription pin={pin} />;
	if (pin.data_type === IVariableType.Boolean)
		return (
			<BooleanVariable
				pin={pin}
				value={cachedDefaultValue}
				setValue={async (value) => {
					setCachedDefaultValue(value);
					await saveDefaultValue(value);
				}}
			/>
		);
	if (
		(pin.options?.valid_values?.length ?? 0) > 0 &&
		pin.data_type === IVariableType.String
	)
		return (
			<EnumVariable
				pin={pin}
				value={cachedDefaultValue}
				setValue={async (value) => {
					setCachedDefaultValue(value);
					await saveDefaultValue(value);
				}}
			/>
		);

	if (
		pin.name.startsWith("bit_id") &&
		pin.data_type === IVariableType.String &&
		pin.value_type === IValueType.Normal
	) {
		return (
			<BitVariable
				pin={pin}
				value={cachedDefaultValue}
				appId={appId}
				setValue={async (value) => {
					setCachedDefaultValue(value);
					await saveDefaultValue(value);
				}}
			/>
		);
	}

	if (
		pin.name.startsWith("fn_ref") &&
		pin.data_type === IVariableType.String &&
		pin.value_type === IValueType.Normal
	) {
		return (
			<FnVariable
				boardId={boardId}
				pin={pin}
				value={cachedDefaultValue}
				appId={appId}
				setValue={async (value) => {
					setCachedDefaultValue(value);
					await saveDefaultValue(value);
				}}
			/>
		);
	}

	if (
		pin.name.startsWith("var_ref") &&
		pin.data_type === IVariableType.String &&
		pin.value_type === IValueType.Normal
	) {
		return (
			<VarVariable
				boardId={boardId}
				pin={pin}
				value={cachedDefaultValue}
				appId={appId}
				setValue={async (value) => {
					setCachedDefaultValue(value);
					await saveDefaultValue(value);
				}}
			/>
		);
	}

	if (
		pin.name.startsWith("element_ref") &&
		pin.data_type === IVariableType.String &&
		pin.value_type === IValueType.Normal
	) {
		return (
			<ElementSelect
				pin={pin}
				value={cachedDefaultValue}
				appId={appId}
				setValue={async (value) => {
					setCachedDefaultValue(value);
					await saveDefaultValue(value);
				}}
			/>
		);
	}

	return (
		<WithMenu nodeId={nodeId} pin={pin} defaultValue={cachedDefaultValue} />
	);
});

function WithMenuInner({
	nodeId,
	pin,
	defaultValue,
}: Readonly<{
	nodeId: string;
	pin: IPin;
	defaultValue: number[] | undefined | null;
}>) {
	const { editPin } = useFlowControlState();
	return (
		<>
			<VariableDescription pin={pin} />
			<Button
				size={"icon"}
				variant={"ghost"}
				className="w-fit h-fit text-foreground"
				onClick={() => {
					editPin(nodeId, pin);
				}}
			>
				<VariableIcon
					className={`size-[0.45rem] ${(typeof defaultValue === "undefined" || defaultValue === null) && "text-primary"}`}
				/>
			</Button>
		</>
	);
}

const WithMenu = memo(WithMenuInner) as typeof WithMenuInner;
