"use client";

import { useTranslation } from "@flow-like/locales";
import { useSearchParams } from "next/navigation";
import { RolePermissions } from "../../../lib/permission/role-permission";
import { PermissionGate } from "../permission";
import { GroupManagement } from "./group-management";

/**
 * Suites live outside Team Management: a suite is a presentation grant, not a
 * membership one, so a private app — which has no team surface at all — must
 * still be able to curate and be invited into one.
 */
export function SuitesPage() {
	const { t } = useTranslation("settings");
	const searchParams = useSearchParams();
	const appId = searchParams.get("id");

	if (!appId) {
		return (
			<div className="p-10 text-center text-muted-foreground">
				{t("noAppSelected", "No app selected.")}
			</div>
		);
	}

	return (
		<div className="h-full overflow-y-auto px-1">
			<PermissionGate
				appId={appId}
				require={[RolePermissions.ReadTeam]}
				feature={t("suites", "Suites")}
				description={t(
					"yourRoleCannotSeeWhichSuitesThisAppBelongsTo",
					"Your role cannot see which suites this app belongs to.",
				)}
			>
				<GroupManagement appId={appId} />
			</PermissionGate>
		</div>
	);
}
