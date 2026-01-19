import {
	useDraggable,
	/* DndContext, type DragEndEvent, PointerSensor, useSensor, useSensors, closestCenter, */ useDroppable,
} from "@dnd-kit/core";
import {
	ChevronDown,
	ChevronRight,
	CircleDotIcon,
	CirclePlusIcon,
	EllipsisVerticalIcon,
	EyeIcon,
	EyeOffIcon,
	FolderIcon,
	GripIcon,
	ListIcon,
	Trash2Icon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "../../../components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "../../../components/ui/dropdown-menu";
import { Input } from "../../../components/ui/input";
import { Label } from "../../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
	SelectValue,
} from "../../../components/ui/select";
import { Separator } from "../../../components/ui/separator";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "../../../components/ui/sheet";
import { Switch } from "../../../components/ui/switch";
import {
	type IGenericCommand,
	removeVariableCommand,
	upsertVariableCommand,
} from "../../../lib";
import type { IBoard, IVariable } from "../../../lib/schema/flow/board";
import { IVariableType } from "../../../lib/schema/flow/node";
import { IValueType } from "../../../lib/schema/flow/pin";
import { convertJsonToUint8Array } from "../../../lib/uint8";
import { typeToColor } from "../utils";
import { NewVariableDialog } from "./new-variable-dialog";
import { VariablesMenuEdit } from "./variables-menu-edit";

export function VariablesMenu({
	board,
	executeCommand,
}: Readonly<{
	board: IBoard;
	executeCommand: (command: IGenericCommand, append: boolean) => Promise<any>;
}>) {
	const [showNewVariableDialog, setShowNewVariableDialog] = useState(false);

	const upsertVariable = useCallback(
		async (variable: IVariable) => {
			const oldVariable = board.variables[variable.id];
			if (oldVariable === variable) return;
			const command = upsertVariableCommand({
				variable,
			});

			await executeCommand(command, false);
		},
		[board],
	);

	const removeVariable = useCallback(
		async (variable: IVariable) => {
			const command = removeVariableCommand({
				variable,
			});
			await executeCommand(command, false);
		},
		[board],
	);

	const tree = useMemo(
		() => buildCategoryTree(Object.values(board.variables)),
		[board.variables],
	);

	// Listen for cross-folder drops dispatched by FlowWrapper
	useEffect(() => {
		const handler = (e: Event) => {
			const { variable, targetPath } = (e as CustomEvent).detail as {
				variable: IVariable;
				targetPath: string;
			};
			if (!variable?.editable) return;
			const nextCategory = targetPath === "__root" ? undefined : targetPath;
			if (variable.category === nextCategory) return;
			void upsertVariable({ ...variable, category: nextCategory });
		};
		document.addEventListener(
			"variables-folder-drop",
			handler as EventListener,
		);
		return () =>
			document.removeEventListener(
				"variables-folder-drop",
				handler as EventListener,
			);
	}, [upsertVariable]);

	return (
		<div className="flex flex-col gap-2 p-4">
			<div className="flex flex-row items-center gap-4 mb-2">
				<h2>Variables</h2>
				<Button
					className="gap-2"
					onClick={() => setShowNewVariableDialog(true)}
				>
					<CirclePlusIcon />
					New
				</Button>
			</div>

			<NewVariableDialog
				open={showNewVariableDialog}
				onOpenChange={setShowNewVariableDialog}
				onCreateVariable={upsertVariable}
			/>

			<CategoryTree
				root={tree}
				onVariableChange={(variable) => {
					if (!variable.editable) return;
					upsertVariable(variable);
				}}
				onVariableDeleted={(variable) => {
					if (!variable.editable) return;
					removeVariable(variable);
				}}
			/>
		</div>
	);
}

export function Variable({
	variable,
	onVariableChange,
	onVariableDeleted,
	preview = false,
}: Readonly<{
	variable: IVariable;
	onVariableDeleted: (variable: IVariable) => void;
	onVariableChange: (variable: IVariable) => void;
	preview?: boolean;
}>) {
	const { attributes, listeners, setNodeRef, transform } = useDraggable({
		id: variable.id,
		data: variable,
	});

	const [localVariable, setLocalVariable] = useState(variable);
	const [openEdit, setOpenEdit] = useState(false);

	const saveVariable = useCallback(() => {
		if (localVariable === variable) return;
		onVariableChange(localVariable);
	}, [localVariable, variable]);

	function defaultValueFromType(
		valueType: IValueType,
		variableType: IVariableType,
	) {
		if (valueType === IValueType.Array) {
			return [];
		}
		if (valueType === IValueType.HashSet) {
			return new Set();
		}
		if (valueType === IValueType.HashMap) {
			return new Map();
		}
		switch (variableType) {
			case IVariableType.Boolean:
				return false;
			case IVariableType.Date:
				return new Date().toISOString();
			case IVariableType.Float:
				return 0.0;
			case IVariableType.Integer:
				return 0;
			case IVariableType.Generic:
				return null;
			case IVariableType.PathBuf:
				return "";
			case IVariableType.String:
				return "";
			case IVariableType.Struct:
				return {};
			case IVariableType.Byte:
				return null;
			case IVariableType.Execution:
				return null;
		}
	}

	const isArrayDropdown = (
		<DropdownMenu>
			<DropdownMenuTrigger>
				<ValueTypeIcon
					value_type={localVariable.value_type}
					data_type={localVariable.data_type}
				/>
			</DropdownMenuTrigger>
			<DropdownMenuContent>
				<DropdownMenuItem
					className="gap-2"
					onClick={(e) => {
						setLocalVariable((old) => ({
							...old,
							value_type: IValueType.Normal,
							default_value: convertJsonToUint8Array(
								defaultValueFromType(IValueType.Normal, old.data_type),
							),
						}));
						e.stopPropagation();
					}}
				>
					<div
						className="w-4 h-2 rounded-full"
						style={{ backgroundColor: typeToColor(localVariable.data_type) }}
					/>{" "}
					Single
				</DropdownMenuItem>
				<DropdownMenuItem
					className="gap-2"
					onClick={(e) => {
						setLocalVariable((old) => ({
							...old,
							value_type: IValueType.Array,
							default_value: convertJsonToUint8Array(
								defaultValueFromType(IValueType.Array, old.data_type),
							),
						}));
						e.stopPropagation();
					}}
				>
					<GripIcon
						className="w-4 h-4"
						style={{ color: typeToColor(localVariable.data_type) }}
					/>{" "}
					Array
				</DropdownMenuItem>
				<DropdownMenuItem
					className="gap-2"
					onClick={(e) => {
						setLocalVariable((old) => ({
							...old,
							value_type: IValueType.HashSet,
							default_value: convertJsonToUint8Array(
								defaultValueFromType(IValueType.HashSet, old.data_type),
							),
						}));
						e.stopPropagation();
					}}
				>
					<EllipsisVerticalIcon
						className="w-4 h-4"
						style={{ color: typeToColor(localVariable.data_type) }}
					/>{" "}
					Set
				</DropdownMenuItem>
				<DropdownMenuItem
					className="gap-2"
					onClick={(e) => {
						setLocalVariable((old) => ({
							...old,
							value_type: IValueType.HashMap,
							default_value: convertJsonToUint8Array(
								defaultValueFromType(IValueType.HashMap, old.data_type),
							),
						}));
						e.stopPropagation();
					}}
				>
					<ListIcon
						className="w-4 h-4"
						style={{ color: typeToColor(localVariable.data_type) }}
					/>{" "}
					Map
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);

	const element = (
		<div
			ref={setNodeRef}
			className={`relative flex w-full flex-row items-center justify-between gap-2 border p-1 px-2 rounded-md bg-card text-card-foreground ${transform ? "opacity-40" : ""} ${!variable.editable ? "text-muted-foreground" : ""}`}
			{...listeners}
			{...attributes}
		>
			<div className="flex flex-row gap-2 items-center" data-no-dnd>
				{isArrayDropdown}
				<p
					className={`text-start line-clamp-2 ${!variable.editable ? "text-muted-foreground" : ""}`}
				>
					{localVariable.name}
				</p>
			</div>
			<div className="flex flex-row items-center gap-2" data-no-dnd>
				<Button
					disabled={!variable.editable}
					variant="ghost"
					size="icon"
					onClick={(event) => {
						event.stopPropagation();
						setLocalVariable((old) => ({ ...old, exposed: !old.exposed }));
					}}
				>
					{localVariable.exposed ? (
						<EyeIcon className="w-4 h-4" />
					) : (
						<EyeOffIcon className="w-4 h-4" />
					)}
				</Button>
			</div>
		</div>
	);

	if (preview) return element;

	const selectPreviewElement = useCallback((type: IVariableType) => {
		return (
			<div className="flex items-center gap-2">
				<div
					className="size-2 rounded-full"
					style={{ backgroundColor: typeToColor(type) }}
				/>
				<span>{type}</span>
			</div>
		);
	}, []);

	const valueTypePreviewElement = useCallback(
		(valueType: IValueType) => {
			const color = typeToColor(localVariable.data_type);
			const iconClass = "w-3.5 h-3.5";

			const getIcon = () => {
				switch (valueType) {
					case IValueType.Normal:
						return <CircleDotIcon className={iconClass} style={{ color }} />;
					case IValueType.Array:
						return <GripIcon className={iconClass} style={{ color }} />;
					case IValueType.HashSet:
						return (
							<EllipsisVerticalIcon className={iconClass} style={{ color }} />
						);
					case IValueType.HashMap:
						return <ListIcon className={iconClass} style={{ color }} />;
					default:
						return null;
				}
			};

			const getLabel = () => {
				switch (valueType) {
					case IValueType.Normal:
						return "Single";
					case IValueType.Array:
						return "Array";
					case IValueType.HashSet:
						return "Set";
					case IValueType.HashMap:
						return "Map";
					default:
						return valueType;
				}
			};

			return (
				<div className="flex items-center gap-2">
					{getIcon()}
					<span>{getLabel()}</span>
				</div>
			);
		},
		[localVariable.data_type],
	);

	return (
		<Sheet
			open={openEdit}
			onOpenChange={(open) => {
				if (!localVariable.editable) return;
				setOpenEdit(open);
				if (!open) {
					saveVariable();
				}
			}}
		>
			<SheetTrigger asChild>{element}</SheetTrigger>
			<SheetContent className="flex flex-col gap-6 max-h-screen overflow-hidden px-3 pt-2 pb-4">
				<SheetHeader>
					<SheetTitle className="flex flex-row items-center gap-2">
						Edit Variable
					</SheetTitle>
					<SheetDescription className="flex flex-col gap-6 text-foreground">
						<p className="text-muted-foreground">
							Edit the variable properties to your liking.
						</p>
						<Separator />
					</SheetDescription>
				</SheetHeader>

				<div className="grid w-full max-w-sm items-center gap-1.5">
					<Label htmlFor="name">Variable Name</Label>
					<Input
						value={localVariable.name}
						onChange={(e) => {
							setLocalVariable((old) => ({ ...old, name: e.target.value }));
						}}
						id="name"
						placeholder="Name"
					/>
				</div>

				<div className="grid w-full max-w-sm items-center gap-1.5">
					<Label htmlFor="category">Category</Label>
					<Input
						id="category"
						value={localVariable.category ?? ""}
						onChange={(e) => {
							const v = e.target.value;
							setLocalVariable((old) => ({
								...old,
								category: v.trim() === "" ? undefined : v,
							}));
						}}
						placeholder="e.g. Main/Bools"
					/>
					<small className="text-[0.8rem] text-muted-foreground">
						Use “/” to create nested folders. Leave empty for top-level.
					</small>
				</div>

				<div className="grid w-full max-w-sm items-center gap-1.5">
					<Label htmlFor="var_type">Variable Type</Label>
					<div className="flex flex-row gap-2">
						<Select
							value={localVariable.data_type}
							onValueChange={(value) =>
								setLocalVariable((old) => ({
									...old,
									data_type: value as IVariableType,
									default_value: convertJsonToUint8Array(
										defaultValueFromType(
											old.value_type,
											value as IVariableType,
										),
									),
								}))
							}
						>
							<SelectTrigger id="var_type" className="flex-1">
								<SelectValue placeholder="Data Type" />
							</SelectTrigger>
							<SelectContent>
								<SelectGroup>
									<SelectLabel>Data Type</SelectLabel>
									<SelectItem value="Boolean">
										{selectPreviewElement(IVariableType.Boolean)}
									</SelectItem>
									<SelectItem value="Date">
										{selectPreviewElement(IVariableType.Date)}
									</SelectItem>
									<SelectItem value="Float">
										{selectPreviewElement(IVariableType.Float)}
									</SelectItem>
									<SelectItem value="Integer">
										{selectPreviewElement(IVariableType.Integer)}
									</SelectItem>
									<SelectItem value="Generic">
										{selectPreviewElement(IVariableType.Generic)}
									</SelectItem>
									<SelectItem value="PathBuf">
										{selectPreviewElement(IVariableType.PathBuf)}
									</SelectItem>
									<SelectItem value="String">
										{selectPreviewElement(IVariableType.String)}
									</SelectItem>
									<SelectItem value="Struct">
										{selectPreviewElement(IVariableType.Struct)}
									</SelectItem>
									<SelectItem value="Byte">
										{selectPreviewElement(IVariableType.Byte)}
									</SelectItem>
								</SelectGroup>
							</SelectContent>
						</Select>
						<Select
							value={localVariable.value_type}
							onValueChange={(value) =>
								setLocalVariable((old) => ({
									...old,
									value_type: value as IValueType,
									default_value: convertJsonToUint8Array(
										defaultValueFromType(value as IValueType, old.data_type),
									),
								}))
							}
						>
							<SelectTrigger className="w-28">
								<SelectValue placeholder="Value Type" />
							</SelectTrigger>
							<SelectContent>
								<SelectGroup>
									<SelectLabel>Value Type</SelectLabel>
									<SelectItem value="Normal">
										{valueTypePreviewElement(IValueType.Normal)}
									</SelectItem>
									<SelectItem value="Array">
										{valueTypePreviewElement(IValueType.Array)}
									</SelectItem>
									<SelectItem value="HashSet">
										{valueTypePreviewElement(IValueType.HashSet)}
									</SelectItem>
									<SelectItem value="HashMap">
										{valueTypePreviewElement(IValueType.HashMap)}
									</SelectItem>
								</SelectGroup>
							</SelectContent>
						</Select>
					</div>
				</div>

				<div className="flex flex-col gap-1">
					<div className="flex items-center space-x-2">
						<Switch
							checked={localVariable.exposed}
							onCheckedChange={(checked) =>
								setLocalVariable((old) => ({ ...old, exposed: checked }))
							}
							id="exposed"
						/>
						<Label htmlFor="exposed">Is Exposed?</Label>
					</div>
					<small className="text-[0.8rem] text-muted-foreground">
						If you expose a variable it will be visible in the configuration tab
						of your App.
					</small>
				</div>

				<div className="flex flex-col gap-1">
					<div className="flex items-center space-x-2">
						<Switch
							checked={localVariable.secret}
							onCheckedChange={(checked) =>
								setLocalVariable((old) => ({ ...old, secret: checked }))
							}
							id="secret"
						/>
						<Label htmlFor="secret">Is Secret?</Label>
					</div>
					<small className="text-[0.8rem] text-muted-foreground">
						A secret variable will be covered for input (e.g passwords)
					</small>
				</div>

				<Separator />

				<div className="flex grow h-full flex-col max-h-full overflow-auto">
					{!localVariable.exposed && (
						<VariablesMenuEdit
							key={`${localVariable.value_type} - ${localVariable.data_type}-${localVariable.secret}`}
							variable={localVariable}
							updateVariable={async (variable) =>
								setLocalVariable((old) => ({
									...old,
									default_value: variable.default_value,
								}))
							}
						/>
					)}
				</div>

				<Button
					variant="destructive"
					onClick={() => {
						onVariableDeleted(variable);
					}}
				>
					<Trash2Icon />
				</Button>
			</SheetContent>
		</Sheet>
	);
}

export function ValueTypeIcon({
	value_type,
	data_type,
	className,
}: Readonly<{
	value_type: IValueType;
	data_type: IVariableType;
	className?: string;
}>) {
	if (value_type === IValueType.Array)
		return (
			<GripIcon
				className={`w-4 h-4 ${className}`}
				style={{ color: typeToColor(data_type) }}
			/>
		);
	if (value_type === IValueType.HashSet)
		return (
			<EllipsisVerticalIcon
				className={`w-4 h-4 ${className}`}
				style={{ color: typeToColor(data_type) }}
			/>
		);
	if (value_type === IValueType.HashMap)
		return (
			<ListIcon
				className={`w-4 h-4 ${className}`}
				style={{ color: typeToColor(data_type) }}
			/>
		);
	return (
		<div
			className={`w-4 h-2 min-h-2 min-w-4 rounded-full ${className}`}
			style={{ backgroundColor: typeToColor(data_type) }}
		/>
	);
}

// Category grouping

type CategoryNode = {
	name: string;
	path: string;
	vars: IVariable[];
	children: Record<string, CategoryNode>;
};

const buildCategoryTree = (vars: IVariable[]): CategoryNode => {
	const root: CategoryNode = { name: "", path: "", vars: [], children: {} };
	for (const v of vars) {
		const raw = (v as any)?.category as string | null | undefined;
		const cat = (raw ?? "").trim();
		if (!cat) {
			root.vars.push(v);
			continue;
		}
		const segments = cat
			.split("/")
			.map((s) => s.trim())
			.filter(Boolean);
		let node = root;
		let path = "";
		for (const seg of segments) {
			path = path ? `${path}/${seg}` : seg;
			if (!node.children[seg])
				node.children[seg] = { name: seg, path, vars: [], children: {} };
			node = node.children[seg];
		}
		node.vars.push(v);
	}
	return root;
};

const countRecursive = (node: CategoryNode): number =>
	node.vars.length +
	Object.values(node.children).reduce((sum, c) => sum + countRecursive(c), 0);

const CategoryTree: React.FC<{
	root: CategoryNode;
	onVariableChange: (v: IVariable) => void;
	onVariableDeleted: (v: IVariable) => void;
}> = ({ root, onVariableChange, onVariableDeleted }) => {
	const [open, setOpen] = useState<Record<string, boolean>>({});
	const isOpen = useCallback((path: string) => open[path] ?? true, [open]);
	const toggle = useCallback((path: string) => {
		setOpen((prev) => ({ ...prev, [path]: !(prev[path] ?? true) }));
	}, []);

	// Root droppable to move items to top-level (category undefined)
	const { setNodeRef: setRootRef, isOver: rootOver } = useDroppable({
		id: "__root",
	});

	const childKeys = useMemo(
		() => Object.keys(root.children).sort((a, b) => a.localeCompare(b)),
		[root.children],
	);
	const varsSorted = useMemo(
		() => [...root.vars].sort((a, b) => a.name.localeCompare(b.name)),
		[root.vars],
	);

	return (
		<div
			ref={setRootRef}
			className={`space-y-2 rounded-md ${rootOver ? "ring-1 ring-primary/40" : ""}`}
		>
			{varsSorted.length > 0 && (
				<div className="flex flex-col gap-2">
					{varsSorted.map((variable) => (
						<Variable
							key={variable.id}
							variable={variable}
							onVariableChange={onVariableChange}
							onVariableDeleted={onVariableDeleted}
						/>
					))}
				</div>
			)}
			{childKeys.length > 0 && (
				<div className="space-y-2">
					{childKeys.map((k) => (
						<FolderNode
							key={root.children[k].path}
							node={root.children[k]}
							depth={1}
							isOpen={isOpen}
							toggle={toggle}
							onVariableChange={onVariableChange}
							onVariableDeleted={onVariableDeleted}
						/>
					))}
				</div>
			)}
		</div>
	);
};

const FolderNode: React.FC<{
	node: CategoryNode;
	depth: number;
	isOpen: (path: string) => boolean;
	toggle: (path: string) => void;
	onVariableChange: (v: IVariable) => void;
	onVariableDeleted: (v: IVariable) => void;
}> = ({ node, depth, isOpen, toggle, onVariableChange, onVariableDeleted }) => {
	const { setNodeRef, isOver } = useDroppable({ id: node.path });
	const childKeys = useMemo(
		() => Object.keys(node.children).sort((a, b) => a.localeCompare(b)),
		[node.children],
	);
	const varsSorted = useMemo(
		() => [...node.vars].sort((a, b) => a.name.localeCompare(b.name)),
		[node.vars],
	);
	const total = countRecursive(node);

	return (
		<div className="rounded-md border">
			<button
				ref={setNodeRef}
				type="button"
				className={`w-full flex items-center gap-2 px-2 py-2 hover:bg-accent/50 ${isOver ? "bg-primary/5" : ""}`}
				onClick={() => toggle(node.path)}
			>
				{isOpen(node.path) ? (
					<ChevronDown className="h-4 w-4 text-muted-foreground" />
				) : (
					<ChevronRight className="h-4 w-4 text-muted-foreground" />
				)}
				<FolderIcon className="h-4 w-4 text-muted-foreground" />
				<span className="text-sm font-medium">{node.name}</span>
				<span className="ml-auto text-xs text-muted-foreground">{total}</span>
			</button>

			{isOpen(node.path) && (
				<div className="p-2 pt-1 space-y-2">
					{varsSorted.map((variable) => (
						<Variable
							key={variable.id}
							variable={variable}
							onVariableChange={onVariableChange}
							onVariableDeleted={onVariableDeleted}
						/>
					))}
					{childKeys.length > 0 && (
						<div className="mt-2 space-y-2">
							{childKeys.map((k) => (
								<FolderNode
									key={node.children[k].path}
									node={node.children[k]}
									depth={depth + 1}
									isOpen={isOpen}
									toggle={toggle}
									onVariableChange={onVariableChange}
									onVariableDeleted={onVariableDeleted}
								/>
							))}
						</div>
					)}
				</div>
			)}
		</div>
	);
};
