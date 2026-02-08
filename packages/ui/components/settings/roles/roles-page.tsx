"use client";

import { createId } from "@paralleldrive/cuid2";
import { Plus, Shield, Star } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useCallback, useMemo, useState } from "react";
import {
	Button,
	Card,
	CardDescription,
	CardHeader,
	type IBackendRole,
	Input,
	useBackend,
	useInvoke,
} from "../../..";
import { RolePermissions } from "../../../lib";
import { RoleCard } from "./role-card";
import { RoleDialog } from "./role-dialog";

export function RolesPage() {
	const searchParams = useSearchParams();
	const appId = searchParams.get("id");
	const backend = useBackend();
	const roles = useInvoke(
		backend.roleState.getRoles,
		backend.roleState,
		[appId!],
		typeof appId === "string",
	);
	const [isDialogOpen, setIsDialogOpen] = useState(false);
	const [editingRole, setEditingRole] = useState<IBackendRole | undefined>();
	const [searchTerm, setSearchTerm] = useState("");

	const { filteredRoles, defaultRole } = useMemo(() => {
		if (!roles.data) return { filteredRoles: [], defaultRole: undefined };
		const defaultRoleId = roles.data[0];
		const allRoles = roles.data[1];
		const defaultRole = allRoles.find((role) => role.id === defaultRoleId);
		const filtered = allRoles.filter(
			(role) =>
				role.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
				role.description.toLowerCase().includes(searchTerm.toLowerCase()),
		);
		const sorted = filtered.toSorted((a, b) => {
			const permA = new RolePermissions(a.permissions);
			const permB = new RolePermissions(b.permissions);
			if (permA.contains(RolePermissions.Owner)) return -1;
			if (permB.contains(RolePermissions.Owner)) return 1;
			if (permA.contains(RolePermissions.Admin)) return -1;
			if (permB.contains(RolePermissions.Admin)) return 1;
			return a.name.localeCompare(b.name);
		});
		return { filteredRoles: sorted, defaultRole };
	}, [roles.data, searchTerm]);

	const handleCreateRole = () => {
		setEditingRole(undefined);
		setIsDialogOpen(true);
	};

	const handleEditRole = (role: IBackendRole) => {
		setEditingRole(role);
		setIsDialogOpen(true);
	};

	const handleSaveRole = useCallback(
		async (roleData: IBackendRole) => {
			if (!appId) return;
			roleData.app_id = appId;
			await backend.roleState.upsertRole(appId, roleData);
			await roles.refetch();
		},
		[appId, backend],
	);

	const handleDuplicateRole = useCallback(
		async (role: IBackendRole) => {
			if (!appId) return;
			const perm = new RolePermissions(role.permissions);
			const cleaned = perm
				.remove(RolePermissions.Owner)
				.remove(RolePermissions.Admin);
			await backend.roleState.upsertRole(appId, {
				...role,
				id: createId(),
				name: `${role.name} (Copy)`,
				permissions: cleaned.toBigInt(),
			});
			await roles.refetch();
		},
		[appId, backend],
	);

	const handleDeleteRole = useCallback(
		async (roleId: string) => {
			if (!appId) return;
			await backend.roleState.deleteRole(appId, roleId);
			await roles.refetch();
		},
		[appId, backend],
	);

	const handleSetDefaultRole = useCallback(
		async (roleId: string) => {
			if (!appId) return;
			await backend.roleState.makeRoleDefault(appId, roleId);
			await roles.refetch();
		},
		[appId, backend],
	);

	return (
		<div className="flex flex-col h-full max-h-full overflow-hidden gap-4 p-4">
			<div className="flex items-center justify-between gap-4">
				<div>
					<h1 className="text-2xl font-bold tracking-tight">Roles</h1>
					<p className="text-sm text-muted-foreground">
						Define roles with permissions and attributes to control access.
					</p>
				</div>
				<Button onClick={handleCreateRole} size="sm">
					<Plus className="h-4 w-4 mr-2" />
					New Role
				</Button>
			</div>

			{defaultRole && (
				<Card className="border-l-4 border-l-primary/60 bg-muted/30 rounded-md">
					<CardHeader className="py-2">
						<div className="flex items-center gap-2">
							<Star className="h-4 w-4 text-primary" />
							<CardDescription className="text-sm m-0">
								New members are assigned the{" "}
								<strong>{defaultRole.name}</strong> role by default.
							</CardDescription>
						</div>
					</CardHeader>
				</Card>
			)}

			<Input
				placeholder="Search roles..."
				value={searchTerm}
				onChange={(e) => setSearchTerm(e.target.value)}
				className="max-w-sm"
			/>

			<div className="flex-1 overflow-auto">
				<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
					{filteredRoles.map((role) => (
						<RoleCard
							key={role.id}
							role={role}
							isDefault={role.id === defaultRole?.id}
							onEdit={handleEditRole}
							onDuplicate={handleDuplicateRole}
							onDelete={handleDeleteRole}
							onSetDefault={handleSetDefaultRole}
						/>
					))}
				</div>

				{filteredRoles.length === 0 && (
					<div className="text-center py-12">
						<Shield className="h-8 w-8 mx-auto text-muted-foreground mb-3" />
						<h3 className="text-base font-semibold mb-1">No roles found</h3>
						<p className="text-sm text-muted-foreground mb-4">
							{searchTerm
								? "Try a different search term."
								: "Create your first role to get started."}
						</p>
						{!searchTerm && (
							<Button onClick={handleCreateRole} size="sm" variant="outline">
								<Plus className="h-4 w-4 mr-2" />
								Create Role
							</Button>
						)}
					</div>
				)}
			</div>

			<RoleDialog
				open={isDialogOpen}
				onOpenChange={setIsDialogOpen}
				role={editingRole}
				onSave={handleSaveRole}
			/>
		</div>
	);
}
