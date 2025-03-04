"use client"

import { VariableIcon } from "lucide-react"
import { useEffect, useState } from "react"
import { Button } from "../../../components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "../../../components/ui/dialog"
import { type IPin, IPinType, IVariableType } from "../../../lib/schema/flow/pin"
import { VariablesMenuEdit } from "../variables/variables-menu-edit"
import { BooleanVariable } from "./variable-types/boolean-variable"
import { VariableDescription } from "./variable-types/default-text"
import { EnumVariable } from "./variable-types/enum-variable"

export function PinEdit({ pin, defaultValue, changeDefaultValue }: Readonly<{ pin: IPin, defaultValue: any, changeDefaultValue: (value: any) => void }>) {
    const [value, setValue] = useState(defaultValue)

    useEffect(() => {
        changeDefaultValue(value)
    }, [value])

    if (pin.pin_type === IPinType.Output) return <VariableDescription pin={pin} />
    if (pin.depends_on.length > 0) return <VariableDescription pin={pin} />
    if (pin.data_type === IVariableType.Boolean) return <BooleanVariable pin={pin} value={value} setValue={setValue} />
    if ((pin.options?.valid_values?.length ?? 0) > 0 && pin.data_type === IVariableType.String) return <EnumVariable pin={pin} value={value} setValue={setValue} /> 

    return <WithMenu pin={pin} defaultValue={value} changeDefaultValue={setValue} />
}


function WithMenu({ pin, defaultValue, changeDefaultValue }: Readonly<{ pin: IPin, defaultValue: number[] | undefined | null, changeDefaultValue: (value: any) => void }>) {

    return <>
        <VariableDescription pin={pin} />
                <Button size={"icon"} variant={"ghost"} className="w-fit h-fit text-foreground">
                    <Dialog>
                        <DialogTrigger asChild>
                            <VariableIcon className={`w-[0.45rem] h-[0.45rem] min-w-[0.45rem] min-h-[0.45rem] ${(typeof defaultValue === "undefined" || defaultValue === null) && "text-primary"}`} />
                        </DialogTrigger>
                        <DialogContent>
                            <DialogHeader>
                                <DialogTitle>Set Default Value</DialogTitle>
                                <DialogDescription>
                                    The default value will only be used if the pin is not connected.
                                </DialogDescription>
                            </DialogHeader>
                            <div className="w-full">
                                <VariablesMenuEdit variable={{
                                    data_type: pin.data_type,
                                    default_value: defaultValue,
                                    exposed: false,
                                    id: pin.id,
                                    value_type: pin.value_type,
                                    name: pin.name,
                                    editable: pin.editable,
                                    secret: false,
                                    category: pin.category,
                                    description: pin.description
                                }} updateVariable={async (variable) => {
                                    changeDefaultValue(variable.default_value)
                                }} />
                            </div>

                        </DialogContent>
                    </Dialog>
                </Button>
    </>
}