import { type IPin } from "../../../../lib/schema/flow/pin"
import { VariableDescription } from "./default-text"
import { Checkbox } from "../../../../components/ui/checkbox"
import { convertJsonToUint8Array, parseUint8ArrayToJson } from "../../../../lib/uint8"
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "../../../../components/ui/select"

export function EnumVariable({ pin, value, setValue }: Readonly<{ pin: IPin, value: number[] | undefined | null, setValue: (value: any) => void }>) {
    return <div className="flex flex-row items-center justify-start">
        <Select defaultValue={parseUint8ArrayToJson(value)} value={parseUint8ArrayToJson(value)} onValueChange={(value) => setValue(convertJsonToUint8Array(value))}>
            <SelectTrigger className="w-full p-0 border-0 text-xs text-nowrap text-start max-h-fit h-4" >
                <small className='text-nowrap text-start m-0'>{parseUint8ArrayToJson(value)}</small>
            </SelectTrigger>
            <SelectContent>
                {
                    pin.options?.valid_values?.map((option) => {
                        return <SelectItem key={option} value={option}>{option}</SelectItem>
                    })
                }
            </SelectContent>
        </Select>
    </div>



}