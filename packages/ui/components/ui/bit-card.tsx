"use client";
import type { UseQueryResult } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DownloadCloudIcon, PackageCheckIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { Progress } from "../../components/ui/progress";
import { useInvoke } from "../../hooks/use-invoke";
import { Bit, type IDownloadProgress } from "../../lib/bit/bit";
import type { IBit } from "../../lib/schema/bit/bit";
import { humanFileSize } from "../../lib/utils";
import type { ISettingsProfile } from "../../types";
import { Avatar, AvatarFallback, AvatarImage } from "./avatar";
import { Badge } from "./badge";
import { BentoGridItem } from "./bento-grid";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "./dropdown-menu";

export function BitCard({
	bit,
	wide = false,
}: Readonly<{ bit: IBit; wide: boolean }>) {
	const [progress, setProgress] = useState<{
		active: boolean;
		progress: IDownloadProgress;
	}>({
		active: false,
		progress: {
			downloaded: 0,
			hash: "",
			max: 0,
			path: "",
		},
	});
	const isInstalled: UseQueryResult<boolean> = useInvoke(
		"is_bit_installed",
		{ bit: bit },
		[bit.id],
	);
	const bitSize: UseQueryResult<number> = useInvoke(
		"get_bit_size",
		{ bit: bit },
		[bit.id],
	);
	const currentProfile: UseQueryResult<ISettingsProfile> = useInvoke(
		"get_current_profile",
		{},
	);

	async function toggleDownload() {
		if (isInstalled.data) {
			console.log("Deleting Bit");
			await invoke("delete_bit", { bit: bit });
			await isInstalled.refetch();
			return;
		}

		setProgress((prev) => {
			prev.active = true;
			return { ...prev };
		});

		const localBit = Bit.fromObject(bit);

		await localBit.download((event) => {
			setProgress((prev) => {
				const total = event.total();
				prev.progress = {
					downloaded: Math.max(total.downloaded, total.max - 1),
					hash: bit.hash,
					max: total.max,
					path: event.files().get(bit.hash)?.path ?? "",
				};
				return { ...prev };
			});
		});
		await isInstalled.refetch();
		setProgress((prev) => {
			prev.active = false;
			prev.progress = {
				...prev.progress,
				downloaded: 0,
				max: 0,
			};
			return { ...prev };
		});
	}

	useEffect(() => {
		if (!bit.hash || bit.hash === "") return;
		const download_model_subscription = listen(
			`download:${bit.hash}`,
			(event: { payload: IDownloadProgress[] }) => {
				if (progress.active) return;
				setProgress((prev) => {
					if (prev.active) return prev;
					prev.progress = event.payload.pop() ?? prev.progress;
					return { ...prev };
				});
			},
		);

		return () => {
			(async () => {
				(await download_model_subscription)();
			})();
		};
	}, []);

	return (
		<div
			className={`${wide ? "md:col-span-2 z-10" : "z-10"} focus:outline-none`}
			data-testid={"box"}
		>
			<DropdownMenu>
				<DropdownMenuTrigger className="relative w-full h-full text-start">
					<BentoGridItem
						className={`h-full w-full overflow-x-hidden ${isInstalled.data && currentProfile.data?.hub_profile.bits.find((b) => b[1] === bit.id) && "!border-[#84cc16]"}`}
						title={
							<div className="flex flex-row items-center text-nowrap text-start">
								<p className="max-w-[60%] overflow-hidden text-ellipsis">
									{bit.meta.en.name}
								</p>{" "}
								<Badge variant={"outline"} className="ml-2">
									{isInstalled ? (
										<PackageCheckIcon
											color={"#84cc16"}
											size={15}
											className="mr-1"
										/>
									) : (
										<DownloadCloudIcon
											color="#db2777"
											size={15}
											className="mr-1"
										/>
									)}{" "}
									{humanFileSize(bitSize.data ?? 0)}
								</Badge>
							</div>
						}
						description={
							<div className="overflow-scroll max-h-20 overflow-x-hidden overflow-y-auto">
								{bit.meta.en.description}
							</div>
						}
						header={
							<div>
								{progress.progress.max > 0 &&
									progress.progress.downloaded !== progress.progress.max && (
										<Progress
											className="mb-2"
											value={
												(100 * progress.progress.downloaded) /
												progress.progress.max
											}
										/>
									)}
								<div className="flex flex-row items-center justify-between">
									<div className="rounded-full w-14 h-14 flex items-center">
										<Avatar className="border">
											<AvatarImage className="p-2" src={bit.icon} />
											<AvatarImage className="" src="/app-logo.webp" />
											<AvatarFallback>NA</AvatarFallback>
										</Avatar>
									</div>
									{bit.repository?.startsWith("https://huggingface.co/") && (
										<img
											src={"/hf-logo.png"}
											width={25}
											height={25}
											alt="Huggingface Logo"
										/>
									)}
								</div>
							</div>
						}
						icon={
							<div className="text-left text-nowrap text-ellipsis max-w-full overflow-hidden">
								{bit.meta.en.tags.map((category) => (
									<Badge className="ml-2" variant="outline" key={category}>
										{category}
									</Badge>
								))}
							</div>
						}
					/>
					{isInstalled.data && (
						<div className="absolute bottom-0 right-0 bg-[#84cc16] w-8 h-8 p-2 flex flex-row items-center justify-center rounded-br-xl rounded-tl-xl">
							<PackageCheckIcon />
						</div>
					)}
				</DropdownMenuTrigger>
				<DropdownMenuContent>
					{(bit.type === "Embedding" ||
						bit.type === "ImageEmbedding" ||
						bit.type === "Vlm" ||
						bit.type === "Llm") && (
						<DropdownMenuItem
							onClick={async () => {
								const profile = currentProfile.data;
								if (!profile) return;
								const bitIndex = profile.hub_profile.bits.findIndex(
									([hub, id]) => id === bit.id,
								);
								if (bitIndex === -1) {
									await invoke("download_bit", { bit });
									await invoke("add_bit", { profile, bit });
									await currentProfile.refetch();
									return;
								}

								await invoke("remove_bit", { profile, bit });
								await currentProfile.refetch();
							}}
						>
							{(currentProfile.data?.hub_profile.bits || []).findIndex(
								([hub, b]) => bit.id === b,
							) > -1
								? "Remove from Profile"
								: "Add to Profile"}
						</DropdownMenuItem>
					)}
					<DropdownMenuItem
						onClick={async () => {
							toggleDownload();
						}}
					>
						{isInstalled.data ? "Delete" : "Download"}
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					{bit.repository && (
						<DropdownMenuItem
							onClick={() => {
								if (bit.repository) open(bit.repository);
							}}
						>
							Open Repository
						</DropdownMenuItem>
					)}
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}
