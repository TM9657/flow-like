"use client";

import { useEffect, useState } from "react";
import type { IProfile } from "../../lib/schema/profile/profile";
import { Avatar } from "../ui/avatar";

export function ProfileAvatar({
	profile,
	className,
}: {
	profile: Pick<IProfile, "id" | "hub" | "icon"> | null | undefined;
	className?: string;
}) {
	return (
		<Avatar className={className}>
			<ProfileAvatarImage
				key={JSON.stringify([profile?.hub, profile?.id])}
				src={profile?.icon ?? undefined}
			/>
		</Avatar>
	);
}

function ProfileAvatarImage({ src }: { src?: string }) {
	const [loadedSrc, setLoadedSrc] = useState<string>();

	useEffect(() => {
		if (!src) {
			setLoadedSrc(undefined);
			return;
		}

		// Signed URLs change during refreshes. Keep the current icon until the
		// replacement has loaded, and ignore requests from an earlier profile.
		let active = true;
		const image = new Image();
		image.onload = () => {
			if (active) setLoadedSrc(src);
		};
		image.onerror = () => {
			if (active) setLoadedSrc(undefined);
		};
		image.src = src;
		if (image.complete && image.naturalWidth > 0) setLoadedSrc(src);

		return () => {
			active = false;
			image.onload = null;
			image.onerror = null;
		};
	}, [src]);

	return (
		<img
			alt=""
			className="aspect-square size-full object-cover"
			src={(src && loadedSrc) || "/app-logo.webp"}
			onError={() => setLoadedSrc(undefined)}
		/>
	);
}
