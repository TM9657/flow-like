import { type IPin } from "../../../../lib/schema/flow/pin"

export function VariableDescription({ pin }: Readonly<{ pin: IPin }>) {
    return <small className='text-nowrap text-start'>{pin.friendly_name}</small>
}