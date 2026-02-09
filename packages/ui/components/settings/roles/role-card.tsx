"use client";

import {
	Copy,
	Crown,
	Edit,
	MoreHorizontal,
	Shield,
	Star,
	Trash2,
	User2Icon,
} from "lucide-react";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
	Badge,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
	type IBackendRole,
	Separator,
} from "../../..";
import { RolePermissions } from "../../../lib";
import {
	PERMISSION_GROUPS,
	countGroupPermissions,
	countTotalPermissions,
} from "./permission-groups";

interface RoleCardProps {
	role: IBackendRole;
	isDefault: boolean;
	onEdit: (role: IBackendRole) => void;
	onDuplicate: (role: IBackendRole) => void;
	onDelete: (roleId: string) => void;
	onSetDefault: (roleId: string) => void;
}

export function RoleCard({
	role,
	isDefault,
	onEdit,
	onDuplicate,
	onDelete,
	onSetDefault,
}: Readonly<RoleCardProps>) {
	const perm = new RolePermissions(role.permissions);
	const isOwner = perm.contains(RolePermissions.Owner);
	const isAdmin = perm.contains(RolePermissions.Admin);
	const { active, total } = countTotalPermissions(perm);

	const RoleIcon = isOwner ? Crown : isAdmin ? Shield : User2Icon;

	return (
		<Card
			className="group relative flex flex-col transition-shadow hover:shadow-md cursor-pointer"
			onClick={() => onEdit(role)}
		>
			<CardHeader className="pb-3">
				<div className="flex items-start justify-between gap-2">
					<div className="flex items-start gap-3 min-w-0 flex-1">
						<div
							className={`shrink-0 w-9 h-9 rounded-lg flex items-center justify-center ${
								isOwner
									? "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
									: isAdmin
										? "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
										: "bg-muted text-muted-foreground"
							}`}
						>
							<RoleIcon className="h-4 w-4" />
						</div>
						<div className="min-w-0 flex-1">
							<div className="flex items-center gap-2">
								<CardTitle className="text-base font-semibold truncate">
									{role.name}
								</CardTitle>
								{isDefault && (
									<Badge
										variant="outline"
										className="text-[10px] px-1.5 py-0 shrink-0"
									>
										Default
									</Badge>
								)}
							</div>
							{role.description && (
								<CardDescription className="text-xs line-clamp-1 mt-0.5">
									{role.description}
								</CardDescription>
							)}
						</div>
					</div>

					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button
								variant="ghost"
								size="icon"
								className="h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
								onClick={(e) => e.stopPropagation()}
							>
								<MoreHorizontal className="h-4 w-4" />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent
							align="end"
							className="w-44"
							onClick={(e) => e.stopPropagation()}
						>
							<DropdownMenuItem onClick={() => onEdit(role)}>
								<Edit className="h-3.5 w-3.5 mr-2" />
								Edit
							</DropdownMenuItem>
							<DropdownMenuItem onClick={() => onDuplicate(role)}>
								<Copy className="h-3.5 w-3.5 mr-2" />
								Duplicate
							</DropdownMenuItem>
							{!isDefault && !isOwner && (
								<DropdownMenuItem onClick={() => onSetDefault(role.id)}>
									<Star className="h-3.5 w-3.5 mr-2" />
									Set as Default
								</DropdownMenuItem>
							)}
							{!isOwner && (
								<>
									<DropdownMenuSeparator />
									<AlertDialog>
										<AlertDialogTrigger asChild>
											<DropdownMenuItem
												onSelect={(e) => e.preventDefault()}
												className="text-destructive focus:text-destructive"
											>
												<Trash2 className="h-3.5 w-3.5 mr-2" />
												Delete
											</DropdownMenuItem>
										</AlertDialogTrigger>
										<AlertDialogContent>
											<AlertDialogHeader>
												<AlertDialogTitle>Delete Role</AlertDialogTitle>
												<AlertDialogDescription>
													Are you sure you want to delete &quot;{role.name}
													&quot;? Members with this role will be reassigned to
													the default role.
												</AlertDialogDescription>
											</AlertDialogHeader>
											<AlertDialogFooter>
												<AlertDialogCancel>Cancel</AlertDialogCancel>
												<AlertDialogAction
													onClick={() => onDelete(role.id)}
													className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
												>
													Delete
												</AlertDialogAction>
											</AlertDialogFooter>
										</AlertDialogContent>
									</AlertDialog>
								</>
							)}
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</CardHeader>

			<CardContent className="pt-0 flex flex-col flex-1 gap-3">
				<div className="flex items-center gap-1.5 text-xs text-muted-foreground">
					<span className="font-medium tabular-nums">
						{active}/{total}
					</span>
					<span>permissions</span>
				</div>

				<div className="flex flex-wrap gap-1.5">
					{PERMISSION_GROUPS.filter(
						(g) => countGroupPermissions(g, perm).active > 0,
					).map((group) => {
						const { active: ga, total: gt } = countGroupPermissions(
							group,
							perm,
						);
						const GroupIcon = group.icon;
						return (
							<Badge
								key={group.id}
								variant="secondary"
								className="text-[11px] gap-1 px-2 py-0.5 font-normal"
							>
								<GroupIcon className="h-3 w-3" />
								{group.label}
								<span className="text-muted-foreground ml-0.5 tabular-nums">
									{ga}/{gt}
								</span>
							</Badge>
						);
					})}
				</div>

				{(role.attributes?.length ?? 0) > 0 && (
					<>
						<Separator className="opacity-40" />
						<div className="flex flex-wrap gap-1.5">
							{role.attributes?.slice(0, 4).map((attr) => (
								<Badge
									key={attr}
									variant="outline"
									className="text-[10px] px-1.5 py-0 font-normal"
								>
									{attr}
								</Badge>
							))}
							{(role.attributes?.length ?? 0) > 4 && (
								<Badge
									variant="outline"
									className="text-[10px] px-1.5 py-0 font-normal text-muted-foreground"
								>
									+{(role.attributes?.length ?? 0) - 4}
								</Badge>
							)}
						</div>
					</>
				)}
			</CardContent>
		</Card>
	);
}
