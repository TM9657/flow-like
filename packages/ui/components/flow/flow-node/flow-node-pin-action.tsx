import { CirclePlusIcon } from "lucide-react";
import type { IPinAction } from "../flow-node";

export function FlowPinAction({
	action,
	index,
	input,
}: Readonly<{ action: IPinAction; index: number; input: boolean }>) {
	return (
		<button
			className={`absolute flex flex-row items-center gap-1 top-0 translate-y-[-50%] translate-x-[-6px] ${input ? "-left-3" : "-right-6"}`}
			style={{
				top: (index - 1) * 15,
				marginTop: "1.75rem",
				height: 15,
			}}
			onClick={() => {
				action.onAction(action.pin);
			}}
		>
			<CirclePlusIcon className="w-3 h-3 bg-background rounded-full" />
			{/* <small className='text-nowrap text-start'>Add Pin</small> */}
		</button>
	);
}
