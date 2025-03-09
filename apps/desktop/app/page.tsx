"use client";
import {
	type ISettingsProfile,
	Skeleton,
	useInvoke
} from "@tm9657/flow-like-ui";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { TutorialDialog } from "../components/tutorial-dialog";

export default function Home() {
	const profile = useInvoke<ISettingsProfile>("get_current_profile", {});
	const router = useRouter();

	function checkOnboarding() {
		const hasOnboarded = localStorage.getItem("onboarding-done");
		if (!hasOnboarded) {
			router.push("/onboarding");
			return true;
		}

		return false;
	}

	useEffect(() => {
		if (checkOnboarding()) return;
	}, []);

	useEffect(() => {
		if (checkOnboarding()) return;
		if (!profile.data) return;
	}, [profile.data]);

	return (
		<main className="min-h-screen items-center w-full max-h-dvh overflow-auto p-4 grid grid-cols-6 justify-start gap-2">
			<TutorialDialog />
			<Skeleton className="col-span-6 h-full min-h-[30dvh]" />
			<Skeleton className="col-span-3 h-full min-h-[20dvh]" />
			<Skeleton className="col-span-3 h-full" />
			<Skeleton className="col-span-2 h-full" />
			<Skeleton className="col-span-2 h-full" />
			<Skeleton className="col-span-2 h-full" />
		</main>
	);
}
