"use client";

import { Loader2 } from "lucide-react";
import { useState } from "react";
import { Button } from "../ui/button";
import type { HomeImageReference } from "./home-image-config";
import { useHomeStorageImage } from "./use-home-storage-image";

export function HomeInformationImage({
	image,
	alt,
}: { image: HomeImageReference; alt: string }) {
	return image.source === "storage" ? (
		<HomeStorageImage
			key={JSON.stringify([image.appId, image.path])}
			appId={image.appId}
			path={image.path}
			alt={alt}
		/>
	) : (
		<HomeImageFrame key={image.url} src={image.url} alt={alt} />
	);
}

function HomeStorageImage({
	appId,
	path,
	alt,
}: { appId: string; path: string; alt: string }) {
	const { src, isLoading, error, retry } = useHomeStorageImage(appId, path);
	if (error) return <HomeImageError storage onRetry={retry} />;
	if (isLoading || !src)
		return (
			<output className="mb-3 flex min-h-24 items-center justify-center gap-2 rounded-xl bg-muted/40 text-xs text-muted-foreground">
				<Loader2 className="size-4 animate-spin" aria-hidden="true" />
				Loading image…
			</output>
		);
	return <HomeImageFrame key={src} src={src} alt={alt} onRetry={retry} />;
}

function HomeImageError({
	storage = false,
	onRetry,
}: { storage?: boolean; onRetry?: () => void }) {
	return (
		<div className="mb-4 space-y-3 rounded-xl border border-dashed p-6 text-center text-xs text-muted-foreground">
			<output className="block">
				{storage
					? "This image could not load. Check your connection and access to the app."
					: "This image could not be loaded. Check its URL in widget settings."}
			</output>
			{onRetry && (
				<Button type="button" variant="outline" size="sm" onClick={onRetry}>
					Try again
				</Button>
			)}
		</div>
	);
}

function HomeImageFrame({
	src,
	alt,
	onRetry,
}: { src: string; alt: string; onRetry?: () => void }) {
	const [failedSrc, setFailedSrc] = useState<string | null>(null);
	if (failedSrc === src)
		return (
			<HomeImageError
				storage={Boolean(onRetry)}
				onRetry={
					onRetry
						? () => {
								setFailedSrc(null);
								onRetry();
							}
						: undefined
				}
			/>
		);
	return (
		<img
			src={src}
			alt={alt}
			loading="lazy"
			referrerPolicy="no-referrer"
			onError={() => setFailedSrc(src)}
			className="mb-3 max-h-64 w-full shrink-0 rounded-xl object-cover"
		/>
	);
}
