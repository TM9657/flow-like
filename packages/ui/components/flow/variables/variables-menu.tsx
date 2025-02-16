import { Button } from "../../../components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "../../../components/ui/dropdown-menu";
import { Input } from "../../../components/ui/input";
import { Label } from "../../../components/ui/label";
import { Separator } from "../../../components/ui/separator";
import {
    Sheet,
    SheetContent,
    SheetDescription,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
} from "../../../components/ui/sheet";
import { type IBoard, type IVariable } from "../../../lib/schema/flow/board";
import { IVariableType } from "../../../lib/schema/flow/node";
import { useDraggable } from '@dnd-kit/core';
import { createId } from "@paralleldrive/cuid2";
import { useDebounce } from "@uidotdev/usehooks";
import { CirclePlusIcon, EllipsisVerticalIcon, EyeIcon, EyeOffIcon, GripIcon, ListIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { typeToColor } from "../utils";
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectTrigger,
    SelectValue,
} from "../../../components/ui/select"
import { Switch } from "../../../components/ui/switch";
import { VariablesMenuEdit } from "./variables-menu-edit";
import { IValueType } from "../../../lib/schema/flow/pin";
import { toast } from "sonner";
import { convertJsonToUint8Array } from "../../../lib/uint8";

export function VariablesMenu({ board, executeCommand }: Readonly<{ board: IBoard, executeCommand: (command: string, args: any, append: boolean) => Promise<any> }>) {

    async function upsertVariable(variable: IVariable) {
        await executeCommand("upsert_variable", { boardId: board.id, variable }, false)
    }

    async function removeVariable(variable: IVariable) {
        await executeCommand("remove_variable", { boardId: board.id, variable }, false)
    }

    return <div className="flex flex-col gap-2 p-4">
        <div className="flex flex-row items-center gap-4 mb-2">
            <h2>Variables</h2>
            <Button className="gap-2" onClick={async () => {
                await upsertVariable({
                    id: createId(),
                    name: "New Variable",
                    data_type: IVariableType.String,
                    exposed: false,
                    value_type: IValueType.Normal,
                    secret: false,
                    editable: true,
                    category: "General",
                    default_value: convertJsonToUint8Array(""),
                    description: ""
                })
            }}>
                <CirclePlusIcon />
                New
            </Button>
        </div>
        {Object.values(board.variables).sort((a, b) => a.name.localeCompare(b.name)).map((variable) => <Variable key={variable.id} variable={variable} onVariableChange={(variable) => {
            if(!variable.editable) {
                // toast.error("This variable is not editable")
                return;
            }
            upsertVariable(variable)
        }} />)}
    </div>
}

export function Variable({ variable, onVariableChange, preview = false }: Readonly<{ variable: IVariable, onVariableChange: (variable: IVariable) => void, preview?: boolean }>) {
    const { attributes, listeners, setNodeRef, transform } = useDraggable({
        id: variable.id,
        data: variable
    });

    const [localVariable, setLocalVariable] = useState(variable);
    const [openEdit, setOpenEdit] = useState(false);
    const debouncedVariable = useDebounce(localVariable, 500);
    useEffect(() => {
        onVariableChange(debouncedVariable)
    }, [debouncedVariable])

    const isArrayDropdown = <DropdownMenu>
        <DropdownMenuTrigger>
            <VariableIcon value_type={localVariable.value_type} data_type={localVariable.data_type} />
        </DropdownMenuTrigger>
        <DropdownMenuContent>
            <DropdownMenuItem className="gap-2" onClick={(e) => {
                setLocalVariable(old => ({ ...old, value_type: IValueType.Normal }))
                e.stopPropagation()
            }}>
                <div className="w-4 h-2 rounded-full" style={{ backgroundColor: typeToColor(localVariable.data_type) }} /> Single
            </DropdownMenuItem>
            <DropdownMenuItem className="gap-2" onClick={(e) => {
                setLocalVariable(old => ({ ...old, value_type: IValueType.Array }))
                e.stopPropagation()
            }}>
                <GripIcon className="w-4 h-4" style={{ color: typeToColor(localVariable.data_type) }} /> Array
            </DropdownMenuItem>
            <DropdownMenuItem className="gap-2" onClick={(e) => {
                setLocalVariable(old => ({ ...old, value_type: IValueType.HashSet }))
                e.stopPropagation()
            }}>
                <EllipsisVerticalIcon className="w-4 h-4" style={{ color: typeToColor(localVariable.data_type) }} /> Set
            </DropdownMenuItem>
            <DropdownMenuItem className="gap-2" onClick={(e) => {
                setLocalVariable(old => ({ ...old, value_type: IValueType.HashMap }))
                e.stopPropagation()
            }}>
                <ListIcon className="w-4 h-4" style={{ color: typeToColor(localVariable.data_type) }} /> Map
            </DropdownMenuItem>
        </DropdownMenuContent>
    </DropdownMenu>

    const element = <div
        ref={setNodeRef}
        // style={{ transform: `translate(${transform?.x ?? 0}px, ${transform?.y ?? 0}px)` }}
        className={`relative flex flex-row items-center justify-between gap-2 border p-1 px-2 rounded-md bg-card text-card-foreground z-100 ${transform && "opacity-0"} ${!variable.editable ? "text-muted-foreground " : ""}`}
        {...listeners}
        {...attributes}
    >
        <div className="flex flex-row gap-2 items-center" data-no-dnd>
            {isArrayDropdown}
            <p className={`${!variable.editable ? "text-muted-foreground" : ""}`}>{localVariable.name}</p>
        </div>
        <div className="flex flex-row items-center gap-2" data-no-dnd>
            <Button disabled={!variable.editable} variant={"ghost"} size={"icon"} onClick={(event) => {
                event.stopPropagation()
                setLocalVariable(old => ({ ...old, exposed: !old.exposed }))
                console.log(localVariable)
            }} >
                {localVariable.exposed ? <EyeIcon className="w-4 h-4" /> : <EyeOffIcon className="w-4 h-4" />}
            </Button>
        </div>
    </div>

    if (preview) return element;

    return <Sheet open={openEdit} onOpenChange={(open) => {
        if(!localVariable.editable) return;
        setOpenEdit(open)
    }}>
        <SheetTrigger>
            {element}
        </SheetTrigger>
        <SheetContent>
            <SheetHeader>
                <SheetTitle className="flex flex-row items-center gap-2">Edit Variable {isArrayDropdown}</SheetTitle>
                <SheetDescription className="flex flex-col gap-6 text-foreground">
                    <p className="text-muted-foreground">Edit the variable properties to your liking.</p>
                    <Separator />
                    <div className="grid w-full max-w-sm items-center gap-1.5">
                        <Label htmlFor="name">Variable Name</Label>
                        <Input value={localVariable.name} onChange={(e) => { setLocalVariable(old => ({ ...old, name: e.target.value })) }} id="name" placeholder="Name" />
                    </div>
                    <div className="grid w-full max-w-sm items-center gap-1.5">
                        <div className="flex flex-row items-center gap-2">{isArrayDropdown}<Label htmlFor="var_type">Variable Type</Label></div>
                        <Select value={localVariable.data_type} onValueChange={(value) => setLocalVariable(old => ({ ...old, data_type: value as IVariableType }))}>
                            <SelectTrigger id="var_type" className="w-full">
                                <SelectValue placeholder="Variable Type" />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectGroup>
                                    <SelectLabel>Variable Type</SelectLabel>
                                    <SelectItem value="Boolean">Boolean</SelectItem>
                                    <SelectItem value="Date">Date</SelectItem>
                                    <SelectItem value="Float">Float</SelectItem>
                                    <SelectItem value="Integer">Integer</SelectItem>
                                    <SelectItem value="Generic">Generic</SelectItem>
                                    <SelectItem value="PathBuf">PathBuf</SelectItem>
                                    <SelectItem value="String">String</SelectItem>
                                    <SelectItem value="Struct">Struct</SelectItem>
                                </SelectGroup>
                            </SelectContent>
                        </Select>
                    </div>
                    <div className="flex flex-col gap-1">
                        <div className="flex items-center space-x-2">
                            <Switch checked={localVariable.exposed} onCheckedChange={(checked) => setLocalVariable(old => ({ ...old, exposed: checked }))} id="exposed" />
                            <Label htmlFor="exposed">Is Exposed?</Label>
                        </div>
                        <small className="text-[0.8rem] text-muted-foreground">If you expose a variable the context (Vault, App, etc. will be able to configure this)</small>
                    </div>
                    <div className="flex flex-col gap-1">
                        <div className="flex items-center space-x-2">
                            <Switch checked={localVariable.secret} onCheckedChange={(checked) => setLocalVariable(old => ({ ...old, secret: checked }))} id="secret" />
                            <Label htmlFor="secret">Is Secret?</Label>
                        </div>
                        <small className="text-[0.8rem] text-muted-foreground">A secret Variable will be covered for input (e.g passwords)</small>
                    </div>
                    <Separator/>
                    {!localVariable.exposed && <VariablesMenuEdit variable={localVariable} updateVariable={async (variable) => setLocalVariable(variable)} />}
                </SheetDescription>
            </SheetHeader>
        </SheetContent>
    </Sheet>
}

function VariableIcon({value_type, data_type, className} : Readonly<{value_type: IValueType, data_type: IVariableType, className?: string}>) {
    if (value_type === IValueType.Array) return <GripIcon className={`w-4 h-4 ${className}`} style={{ color: typeToColor(data_type) }} />
    if (value_type === IValueType.HashSet) return <EllipsisVerticalIcon className={`w-4 h-4 ${className}`} style={{ color: typeToColor(data_type) }} />
    if (value_type === IValueType.HashMap) return <ListIcon className={`w-4 h-4 ${className}`} style={{ color: typeToColor(data_type) }} />
    return <div className={`w-4 h-2 min-h-2 min-w-4 rounded-full ${className}`} style={{ backgroundColor: typeToColor(data_type) }} />
}