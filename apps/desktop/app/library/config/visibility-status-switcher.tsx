"use client";

import {
	type IApp,
	type IAppVisibility,
	useBackend,
	useInvalidateInvoke,
} from "@tm9657/flow-like-ui";
import { VisibilityStatusSwitcher as SharedVisibilityStatusSwitcher } from "@tm9657/flow-like-ui/components/settings/visibility-status/visibility-status-switcher";
import { useCallback } from "react";

interface VisibilityStatusSwitcherProps {
	localApp: IApp;
	refreshApp: () => void;
	canEdit: boolean;
}

export function VisibilityStatusSwitcher({
	localApp,
	refreshApp,
	canEdit,
}: Readonly<VisibilityStatusSwitcherProps>) {
	const backend = useBackend();
	const invalidate = useInvalidateInvoke();

	const handleVisibilityChange = useCallback(
		async (appId: string, newVisibility: IAppVisibility) => {
			await backend.appState.changeAppVisibility(appId, newVisibility);
			await invalidate(backend.appState.getApp, [appId]);
			await invalidate(backend.appState.getApps, []);
			refreshApp();
		},
		[backend.appState, invalidate, refreshApp],
	);

	return (
		<SharedVisibilityStatusSwitcher
			localApp={localApp}
			canEdit={canEdit}
			onVisibilityChange={handleVisibilityChange}
		/>
	);
}
