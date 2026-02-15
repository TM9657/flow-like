"use client";

import {
	AboutSection,
	HeroSkeleton,
	StoreEmptyState,
	StoreHero,
	StoreRecommendations,
	TextEditor,
	useStoreData,
} from "@tm9657/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect } from "react";
import { toast } from "sonner";
import { EVENT_CONFIG } from "../../lib/event-config";

export default function Page() {
	const searchParams = useSearchParams();
	const router = useRouter();
	const id = searchParams.get("id") ?? undefined;
	const purchaseStatus = searchParams.get("purchase");
	const {
		apps,
		app,
		meta,
		isMember,
		isPurchasing,
		hasThumbnail,
		coverUrl,
		iconUrl,
		appName,
		priceLabel,
		canUseApp,
		onUse,
		onSettings,
		onBuy,
		onJoinOrRequest,
	} = useStoreData(id, router, EVENT_CONFIG);

	useEffect(() => {
		if (!purchaseStatus) return;

		if (purchaseStatus === "success") {
			toast.success("Purchase successful! You now have access to this app.", {
				duration: 5000,
			});
			apps.refetch?.();
		} else if (purchaseStatus === "canceled") {
			toast.info("Purchase was canceled. You can try again anytime.");
		}

		const url = new URL(window.location.href);
		url.searchParams.delete("purchase");
		router.replace(url.pathname + url.search, { scroll: false });
	}, [purchaseStatus, apps, router]);

	if (!id) {
		return (
			<div className="flex-1 flex items-center justify-center p-6">
				<StoreEmptyState
					title="No app selected"
					description="Choose an app from the store to view its details."
				/>
			</div>
		);
	}

	if (!app.data || !meta.data) {
		return (
			<main className="flex-col flex grow max-h-full overflow-auto min-h-0 w-full">
				<HeroSkeleton />
				<div className="max-w-5xl mx-auto px-6 md:px-10 pt-8 space-y-4 w-full">
					<div className="h-4 w-3/4 rounded-full bg-muted/20" />
					<div className="h-4 w-1/2 rounded-full bg-muted/20" />
				</div>
			</main>
		);
	}

	return (
		<main className="flex-col flex grow max-h-full overflow-auto min-h-0 w-full">
			<StoreHero
				appId={id}
				hasThumbnail={hasThumbnail}
				coverUrl={coverUrl}
				iconUrl={iconUrl}
				appName={appName}
				priceLabel={priceLabel}
				category={app.data.primary_category ?? "Other"}
				isMember={isMember}
				ratingCount={app.data.rating_count}
				avgRating={app.data.avg_rating ?? 0}
				visibility={app.data.visibility}
				authors={app.data.authors}
				canUseApp={canUseApp}
				price={app.data.price ?? 0}
				isPurchasing={isPurchasing}
				onUse={onUse}
				onSettings={onSettings}
				onBuy={onBuy}
				onJoinOrRequest={onJoinOrRequest}
			/>

			<div className="max-w-5xl mx-auto w-full px-6 md:px-10 pt-8 pb-12 space-y-10">
				<AboutSection app={app.data} meta={meta.data} />

				{meta.data.long_description && (
					<div className="leading-relaxed">
						<TextEditor
							initialContent={meta.data.long_description}
							isMarkdown
						/>
					</div>
				)}

				<StoreRecommendations />
			</div>
		</main>
	);
}
