"use client";

import { useTranslation } from "@flow-like/locales";
import { KeyRoundIcon, LockIcon, ShieldQuestionIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useAppPermissions } from "../../../hooks/use-app-permissions";
import type { RolePermissions } from "../../../lib/permission/role-permission";
import { cn } from "../../../lib/utils";
import { Skeleton } from "../../ui/skeleton";
import { getPermissionLabel } from "../roles/permission-groups";

export type SectionLockKind = "permission" | "visibility";

export interface SectionLockedPanelProps {
	/** What the reader tried to open, e.g. "Team". */
	feature: string;
	/**
	 * `permission` is someone else's decision and offers nothing to click.
	 * `visibility` is a door this account can open, so it carries the key glyph,
	 * the primary accent and an action.
	 */
	kind?: SectionLockKind;
	/** One sentence on what is unreadable and why it matters. */
	description?: string;
	missing?: RolePermissions[];
	/** Whether every entry in `missing` is required. Default false (any-of). */
	requireAll?: boolean;
	roleName?: string;
	/** Only meaningful for a visibility lock — the button that unlocks it. */
	action?: ReactNode;
	className?: string;
}

/**
 * Full-panel stand-in for a section the caller may not open.
 *
 * The rule it exists to enforce: a denial must never render as an empty list.
 * "No members found" and "you cannot read the team" look identical to a reader
 * and lead to opposite next steps.
 */
export function SectionLockedPanel({
	feature,
	kind = "permission",
	description,
	missing = [],
	requireAll = false,
	roleName,
	action,
	className,
}: Readonly<SectionLockedPanelProps>) {
	const { t } = useTranslation("settings");
	const openable = kind === "visibility";
	const Glyph = openable ? KeyRoundIcon : LockIcon;
	const labels = missing
		.map((permission) => getPermissionLabel(permission))
		.filter((label): label is string => !!label);

	return (
		<div
			className={cn(
				"mx-auto flex w-full max-w-xl flex-col items-center gap-4 rounded-xl border border-dashed px-6 py-14 text-center",
				openable ? "border-primary/30 bg-primary/5" : "bg-muted/20",
				className,
			)}
		>
			<div
				className={cn(
					"grid size-12 place-items-center rounded-xl",
					openable ? "bg-primary/10" : "bg-muted",
				)}
			>
				<Glyph
					className={cn(
						"size-5",
						openable ? "text-primary" : "text-muted-foreground",
					)}
				/>
			</div>
			<div className="space-y-1.5">
				<h2 className="font-medium text-foreground">
					{openable
						? t("featureIsNotUnlockedYet", "{{feature}} is not unlocked yet", {
								feature,
							})
						: t("featureIsLocked", "{{feature}} is locked", { feature })}
				</h2>
				<p className="text-sm text-muted-foreground">
					{description ??
						t(
							"yourRoleOnThisProjectCannotOpenThisSection",
							"Your role on this project cannot open this section.",
						)}
				</p>
			</div>
			{!openable && (labels.length > 0 || roleName) && (
				<div className="w-full max-w-sm space-y-2 rounded-lg border bg-background/60 p-3 text-sm">
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
							<span className="shrink-0 text-muted-foreground">
								{labels.length > 1 && !requireAll
									? t("needsAnyOf", "Needs any of")
									: t("needs", "Needs")}
							</span>
							<span className="ml-auto text-right font-medium">
								{labels.join(", ")}
							</span>
						</div>
					)}
				</div>
			)}
			{action}
			{!openable && (
				<p className="flex items-start gap-2 text-xs text-muted-foreground">
					<ShieldQuestionIcon className="mt-0.5 size-3.5 shrink-0" />
					<span>
						{t(
							"onlyAnOwnerOrAdminOfThisProjectCanGrantThatAskOneOfThemToUpdateYourRole",
							"Only an owner or admin of this project can grant that. Ask one of them to update your role.",
						)}
					</span>
				</p>
			)}
		</div>
	);
}

export interface PermissionGateProps {
	appId: string | undefined | null;
	/** Any one of these opens the section, mirroring `ensure_any_permission!`. */
	require: RolePermissions[];
	feature: string;
	description?: string;
	/** Rendered while the role is still resolving. Defaults to a skeleton. */
	fallback?: ReactNode;
	children: ReactNode;
}

/**
 * Wraps a whole config section so it renders its real body only when the
 * caller may read it. Degrades open when the role is unknown — a local-only
 * app has no permission model, and locking it would strand the owner.
 */
export function PermissionGate({
	appId,
	require,
	feature,
	description,
	fallback,
	children,
}: Readonly<PermissionGateProps>) {
	const permissions = useAppPermissions(appId);

	if (permissions.isLoading) {
		return (
			fallback ?? (
				<div className="space-y-3 p-6">
					<Skeleton className="h-8 w-52" />
					<Skeleton className="h-40 w-full" />
				</div>
			)
		);
	}

	if (!permissions.can(...require)) {
		return (
			<SectionLockedPanel
				feature={feature}
				description={description}
				missing={require}
				roleName={permissions.roleName}
			/>
		);
	}

	return <>{children}</>;
}
