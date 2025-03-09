// app/providers.tsx
"use client";
import posthog from "posthog-js";
import { PostHogProvider } from "posthog-js/react";
import { useEffect } from "react";

export function PHProvider({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	useEffect(() => {
		posthog.init("phc_rw0DgMVHMe2psATzz1nN6PJjjddkBj4Dc9FQFVGW0dk", {
			api_host: "https://eu.i.posthog.com",
			person_profiles: "always",
			capture_pageleave: true,
			autocapture: true,
			enable_heatmaps: true,
		});
	}, []);

	return <PostHogProvider client={posthog}>{children}</PostHogProvider>;
}
