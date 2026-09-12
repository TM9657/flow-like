"use client";

import { useTranslation } from "@flow-like/locales";
import { EyeIcon, LockIcon } from "lucide-react";
import type { ReactNode } from "react";
import type { RolePermissions } from "../../../lib/permission/role-permission";
import { cn } from "../../../lib/utils";
import { getPermissionLabel } from "../roles/permission-groups";

export type PermissionNoticeTone = "blocked" | "readOnly";

const TONE = {
	/** The data behind this panel could not be read at all. */
	blocked: {
		Icon: LockIcon,
		className: "border-border bg-muted/50",
		iconClassName: "text-muted-foreground",
	},
	/** The data is visible but this role cannot change it. */
	readOnly: {
		Icon: EyeIcon,
		className: "border-border bg-muted/30",
		iconClassName: "text-muted-foreground",
	},
} as const;

export interface PermissionNoticeProps {
	tone?: PermissionNoticeTone;
	title: string;
	description?: string;
	/** Named in the notice so the reader knows what to ask for. */
	missing?: RolePermissions[];
	/**
	 * Whether every entry in `missing` is required. Default false, matching the
	 * `ensure_any_permission!` shape most gates use; set it where the server
	 * demands all of them, or where `missing` is the set the caller lacks.
	 */
	requireAll?: boolean;
	action?: ReactNode;
	className?: string;
}

/**
 * Inline counterpart to {@link SectionLockedPanel}, for a panel that still has
 * something to show. Use it wherever a partial denial would otherwise render
 * as a confident zero — an empty table, a 0 in a stat tile, a disabled button
 * with no explanation.
 */
export function PermissionNotice({
	tone = "blocked",
	title,
	description,
	missing = [],
	requireAll = false,
	action,
	className,
}: Readonly<PermissionNoticeProps>) {
	const { t } = useTranslation("settings");
	const { Icon, className: toneClass, iconClassName } = TONE[tone];
	const labels = missing
		.map((permission) => getPermissionLabel(permission))
		.filter((label): label is string => !!label);

	return (
		<div
			className={cn(
				"flex items-start gap-3 rounded-lg border p-3 text-sm",
				toneClass,
				className,
			)}
		>
			<Icon className={cn("mt-0.5 size-4 shrink-0", iconClassName)} />
			<div className="min-w-0 flex-1 space-y-1">
				<p className="font-medium text-foreground">{title}</p>
				{description && (
					<p className="text-xs text-muted-foreground">{description}</p>
				)}
				{labels.length > 0 && (
					<p className="text-xs text-muted-foreground">
						{labels.length > 1 && !requireAll
							? t("needsAnyOfLabels", "Needs any of: {{labels}}", {
									labels: labels.join(", "),
								})
							: t("needsLabels", "Needs {{labels}}", {
									labels: labels.join(", "),
								})}
					</p>
				)}
			</div>
			{action}
		</div>
	);
}
