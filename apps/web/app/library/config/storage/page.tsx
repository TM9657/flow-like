"use client";
import {
	RolePermissions,
	StorageSystem,
	useAppPermissions,
	useBackend,
} from "@flow-like/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback } from "react";

export default function Page() {
	const backend = useBackend();
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const permissions = useAppPermissions(id);
	const canWrite =
		!permissions.isLoading && permissions.can(RolePermissions.WriteFiles);
	const prefix = searchParams.get("prefix") ?? "";
	const router = useRouter();

	const fileToUrl = useCallback(
		async (file: string) => {
			// Listed locations are app-relative, which is what the download endpoint
			// expects — it also tolerates raw object-store keys from older listings.
			const results = await backend.storageState.downloadStorageItems(
				id ?? "",
				[file],
			);
			if (results.length > 0 && results[0].url) {
				return results[0].url;
			}
			return "";
		},
		[id, backend.storageState],
	);

	return (
		<StorageSystem
			appId={id ?? ""}
			prefix={decodeURIComponent(prefix)}
			fileToUrl={fileToUrl}
			enableCraftEditors
			canWrite={canWrite}
			updatePrefix={(prefix) => {
				router.push(
					`/library/config/storage?id=${id}&prefix=${encodeURIComponent(prefix)}`,
				);
			}}
			key={`${id}-${prefix}`}
		/>
	);
}
