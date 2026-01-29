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

	const joinApp = useCallback(async () => {
		if (!appId || !token) {
			console.error("App ID or token is missing in the URL parameters.");
			return;
		}

		try {
			await backend.teamState.joinInviteLink(appId, token);
			toast.success("Successfully joined the app!");
			router.push(`/use?id=${appId}`);
		} catch (error) {
			router.push(`/use?id=${appId}`);
		}
	}, [backend, appId, token]);

	useEffect(() => {
		joinApp();
	}, [appId, token]);

	return <LoadingScreen />;
}
