"use client";

import { useTranslation } from "@flow-like/locales";
import { LockIcon, ShieldQuestionIcon } from "lucide-react";
import type { RolePermissions } from "../../../lib/permission/role-permission";
import {
	AlertDialog,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "../../ui/alert-dialog";
import { Button } from "../../ui/button";
import { getPermissionLabel } from "../roles/permission-groups";

export interface PermissionLockDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	/** Section the user tried to reach, e.g. "Team". */
	feature: string;
	/** One sentence on what the section needs. */
	reason: string;
	/** Any one of these would open the section. */
	missing: RolePermissions[];
	/** The caller's role name, when it could be read. */
	roleName?: string;
}

/**
 * Counterpart to the visibility upgrade dialog, for a lock the user cannot
 * lift. It offers no action on purpose: the only honest next step is asking
 * someone with `Admin`, so the dialog names the permission and stops there
 * rather than dangling a button that would 403.
 */
export function PermissionLockDialog({
	open,
	onOpenChange,
	feature,
	reason,
	missing,
	roleName,
}: Readonly<PermissionLockDialogProps>) {
	const { t } = useTranslation("settings");
	const labels = missing
		.map((permission) => getPermissionLabel(permission))
		.filter((label): label is string => !!label);

	return (
		<AlertDialog open={open} onOpenChange={onOpenChange}>
			<AlertDialogContent className="sm:max-w-md">
				<AlertDialogHeader>
					<div className="flex items-center gap-3">
						<div className="p-2 rounded-full bg-muted">
							<LockIcon className="h-5 w-5 text-muted-foreground" />
						</div>
						<AlertDialogTitle className="text-left">
							{t("featureIsLocked", "{{feature}} is locked", { feature })}
						</AlertDialogTitle>
					</div>
					<AlertDialogDescription className="text-left text-muted-foreground">
						{reason}
					</AlertDialogDescription>
				</AlertDialogHeader>

				<div className="space-y-3 py-2">
					<div className="rounded-lg border bg-muted/40 p-3 space-y-2 text-sm">
						{roleName && (
							<div className="flex items-center gap-2">
								<span className="text-muted-foreground">
									{t("yourRole", "Your role")}
								</span>
								<span className="ml-auto font-medium">{roleName}</span>
							</div>
						)}
						{labels.length > 0 && (
							<div className="flex items-start gap-2">
								<span className="text-muted-foreground shrink-0">
									{labels.length > 1
										? t("needsAnyOf", "Needs any of")
										: t("needs", "Needs")}
								</span>
								<span className="ml-auto text-right font-medium">
									{labels.join(", ")}
								</span>
							</div>
						)}
					</div>
					<p className="flex items-start gap-2 text-xs text-muted-foreground">
						<ShieldQuestionIcon className="mt-0.5 size-3.5 shrink-0" />
						<span>
							{t(
								"onlyAnOwnerOrAdminOfThisProjectCanGrantThatAskOneOfThemToUpdateYourRole",
								"Only an owner or admin of this project can grant that. Ask one of them to update your role.",
							)}
						</span>
					</p>
				</div>

				<AlertDialogFooter>
					<Button
						variant="outline"
						className="w-full sm:w-auto"
						onClick={() => onOpenChange(false)}
					>
						{t("close", "Close")}
					</Button>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}
