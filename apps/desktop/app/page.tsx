"use client";
import {
	HomeSwimlanes,
	Skeleton,
	TutorialDialog,
	useBackend,
} from "@tm9657/flow-like-ui";
import type { ISettingsProfile } from "@tm9657/flow-like-ui/types";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { useTauriInvoke } from "../components/useInvoke";

export default function Home() {
	const backend = useBackend();
	const router = useRouter();
	const [isCheckingProfiles, setIsCheckingProfiles] = useState(true);
	const profiles = useTauriInvoke<Record<string, ISettingsProfile>>(
		"get_profiles",
		{},
	);

	useEffect(() => {
		if (profiles.isLoading) return;

		// Handle successful data load
		if (profiles.data) {
			const profileCount = Object.keys(profiles.data).length;

			if (profileCount === 0) {
				router.push("/onboarding");
			} else {
				setIsCheckingProfiles(false);
			}
			return;
		}

		// Only redirect on actual errors, not on undefined/null data during loading
		if (profiles.isError) {
			console.error("Failed to load profiles:", profiles.error);
			router.push("/onboarding");
		}
	}, [
		profiles.data,
		profiles.isLoading,
		profiles.isError,
		profiles.error,
		router,
	]);

	if (profiles.isLoading || isCheckingProfiles) {
		return (
			<main className="flex flex-col flex-1 w-full min-h-0 overflow-hidden">
				<TutorialDialog />
				<div className="flex-1 min-h-0 overflow-auto p-4 grid grid-cols-6 justify-start gap-2">
					<Skeleton className="col-span-6 h-full min-h-[30dvh]" />
					<Skeleton className="col-span-3 h-full min-h-[20dvh]" />
					<Skeleton className="col-span-3 h-full" />
					<Skeleton className="col-span-2 h-full" />
					<Skeleton className="col-span-2 h-full" />
					<Skeleton className="col-span-2 h-full" />
				</div>
			</main>
		);
	}

	return (
		<main className="flex flex-col flex-1 w-full min-h-0 overflow-hidden">
			<TutorialDialog />
			<HomeSwimlanes />
		</main>
	);
}
