"use client";

import { useCallback, useEffect, useState } from "react";
import type { IHub } from "../lib";
import { useBackend } from "../state/backend-state";
import { useInvoke } from "./use-invoke";

export function useHub() {
	const backend = useBackend();
	const profile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
	);
	const [hub, setHub] = useState<IHub | undefined>();

	const fetchHub = useCallback(async () => {
		if (!profile.data?.hub) return;
		const protocol = (profile.data?.secure ?? true) ? "https" : "http";
		const hubData = await fetch(
			`${protocol}://${profile.data?.hub}/api/v1`,
			{},
		);
		const hubJson: IHub = await hubData.json();
		setHub(hubJson);
	}, [profile.data?.hub]);

	useEffect(() => {
		fetchHub();
	}, [profile.data?.hub]);

	return { hub, refetch: fetchHub };
}
