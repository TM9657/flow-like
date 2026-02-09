"use client";

import { NotFoundPage } from "@tm9657/flow-like-ui";
import { useRouter } from "next/navigation";

export default function NotFound() {
	const router = useRouter();

	return (
		<NotFoundPage
			onGoBack={() => router.back()}
			onGoHome={() => router.push("/")}
		/>
	);
}
