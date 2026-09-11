"use client";

import { useTranslation } from "@flow-like/locales";
import { memo } from "react";
import { useInvoke } from "../../../hooks/use-invoke";
import { userDisplayName } from "../../../lib/user-display";
import {
	useBackend,
	useBackendReady,
	useSignedIn,
} from "../../../state/backend-state";
import { AccountMenu } from "../../account/account-menu";
import { useAccountMenu } from "../../account/account-menu-context";

export const BoardAccountItem = memo(function BoardAccountItem() {
	const HostAccountMenu = useAccountMenu();
	return HostAccountMenu ? (
		<HostAccountMenu compact />
	) : (
		<StandaloneAccountMenu />
	);
});

// Embedded hosts can render a board without the application sidebar provider.
function StandaloneAccountMenu() {
	const { t } = useTranslation("flow");
	const backend = useBackend();
	const signedIn = useSignedIn();
	const backendReady = useBackendReady();
	const info = useInvoke(
		backend.userState.getInfo,
		backend.userState,
		[],
		backendReady && signedIn,
		[signedIn],
	);
	const notifications = useInvoke(
		backend.userState.getNotifications,
		backend.userState,
		[],
		backendReady,
		[signedIn],
	);
	return (
		<AccountMenu
			compact
			displayName={userDisplayName(
				signedIn ? info.data : undefined,
				t("offline", "Offline"),
			)}
			email={signedIn ? (info.data?.email ?? "") : t("signedOut", "Signed out")}
			avatar={signedIn ? info.data?.avatar : undefined}
			signedIn={signedIn}
			notificationCount={
				(notifications.data?.unread_count ?? 0) +
				(notifications.data?.invites_count ?? 0)
			}
		/>
	);
}
