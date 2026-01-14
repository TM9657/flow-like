"use client";

import { useBackend, useInvoke, type IBit, AIModelPage } from "@tm9657/flow-like-ui";
import {useCallback } from "react";

export default function SettingsAiPage() {
	const backend = useBackend()
	const profile = useInvoke(backend.userState.getProfile, backend.userState, []);



	if (!profile.data) {
		return null;
	}

	return (
		<AIModelPage/>);
}
