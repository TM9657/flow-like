"use client";

import {
	AppReviewsSection,
	RolePermissions,
	useAppPermissions,
	useBackend,
	useInvalidateInvoke,
	useInvoke,
} from "@flow-like/flow-like-ui";
import { ProjectDashboard } from "@flow-like/flow-like-ui/components/settings/dashboard";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback } from "react";
import {
	AppAccessSection,
	AppComplianceSection,
} from "./visibility-status-switcher";

export default function DashboardPage() {
	const backend = useBackend();
	const router = useRouter();
	const invalidate = useInvalidateInvoke();
	const searchParams = useSearchParams();
	const id = searchParams.get("id") ?? "";
	const permissions = useAppPermissions(id);
	// Visibility, forking and the conformity assessment are all `Owner`-guarded
	// server-side, so the slots get the real answer instead of a literal `true`.
	// A local-only project has no role table, and `can` degrades open there.
	const canEditApp = permissions.can(RolePermissions.Owner);

	const app = useInvoke(
		backend.appState.getApp,
		backend.appState,
		[id],
		id.length > 0,
	);
	const metadata = useInvoke(
		backend.appState.getAppMeta,
		backend.appState,
		[id],
		id.length > 0,
	);

	const refreshApp = useCallback(async () => {
		await app.refetch();
		await invalidate(backend.appState.getApps, []);
	}, [app, invalidate, backend.appState]);

	if (!id || !app.data) {
		return (
			<ProjectDashboard appId={id} onDeleted={() => router.push("/library")} />
		);
	}

	return (
		<ProjectDashboard
			appId={id}
			onDeleted={() => router.push("/library")}
			slots={{
				access: (
					<AppAccessSection
						localApp={app.data}
						appName={metadata.data?.name ?? id}
						canEdit={canEditApp}
						refreshApp={refreshApp}
					/>
				),
				compliance: (
					<AppComplianceSection localApp={app.data} canEdit={canEditApp} />
				),
				reviews: <AppReviewsSection appId={id} onReviewChanged={refreshApp} />,
			}}
		/>
	);
}
