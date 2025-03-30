"use client";
import type { UseQueryResult } from "@tanstack/react-query";
import { Bit, Button } from "@tm9657/flow-like-ui";
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
import { useEffect, useState } from "react";
import { useTauriInvoke } from "../../components/useInvoke";

export default function Onboarding() {
	const [profiles, setProfiles] = useState<[ISettingsProfile, IBit[]][]>([]);
	const [route, setRoute] = useState("");
	const [totalSize, setTotalSize] = useState(0);
	const defaultProfiles: UseQueryResult<[ISettingsProfile, IBit[]][]> =
		useTauriInvoke("get_default_profiles", {});
	const [activeProfiles, setActiveProfiles] = useState<string[]>([]);

	async function calculateSize() {
		const bits = new Map<string, Bit>();
		activeProfiles.forEach((profileId) => {
			const profile = profiles.find(
				([profile]) => profile.hub_profile.id === profileId,
			);
			if (!profile) return;
			profile[1].forEach((bit) => bits.set(bit.id, Bit.fromObject(bit)));
		});

		const sizes = await Promise.all(
			Array.from(bits.values()).map((bit) => bit.fetchSize()),
		);
		setTotalSize(sizes.reduce((acc, size) => acc + size, 0));
	}

	useEffect(() => {
		calculateSize().then((res) => console.log(res));
		if (activeProfiles.length === 0) return setRoute("");

		const params = activeProfiles.reduce((previous, current, index) => {
			if (index === 0) return `profiles=${current}`;
			if (previous && previous !== "") return `${previous}&profiles=${current}`;
			return `profiles=${current}`;
		}, "");

		setRoute(params.startsWith("profile=") ? params : `profiles=${params}`);
	}, [activeProfiles]);

	useEffect(() => {
		if (!defaultProfiles.data) return;
		const profiles: [ISettingsProfile, Bit[]][] = defaultProfiles.data as any;
		setProfiles(profiles);
	}, [defaultProfiles.data]);

	return (
		<div className="flex flex-col items-center justify-center">
			<h1>
				Welcome to <span className="highlight">Flow-Like</span>
			</h1>
			<h2>What are you interested in?</h2>
			<div className="flex flex-row items-center rounded-md my-8">
				<div className="flex flex-row items-center justify-center flex-wrap gap-4">
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
			<h4>
				Estimated Download Size:{" "}
				<b className="highlight">{humanFileSize(totalSize)}</b>
			</h4>
			<br />
			<a className="w-full" href={`/onboarding/download?${route}`}>
				<Button disabled={route.length === 0} className="gap-2 w-full">
					<ArrowBigRight /> Next
				</Button>
			</a>
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
			className="group cursor-pointer bg-card p-4 relative flex flex-col items-center justify-center h-64 w-44 rounded-lg overflow-hidden border"
		>
			<img
				className="absolute object-cover top-0 bottom-0 right-0 left-0 z-0 w-full h-full"
				src={profile.hub_profile.thumbnail}
				width={150}
				height={150}
				alt={profile.hub_profile.name}
			/>
			<div
				className={`absolute top-0 bottom-0 right-0 left-0 ${active ? "bg-background/40" : "bg-background/80"} group-hover:bg-background/40`}
			/>
			<h3 className="z-10 w-full text-foreground">
				{profile.hub_profile.name}
			</h3>
			<p className="z-10 line-clamp-3	text-foreground">
				{profile.hub_profile.description}
			</p>
			<div className="flex flex-row flex-wrap z-20 w-full items-center gap-2 justify-start h-14 mt-2">
				{bits.map((bit) => (
					<BitHover bit={bit} key={bit.id}>
						<Avatar key={bit.id} className="w-7 h-7">
							<AvatarImage
								className="rounded-full w-7 h-7"
								width={7}
								src={bit.icon}
							/>
							<AvatarImage
								className="rounded-full w-7 h-7"
								width={7}
								src="/app-logo.webp"
							/>
							<AvatarFallback>NA</AvatarFallback>
						</Avatar>
					</BitHover>
				))}
			</div>
		</button>
	);
}
