"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect } from "react";

/**
 * Configuration merged into Setup, where app-wide parameters are the second
 * segment. A static export has no server redirects, so this forwards on the
 * client — and it forwards the whole query string, because every config route
 * reads the app identity from `?id=` and losing it renders a blank page.
 */
export default function Page() {
	const router = useRouter();
	const searchParams = useSearchParams();

	useEffect(() => {
		const params = new URLSearchParams(searchParams.toString());
		params.set("mode", "shared");
		router.replace(`/library/config/setup?${params.toString()}`);
	}, [router, searchParams]);

	return null;
}
