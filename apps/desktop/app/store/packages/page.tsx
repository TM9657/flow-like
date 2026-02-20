"use client";

import { PackagesStorePage } from "@tm9657/flow-like-ui";
import { useCallback } from "react";
import { useAuth } from "react-oidc-context";
import { usePackageStatusMap } from "../../../hooks/use-package-status";
import { fetcher } from "../../../lib/api";

export default function Page() {
	const auth = useAuth();
	const statusMap = usePackageStatusMap();
	const getPackageStatus = useCallback(
		(packageId: string) => statusMap.get(packageId),
		[statusMap],
	);

	return (
		<PackagesStorePage
			fetcher={fetcher}
			auth={auth}
			getPackageStatus={getPackageStatus}
		/>
	);
}
