"use client";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { StorageSystem } from "@tm9657/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback } from "react";

export default function Page() {
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const prefix = searchParams.get("prefix") ?? "";
	const router = useRouter();

	const fileToUrl = useCallback(
		async (file: string) => {
			const url = await invoke<string>("storage_to_fullpath", {
				appId: id,
				prefix: file.split("/").slice(3).join("/"),
			});
			return convertFileSrc(url);
		},
		[id],
	);

	return (
		<StorageSystem
			appId={id ?? ""}
			prefix={decodeURIComponent(prefix)}
			fileToUrl={fileToUrl}
			updatePrefix={(prefix) => {
				router.push(
					`/library/config/storage?id=${id}&prefix=${encodeURIComponent(prefix)}`,
				);
			}}
			key={`${id}-${prefix}`}
		/>
	);
}
