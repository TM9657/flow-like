"use client";
import type { UseQueryResult } from "@tanstack/react-query";
import { useInvoke } from "@tm9657/flow-like-ui";
import { humanFileSize } from "@tm9657/flow-like-ui/lib/utils";
import type { ISystemInfo } from "@tm9657/flow-like-ui/types";

export default function SettingsPage() {
	const systemInfo: UseQueryResult<ISystemInfo> = useInvoke(
		"get_system_info",
		{},
	);

	return (
		<main className="justify-start flex min-h-dvh flex-col items-center w-full pr-4">
			<div className="flex flex-row items-center justify-between w-full max-w-screen-2xl">
				<h1 className="scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl">
					System Info
				</h1>
			</div>
			<br />
			<p className="w-full">
				<b>Cores</b>: {systemInfo.data?.cores}
			</p>
			<p className="w-full">
				<b>VRAM</b>: {humanFileSize(systemInfo.data?.vram ?? 0)}
			</p>
			<p className="w-full">
				<b>RAM</b>: {humanFileSize(systemInfo.data?.ram ?? 0)}
			</p>
		</main>
	);
}
