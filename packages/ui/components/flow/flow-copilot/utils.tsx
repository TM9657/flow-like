import {
	CircleDotIcon,
	EditIcon,
	LinkIcon,
	MessageSquareIcon,
	MoveIcon,
	PencilIcon,
	PlusCircleIcon,
	SquarePenIcon,
	Unlink2Icon,
	XCircleIcon,
} from "lucide-react";

import type { BoardCommand } from "../../../lib/schema/flow/copilot";

export function getCommandIcon(cmd: BoardCommand, size = "w-4 h-4") {
	switch (cmd.command_type) {
		case "AddNode":
			return <PlusCircleIcon className={size} />;
		case "RemoveNode":
			return <XCircleIcon className={size} />;
		case "ConnectPins":
			return <LinkIcon className={size} />;
		case "DisconnectPins":
			return <Unlink2Icon className={size} />;
		case "UpdateNodePin":
			return <PencilIcon className={size} />;
		case "MoveNode":
			return <MoveIcon className={size} />;
		case "CreateVariable":
		case "UpdateVariable":
		case "DeleteVariable":
			return <EditIcon className={size} />;
		case "CreateComment":
		case "UpdateComment":
		case "DeleteComment":
			return <MessageSquareIcon className={size} />;
		case "CreateLayer":
		case "AddNodesToLayer":
		case "RemoveNodesFromLayer":
			return <SquarePenIcon className={size} />;
		default:
			return <CircleDotIcon className={size} />;
	}
}

export function getCommandSummary(cmd: BoardCommand): string {
	if (cmd.summary) return cmd.summary;
	switch (cmd.command_type) {
		case "AddNode":
			return `Add ${cmd.friendly_name || cmd.node_type.split("::").pop() || "node"}`;
		case "RemoveNode":
			return "Remove node";
		case "ConnectPins":
			return `Connect ${cmd.from_pin} → ${cmd.to_pin}`;
		case "DisconnectPins":
			return "Disconnect pins";
		case "UpdateNodePin":
			return `Set ${cmd.pin_id}`;
		case "MoveNode":
			return "Move node";
		default:
			return cmd.command_type.replace(/([A-Z])/g, " $1").trim();
	}
}

export function getCommandColor(cmd: BoardCommand): string {
	switch (cmd.command_type) {
		case "AddNode":
			return "from-green-500/20 to-emerald-500/10 border-green-500/30";
		case "RemoveNode":
			return "from-red-500/20 to-rose-500/10 border-red-500/30";
		case "ConnectPins":
			return "from-blue-500/20 to-cyan-500/10 border-blue-500/30";
		case "DisconnectPins":
			return "from-orange-500/20 to-amber-500/10 border-orange-500/30";
		case "UpdateNodePin":
			return "from-violet-500/20 to-purple-500/10 border-violet-500/30";
		default:
			return "from-primary/20 to-primary/10 border-primary/30";
	}
}
