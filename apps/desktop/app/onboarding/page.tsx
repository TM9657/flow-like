"use client";
import type { IHub, UseQueryResult } from "@tm9657/flow-like-ui";
import { Bit, Button, useBackend } from "@tm9657/flow-like-ui";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
} from "@tm9657/flow-like-ui/components/ui/avatar";
import { BitHover } from "@tm9657/flow-like-ui/components/ui/bit-hover";
import type { IBit } from "@tm9657/flow-like-ui/lib/schema/bit/bit";
import { humanFileSize } from "@tm9657/flow-like-ui/lib/utils";
import type { ISettingsProfile } from "@tm9657/flow-like-ui/types";
import { ArrowBigRight } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTauriInvoke } from "../../components/useInvoke";

export default function Onboarding() {
	const backend = useBackend();
	const [profiles, setProfiles] = useState<[ISettingsProfile, IBit[]][]>([]);
	const [route, setRoute] = useState("");
	const [totalSize, setTotalSize] = useState(0);
	const defaultProfiles: UseQueryResult<[[ISettingsProfile, IBit[]][], IHub]> =
		useTauriInvoke("get_default_profiles", {});
	const [activeProfiles, setActiveProfiles] = useState<string[]>([]);

	const calculateSize = useCallback(async () => {
		const bits = new Map<string, Bit>();
		activeProfiles.forEach((profileId) => {
			const profile = profiles.find(
				([profile]) => profile.hub_profile.id === profileId,
			);
			if (!profile) return;
			profile[1].forEach((bit) => {
				const bitInstance = Bit.fromObject(bit);
				bitInstance.setBackend(backend);
				bits.set(bit.id, bitInstance);
			});
		});

		const sizes = await Promise.all(
			Array.from(bits.values()).map((bit) => bit.fetchSize()),
		);
		setTotalSize(sizes.reduce((acc, size) => acc + size, 0));
	}, [activeProfiles, profiles, backend]);

	useEffect(() => {
		if (!backend) return;
		calculateSize().then((res) => console.log(res));
		if (activeProfiles.length === 0) return setRoute("");

		const params = activeProfiles.reduce((previous, current, index) => {
			if (index === 0) return `profiles=${current}`;
			if (previous && previous !== "") return `${previous}&profiles=${current}`;
			return `profiles=${current}`;
		}, "");

		setRoute(params.startsWith("profile=") ? params : `profiles=${params}`);
	}, [activeProfiles, backend]);

	useEffect(() => {
		if (!defaultProfiles.data) return;
		const profiles: [[ISettingsProfile, IBit[]][], IHub] =
			defaultProfiles.data as any;
		setProfiles(profiles[0]);
	}, [defaultProfiles.data]);

	return (
		<div className="flex flex-col items-center justify-center h-full">
			<div className="text-center mb-12 space-y-4 max-w-2xl">
				<div className="space-y-2">
					<h1 className="text-5xl font-bold text-foreground tracking-tight">
						Welcome to <span className="highlight">Flow-Like</span>
					</h1>
					<div className="w-24 h-1 bg-gradient-to-r from-primary to-primary/70 mx-auto rounded-full" />
				</div>
				<h2 className="text-2xl text-muted-foreground font-medium mt-6">
					Select your starting profile
				</h2>
				<p className="text-base text-muted-foreground/80 max-w-lg mx-auto leading-relaxed">
					Choose one or more profiles that match your interests. You can always
					add, change or remove profiles later.
				</p>
			</div>

			<div className="w-full max-w-6xl px-2">
				<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 justify-items-center">
					{profiles.map(([profile, bits], index) => (
						<PreviewCard
							key={profile.hub_profile.name}
							bits={bits}
							profile={profile}
							active={activeProfiles.includes(profile.hub_profile.id ?? "")}
							onClick={() => {
								setActiveProfiles((old) => {
									if (old.includes(profile.hub_profile.id ?? ""))
										return old.filter((id) => id !== profile.hub_profile.id);
									return [...old, profile.hub_profile.id ?? ""];
								});
							}}
						/>
					))}
				</div>
			</div>

			<div className="flex flex-row items-center gap-4 w-full max-w-md mt-12">
				<div className="bg-card/70 backdrop-blur-sm border rounded-lg p-3 px-4 text-center shadow-lg flex flex-col items-center text-nowrap">
					<p className="font-semibold text-foreground text-lg">
						{humanFileSize(totalSize)}
					</p>
				</div>
				<a className="flex-1" href={`/onboarding/download?${route}`}>
					<Button
						disabled={route.length === 0}
						className="gap-2 w-full h-12 text-base font-medium"
					>
						<ArrowBigRight className="w-5 h-5" /> Download
					</Button>
				</a>
			</div>
		</div>
	);
}

function PreviewCard({
	profile,
	bits,
	onClick,
	active,
}: Readonly<{
	bits: IBit[];
	profile: ISettingsProfile;
	onClick?: () => void;
	active?: boolean;
}>) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={`group relative flex flex-col w-64 transition-all duration-500 rounded-2xl z-50 border-2 hover:shadow-2xl transform hover:-translate-y-1 ${
				active
					? "border-primary bg-primary/5 shadow-lg shadow-primary/20 scale-105"
					: "border-border bg-card/50 backdrop-blur-sm hover:border-primary/50 hover:bg-card/80"
			}`}
		>
			{/* Selection Indicator */}
			{active && (
				<div className="absolute top-4 right-4 z-10">
					<div className="w-6 h-6 bg-primary rounded-full flex items-center justify-center shadow-lg animate-in zoom-in-50 duration-300">
						<div className="w-2 h-2 bg-primary-foreground rounded-full" />
					</div>
				</div>
			)}

			{/* Thumbnail Section - 16:9 aspect ratio */}
			<div className="relative w-full aspect-video overflow-hidden rounded-t-xl">
				<img
					className="absolute rounded-t-xl inset-0 w-full h-full object-cover transition-transform duration-700 group-hover:scale-110"
					src={profile.hub_profile.thumbnail ?? "/placeholder-thumbnail.webp"}
					width={1280}
					height={640}
					alt={profile.hub_profile.name}
					loading="lazy"
					decoding="async"
					fetchPriority="low"
				/>
				<div
					className={`absolute inset-0 transition-all duration-300 ${
						active ? "bg-primary/20" : "bg-black/20 group-hover:bg-primary/10"
					}`}
				/>

				{/* Gradient Overlay */}
				<div className="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent" />
			</div>

			{/* Content Section */}
			<div className="flex flex-col p-4 space-y-3 flex-1 max-w-full overflow-hidden">
				<h3 className="font-semibold text-foreground text-left leading-tight truncate line-clamp-1 max-w-full overflow-hidden">
					{profile.hub_profile.name}
				</h3>

				<p className="text-sm text-muted-foreground text-left line-clamp-2 leading-relaxed max-w-full overflow-hidden">
					{profile.hub_profile.description}
				</p>

				{/* Bits Preview */}
				<div className="flex flex-row flex-wrap gap-2 pt-2 z-50">
					{bits.slice(0, 6).map((bit) => (
						<BitHover bit={bit} key={bit.id}>
							<Avatar className="border bg-background w-6 h-6 transition-transform duration-200 hover:scale-110 z-50">
								<AvatarImage
									className="p-0.5"
									src={bit.meta?.en?.icon ?? "/app-logo.webp"}
								/>
								<AvatarFallback className="text-xs">NA</AvatarFallback>
							</Avatar>
						</BitHover>
					))}
					{bits.length > 6 && (
						<div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center">
							<span className="text-xs text-muted-foreground">
								+{bits.length - 6}
							</span>
						</div>
					)}
				</div>
			</div>
		</button>
	);
}
