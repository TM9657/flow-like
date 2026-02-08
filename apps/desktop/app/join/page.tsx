"use client";

import { LoadingScreen, useBackend } from "@tm9657/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";

export default function JoinPage() {
	const backend = useBackend();
	const router = useRouter();
	const searchParams = useSearchParams();
	const appId = searchParams.get("appId");
	const token = searchParams.get("token");

	const addToProfile = useCallback(
		async (appId: string) => {
			try {
				const profile = await backend.userState.getSettingsProfile();
				await backend.userState.updateProfileApp(
					profile,
					{ app_id: appId, favorite: false, pinned: false },
					"Upsert",
				);
			} catch (error) {
				console.error("Failed to add app to profile:", error);
			}
		},
		[backend],
	);

	const joinApp = useCallback(async () => {
		if (!appId || !token) {
			console.error("App ID or token is missing in the URL parameters.");
			return;
		}

		try {
			await backend.teamState.joinInviteLink(appId, token);
			await addToProfile(appId);
			toast.success("Successfully joined the app!");
			router.push(`/use?id=${appId}`);
		} catch (error) {
			router.push(`/use?id=${appId}`);
		}
	}, [backend, appId, token, addToProfile]);

	useEffect(() => {
		joinApp();
	}, [appId, token]);

	return <LoadingScreen />;
}
