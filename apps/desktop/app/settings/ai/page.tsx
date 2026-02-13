"use client";

import { AIModelPage, useBackend, useInvoke } from "@tm9657/flow-like-ui";

export default function SettingsAiPage() {
	const backend = useBackend();
	const profile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
	);

	if (!profile.data) {
		return null;
	}

	return <AIModelPage />;
}
