"use client";

import { createId } from "@paralleldrive/cuid2";
import { Check, ChevronDown, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
	Badge,
	Button,
	Checkbox,
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	type IBackendRole,
	Input,
	Label,
	Separator,
	Switch,
	Textarea,
} from "../../..";
import { RolePermissions } from "../../../lib";
import {
	PERMISSION_GROUPS,
	type PermissionGroup,
	countGroupPermissions,
} from "./permission-groups";

interface RoleDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	role?: IBackendRole;
	onSave: (roleData: IBackendRole) => void | Promise<void>;
}

export function RoleDialog({
	open,
	onOpenChange,
	role,
	onSave,
}: Readonly<RoleDialogProps>) {
	const [name, setName] = useState("");
	const [description, setDescription] = useState("");
	const [permissions, setPermissions] = useState(new RolePermissions());
	const [attributes, setAttributes] = useState<string[]>([]);
	const [attrInput, setAttrInput] = useState("");

	useEffect(() => {
		if (role) {
			setName(role.name);
			setDescription(role.description);
			setPermissions(new RolePermissions(role.permissions));
			setAttributes([...(role.attributes ?? [])]);
		} else {
			setName("");
			setDescription("");
			setPermissions(new RolePermissions());
			setAttributes([]);
		}
		setAttrInput("");
	}, [role, open]);

	const isOwnerRole = useMemo(
		() => !!role && new RolePermissions(role.permissions).contains(RolePermissions.Owner),
		[role],
	);

	const togglePermission = useCallback((perm: RolePermissions) => {
		if (isOwnerRole && perm.equals(RolePermissions.Owner)) return;
		setPermissions((prev) =>
			prev.contains(perm) ? prev.remove(perm) : prev.insert(perm),
		);
	}, [isOwnerRole]);

	const toggleGroup = useCallback(
		(group: PermissionGroup) => {
			setPermissions((prev) => {
				const { active, total } = countGroupPermissions(group, prev);
				if (active === total) {
					let next = prev;
					for (const p of group.permissions) {
						if (isOwnerRole && p.permission.equals(RolePermissions.Owner)) continue;
						next = next.remove(p.permission);
					}
					return next;
				}
				let next = prev;
				for (const p of group.permissions) next = next.insert(p.permission);
				return next;
			});
		},
		[isOwnerRole],
	);

	const isAdminRole = useMemo(() => permissions.contains(RolePermissions.Admin), [permissions]);

	const addAttribute = useCallback(() => {
		const val = attrInput.trim();
		if (val && !attributes.includes(val)) {
			setAttributes((prev) => [...prev, val]);
			setAttrInput("");
		}
	}, [attrInput, attributes]);

	const removeAttribute = useCallback((attr: string) => {
		setAttributes((prev) => prev.filter((a) => a !== attr));
	}, []);

	const handleSave = useCallback(async () => {
		if (!name.trim()) return;
		const now = new Date().toISOString().replace("Z", "");
		const savedAttributes = [...attributes];
		const data: IBackendRole = role
			? {
					...role,
					name: name.trim(),
					description,
					permissions: permissions.toBigInt(),
					attributes: savedAttributes,
				}
			: {
					id: createId(),
					app_id: "",
					name: name.trim(),
					description,
					permissions: permissions.toBigInt(),
					attributes: savedAttributes,
					created_at: now,
					updated_at: now,
				};
		await onSave(data);
		onOpenChange(false);
	}, [name, description, permissions, attributes, onSave, onOpenChange, role]);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-2xl max-h-[85vh] flex flex-col">
				<DialogHeader>
					<DialogTitle>{role ? "Edit Role" : "New Role"}</DialogTitle>
					<DialogDescription>
						{role
							? "Modify role settings, permissions, and attributes."
							: "Create a new role with specific permissions and attributes."}
					</DialogDescription>
				</DialogHeader>

				<div className="flex-1 min-h-0 overflow-y-auto -mx-6 px-6 [scrollbar-width:none] [-webkit-overflow-scrolling:touch] [&::-webkit-scrollbar]:hidden">
					<div className="space-y-6 py-2">
						{/* Basic info */}
						<div className="space-y-4">
							<div className="space-y-2">
								<Label htmlFor="role-name">Name</Label>
								<Input
									id="role-name"
									placeholder="e.g. Editor, Viewer..."
									value={name}
									onChange={(e) => setName(e.target.value)}
								/>
							</div>
							<div className="space-y-2">
								<Label htmlFor="role-desc">Description</Label>
								<Textarea
									id="role-desc"
									placeholder="What can this role do?"
									value={description}
									onChange={(e) => setDescription(e.target.value)}
									rows={2}
								/>
							</div>
						</div>

						<Separator />

						{/* Permissions */}
						<div className="space-y-3">
							<Label className="text-sm font-semibold">Permissions</Label>
							<div className="space-y-2">
								{PERMISSION_GROUPS.map((group) => (
									<PermissionGroupSection
										key={group.id}
										group={group}
										current={permissions}
										onTogglePermission={togglePermission}
										onToggleGroup={toggleGroup}
										disabled={isOwnerRole && group.id === "system"}
										isAdminRole={isAdminRole}
									/>
								))}
							</div>
						</div>

						<Separator />

						{/* Attributes */}
						<div className="space-y-3">
							<Label className="text-sm font-semibold">Attributes</Label>
							<p className="text-xs text-muted-foreground">
								Custom key-value tags for filtering and policy rules.
							</p>
							<div className="flex gap-2">
								<Input
								placeholder="Add attribute..."
									value={attrInput}
									onChange={(e) => setAttrInput(e.target.value)}
									onKeyDown={(e) => e.key === "Enter" && addAttribute()}
									className="flex-1"
								/>
								<Button
									onClick={addAttribute}
									variant="outline"
									size="sm"
									disabled={!attrInput.trim()}
								>
									Add
								</Button>
							</div>
							{attributes.length > 0 && (
								<div className="flex flex-wrap gap-1.5 pt-1">
									{attributes.map((attr) => (
										<Badge
											key={attr}
											variant="secondary"
											className="gap-1 pr-1 cursor-pointer"
											onClick={() => removeAttribute(attr)}
										>
											{attr}
											<X className="h-3 w-3" />
										</Badge>
									))}
								</div>
							)}
						</div>
					</div>
				</div>

				<DialogFooter className="pt-4">
					<Button variant="outline" onClick={() => onOpenChange(false)}>
						Cancel
					</Button>
					<Button onClick={handleSave} disabled={!name.trim()}>
						{role ? "Save Changes" : "Create Role"}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function PermissionGroupSection({
	group,
	current,
	onTogglePermission,
	onToggleGroup,
	disabled,
	isAdminRole,
}: Readonly<{
	group: PermissionGroup;
	current: RolePermissions;
	onTogglePermission: (perm: RolePermissions) => void;
	onToggleGroup: (group: PermissionGroup) => void;
	disabled?: boolean;
	isAdminRole?: boolean;
}>) {
	const isNonSystemGroup = group.id !== "system";
	const impliedByAdmin = isAdminRole && isNonSystemGroup;
	const { active, total } = countGroupPermissions(group, current);
	const effectiveAllActive = impliedByAdmin || active === total;
	const someActive = !effectiveAllActive && active > 0;
	const GroupIcon = group.icon;

	return (
		<Collapsible>
			<div className="flex items-center gap-2 rounded-lg border px-3 py-2 hover:bg-muted/50 transition-colors">
				<Checkbox
					checked={effectiveAllActive ? true : someActive ? "indeterminate" : false}
					onCheckedChange={() => !disabled && !impliedByAdmin && onToggleGroup(group)}
					disabled={disabled || impliedByAdmin}
					className="shrink-0"
				/>
				<CollapsibleTrigger className="flex items-center gap-2 flex-1 min-w-0 text-left">
					<GroupIcon className="h-4 w-4 text-muted-foreground shrink-0" />
					<span className="text-sm font-medium truncate">{group.label}</span>
					{impliedByAdmin && (
						<span className="text-[10px] text-muted-foreground italic">via Admin</span>
					)}
					<span className="text-xs text-muted-foreground tabular-nums ml-auto shrink-0">
						{impliedByAdmin ? total : active}/{total}
					</span>
					<ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0 transition-transform in-data-[state=open]:rotate-180" />
				</CollapsibleTrigger>
			</div>
			<CollapsibleContent>
				<div className="ml-8 mt-1 mb-2 space-y-1">
					{group.permissions.map((entry) => {
						const isExplicit = current.contains(entry.permission);
						const isActive = isExplicit || (impliedByAdmin ?? false);
						const EntryIcon = entry.icon;
						const isLocked =
							(disabled && entry.permission.equals(RolePermissions.Owner)) ||
							(impliedByAdmin && isNonSystemGroup);
						return (
							<label
								key={entry.label}
								className="flex items-center gap-3 rounded-md px-3 py-1.5 hover:bg-muted/40 transition-colors cursor-pointer"
							>
								<Switch
									checked={isActive}
									onCheckedChange={() => onTogglePermission(entry.permission)}
									disabled={isLocked}
									className="shrink-0 scale-75"
								/>
								<EntryIcon className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
								<div className="min-w-0 flex-1">
									<p className="text-sm leading-tight">{entry.label}</p>
									<p className="text-xs text-muted-foreground leading-tight">
										{entry.description}
									</p>
								</div>
								{isActive && (
									<Check className="h-3.5 w-3.5 text-primary shrink-0" />
								)}
							</label>
						);
					})}
				</div>
			</CollapsibleContent>
		</Collapsible>
	);
}
