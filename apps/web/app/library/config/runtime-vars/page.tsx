"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect } from "react";

/**
 * Runtime Variables merged into Setup, which opens on this lane. A static
 * export has no server redirects, so this forwards on the client, preserving
 * `?id=` and anything else the caller carried.
 */
export default function Page() {
	const router = useRouter();
	const searchParams = useSearchParams();

	useEffect(() => {
		const query = searchParams.toString();
		router.replace(`/library/config/setup${query ? `?${query}` : ""}`);
	}, [router, searchParams]);

	return null;
}
