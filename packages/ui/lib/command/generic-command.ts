import { createId } from "@paralleldrive/cuid2";
import type { IAddNode } from "../schema/flow/board/commands/add-node";
import { ICommandType, type IGeneric } from "../schema/flow/board/commands/generic";
import type { IRemoveComment } from "../schema/flow/board/commands/remove-comment";
import type { IUpsertComment } from "../schema/flow/board/commands/upsert-comment";
import type { ICopyPaste } from "../schema/flow/board/commands/copy-paste";
import type { IMoveNode } from "../schema/flow/board/commands/move-node";
import type { IRemoveNode } from "../schema/flow/board/commands/remove-node";
import type { IUpdateNode } from "../schema/flow/board/commands/update-node";
import type { IConnectPins } from "../schema/flow/board/commands/connect-pins";
import type { IDisconnectPins } from "../schema/flow/board/commands/disconnect-pins";
import type { IUpsertPin } from "../schema/flow/board/commands/upsert-pin";
import type { IUpsertVariable } from "../schema/flow/board/commands/upsert-variable";
import type { IRemoveVariable } from "../schema/flow/board/commands/remove-variable";
import type { INode } from "../schema";

export function addNodeCommand(command: IAddNode) : {command: IGeneric, node: INode} {
    command.node.id = createId()

    let pin_ids = Object.keys(command.node.pins);
    for (const pin of pin_ids) {
        let newId = createId();
        command.node.pins[newId] = command.node.pins[pin];
        command.node.pins[newId].id = newId;
        delete command.node.pins[pin];
    }

    const generic_command = {
        ...command,
        command_type: ICommandType.AddNode,
    }

    return {
        command: generic_command as any,
        node: command.node,
    }
}

export function removeCommentCommand(command: IRemoveComment): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.RemoveComment,
    }

    return generic_command as any;
}

export function upsertCommentCommand(command: IUpsertComment): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.UpsertComment,
    }

    return generic_command as any;
}

export function copyPasteCommand(command: ICopyPaste): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.CopyPaste,
    }

    return generic_command as any;
}

export function moveNodeCommand(command: IMoveNode): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.MoveNode,
    }

    return generic_command as any;
}

export function removeNodeCommand(command: IRemoveNode): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.RemoveNode,
    }

    return generic_command as any;
}

export function updateNodeCommand(command: IUpdateNode): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.UpdateNode,
    }

    return generic_command as any;
}

export function connectPinsCommand(command: IConnectPins): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.ConnectPin,
    }

    return generic_command as any;
}

export function disconnectPinsCommand(command: IDisconnectPins): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.DisconnectPin,
    }

    return generic_command as any;
}

export function upsertPinCommand(command: IUpsertPin): IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.UpsertPin,
    }

    return generic_command as any;
}

export function upsertVariableCommand(command: IUpsertVariable) : IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.UpsertVariable,
    }

    return generic_command as any;
}

export function removeVariableCommand(command: IRemoveVariable) : IGeneric {
    const generic_command = {
        ...command,
        command_type: ICommandType.RemoveVariable,
    }

    return generic_command as any;
}