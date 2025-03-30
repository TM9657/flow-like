import { DndContext, MouseSensor, useSensor, useSensors } from "@dnd-kit/core";
import { useCallback, useState } from "react";
import type { IVariable } from "../../lib/schema/flow/variable";
import { Button } from "../ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../ui/dialog";
import { FlowBoard } from "./flow-board";

export function FlowWrapper({ boardId, appId }: Readonly<{ boardId: string, appId: string }>) {
	const mouseSensor = useSensor(MouseSensor, {
		activationConstraint: {
			distance: 10,
		},
	});

	const [detail, setDetail] = useState<
		| undefined
		| {
				variable: IVariable;
				screenPosition: { x: number; y: number };
		  }
	>();

	const sensors = useSensors(mouseSensor);

	const placeNode = useCallback(
		async (operation: "set" | "get") => {
			document.dispatchEvent(
				new CustomEvent("flow-drop", {
					detail: { ...detail, operation },
				}),
			);
			setDetail(undefined);
		},
		[detail, boardId],
	);

	return (
		<DndContext
			sensors={sensors}
			onDragEnd={(event) => {
				if (event.over?.id !== "flow") return;
				console.dir(event);
				const mouseEvent: MouseEvent = event.activatorEvent as MouseEvent;
				setDetail({
					variable: event.active.data.current as IVariable,
					screenPosition: {
						x: mouseEvent.screenX + event.delta.x,
						y: mouseEvent.screenY + event.delta.y,
					},
				});
			}}
		>
			<FlowBoard boardId={boardId} appId={appId} />
			<Dialog
				open={detail !== undefined}
				onOpenChange={(open) => {
					if (!open) setDetail(undefined);
				}}
			>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Reference: {detail?.variable.name}</DialogTitle>
					</DialogHeader>
					<div className="w-full flex items-center justify-start gap-2">
						<Button
							className="w-full"
							variant={"outline"}
							onClick={() => placeNode("get")}
						>
							Get
						</Button>
						<Button
							className="w-full"
							variant={"outline"}
							onClick={() => placeNode("set")}
						>
							Set
						</Button>
					</div>
				</DialogContent>
			</Dialog>
		</DndContext>
	);
}
