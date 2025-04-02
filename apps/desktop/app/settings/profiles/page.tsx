"use client";

import { invoke } from "@tauri-apps/api/core";
import {
	Input,
	type UseQueryResult,
	useBackend,
	useInvalidateInvoke,
} from "@tm9657/flow-like-ui";
import { Label } from "@tm9657/flow-like-ui/components/ui/label";
import { Switch } from "@tm9657/flow-like-ui/components/ui/switch";
import type { ISettingsProfile, ISystemInfo } from "@tm9657/flow-like-ui/types";
import { useTauriInvoke } from "../../../components/useInvoke";

export default function SettingsPage() {
	const backend = useBackend();
	const invalidate = useInvalidateInvoke();
	const profiles: UseQueryResult<ISettingsProfile> = useTauriInvoke(
		"get_profiles",
		{},
	);
	const systemInfo: UseQueryResult<ISystemInfo> = useTauriInvoke(
		"get_system_info",
		{},
	);
	const currentProfile: UseQueryResult<ISettingsProfile | undefined> =
		useTauriInvoke("get_current_profile", {});

	async function upsertProfile(profile: ISettingsProfile) {
		await invoke("upsert_profile", { profile });
		await profiles.refetch();
		await currentProfile.refetch();
	}

	return (
		<main className="justify-start flex min-h-dvh flex-col items-center w-full pr-4">
			<div className="flex flex-row items-center justify-between w-full max-w-screen-2xl">
				<h1 className="scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl">
					Profiles
				</h1>
			</div>
			<br />
			<div className="border p-4 w-full bg-card text-card-foreground">
				{currentProfile.data && (
					<div className="flex flex-col items-center justify-between w-full">
						<div className="flex flex-row items-start w-full">
							<img
								className="rounded-md"
								width={256}
								height={256}
								src={currentProfile.data.hub_profile.thumbnail}
								alt="thumbnail"
							/>
							<div className="w-full px-4 gap-4 flex flex-col">
								<div>
									<Label htmlFor="name">Name</Label>
									<Input
										id="name"
										placeholder="Name"
										value={currentProfile.data.hub_profile.name}
										onChange={async (e) => {
											if (currentProfile.data)
												await upsertProfile({
													...currentProfile.data,
													hub_profile: {
														...currentProfile.data.hub_profile,
														name: e.target.value,
													},
												});
										}}
									/>
								</div>
								<div>
									<Label htmlFor="description">Description</Label>
									<Input
										id="description"
										placeholder="Description"
										value={currentProfile.data.hub_profile.description}
										onChange={async (e) => {
											if (currentProfile.data)
												await upsertProfile({
													...currentProfile.data,
													hub_profile: {
														...currentProfile.data.hub_profile,
														description: e.target.value,
													},
												});
										}}
									/>
								</div>
								<div>
									<Label htmlFor="context_size">Max Context Size</Label>
									<Input
										id="context_size"
										placeholder="Max Context"
										value={
											currentProfile.data.execution_settings?.max_context_size
										}
										type="number"
										onChange={async (e) => {
											if (currentProfile.data)
												await upsertProfile({
													...currentProfile.data,
													execution_settings: {
														...currentProfile.data.execution_settings,
														max_context_size: Number.parseInt(e.target.value),
													},
												});
										}}
									/>
								</div>
								<div className="flex items-center space-x-2">
									<Switch
										checked={
											typeof currentProfile.data.execution_settings
												?.gpu_mode === "boolean"
												? currentProfile.data.execution_settings?.gpu_mode
												: true
										}
										id="gpu"
										onCheckedChange={async (checkedNew) => {
											if (currentProfile.data)
												await upsertProfile({
													...currentProfile.data,
													execution_settings: {
														...currentProfile.data.execution_settings,
														gpu_mode:
															(systemInfo.data?.vram || 0) > 0
																? checkedNew
																: false,
													},
												});
										}}
									/>
									<Label htmlFor="gpu">GPU</Label>
								</div>
							</div>
						</div>
						<br />
					</div>
				)}
			</div>
			<br />
			<h2 className="w-full">Other Profiles</h2>
			<br />
			<div className="w-full flex flex-row gap-4 flex-wrap">
				{profiles?.data &&
					Object.values(profiles.data)
						.filter(
							(profile) =>
								profile.hub_profile.name !==
								currentProfile.data?.hub_profile.name,
						)
						.map((profile) => (
							<button
								key={profile.hub_profile.id}
								className="w-[256px] h-[256px] relative items-center rounded-md overflow-hidden flex flex-col justify-center p-4 cursor-pointer"
								onClick={async () => {
									if (profile.hub_profile.id !== "")
										await invoke("set_current_profile", {
											profileId: profile.hub_profile.id,
										});
									await invalidate(backend.getProfile, []);
									await invalidate(backend.getSettingsProfile, []);
								}}
							>
								<img
									className="absolute top-0 left-0 right-0 bottom-0 z-0"
									width={256}
									height={256}
									src={profile.hub_profile.thumbnail}
									alt="thumbnail"
								/>
								<div className="absolute top-0 left-0 right-0 bottom-0 bg-background/50 z-0" />
								<div className="relative">
									<h4 className="z-40 relative">{profile.hub_profile.name}</h4>
									<p className="line-clamp-3">
										{profile.hub_profile.description}
									</p>
								</div>
							</button>
						))}
			</div>
			<br />
			<br />
		</main>
	);
}
