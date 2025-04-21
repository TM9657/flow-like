"use client";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { StorageSystem } from "@tm9657/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback } from "react";
import { toast } from "sonner";
import { useTauriInvoke } from "../../../../components/useInvoke";

interface IStorageItem {
	location: string;
	last_modified: string;
	size: number;
	e_tag?: string;
	version?: string;
}

export default function Page() {
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const prefix = searchParams.get("prefix") ?? "";
	const files = useTauriInvoke<IStorageItem[]>(
		"storage_list",
		{ appId: id, prefix },
		[id ?? "", prefix],
		typeof id === "string",
	);
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

	const uploadFile = useCallback(
		async (file: string, folder: boolean) => {
			await invoke<string>("storage_add", {
				appId: id,
				prefix: file,
				folder,
			});
			await files.refetch();
		},
		[id],
	);

	const deleteFile = useCallback(
		async (file: string) => {
			await invoke<string>("storage_remove", {
				appId: id,
				prefix: file,
			});
			await files.refetch();
		},
		[id],
	);

	const shareFile = useCallback(
		async (file: string) => {
			const url = await invoke<string>("storage_get", {
				appId: id,
				prefix: file,
			});
			writeText(url);
			toast.success("Copied to clipboard");
		},
		[id],
	);

	return (
		<main className="flex flex-col gap-2 h-full max-h-full w-full flex-grow overflow-hidden">
			<StorageSystem
				appId={id ?? ""}
				prefix={decodeURIComponent(prefix)}
				files={files.data ?? []}
				deleteFile={deleteFile}
				shareFile={shareFile}
				fileToUrl={fileToUrl}
				moveFile={async (file, newPrefix) => {}}
				uploadFile={uploadFile}
				updatePrefix={(prefix) => {
					router.push(
						`/store/config/storage?id=${id}&prefix=${encodeURIComponent(prefix)}`,
					);
				}}
				key={`${id}-${prefix}`}
			/>
		</main>
	);
}
