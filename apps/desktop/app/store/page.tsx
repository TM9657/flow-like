"use client";

import { TextEditor } from "@tm9657/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { AboutSection } from "./components/AboutSection";
import { StoreHero } from "./components/StoreHero";
import { DetailsCard } from "./components/StoreInfo";
import { StoreRecommendations } from "./components/StoreRecommendations";
import { EmptyState, HeroSkeleton } from "./components/StoreSkeletons";
import { useStoreData } from "./components/useStoreData";

export default function Page() {
    const searchParams = useSearchParams();
    const router = useRouter();
    const id = searchParams.get("id") ?? undefined;
    const {
        apps,
        app,
        meta,
        isMember,
        coverUrl,
        iconUrl,
        appName,
        priceLabel,
        onUse,
        onSettings,
        onBuy,
        onJoinOrRequest,
    } = useStoreData(id, router);

    if (!id) {
        return (
            <div className="container mx-auto px-4 py-10">
                <EmptyState title="No app selected" description="Choose an app from the store to view its details." />
            </div>
        );
    }

    if (!app.data || !meta.data) {
        return (
            <div className="container mx-auto px-4 py-10 space-y-6">
                <HeroSkeleton />
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                    <div className="lg:col-span-2 rounded-xl border bg-card p-6" />
                    <div className="rounded-xl border bg-card p-6" />
                </div>
            </div>
        );
    }

    return (
        <div className="container mx-auto px-4 py-8 flex flex-col gap-8 flex-1 min-h-0">
            <StoreHero
                coverUrl={coverUrl}
                iconUrl={iconUrl}
                appName={appName}
                priceLabel={priceLabel}
                category={app.data.primary_category ?? "Other"}
                ageRating={(meta.data.age_rating ?? 0) + "+"}
                isMember={isMember}
                ratingCount={app.data.rating_count}
                avgRating={app.data.avg_rating ?? 0}
                visibility={app.data.visibility}
                authors={app.data.authors}
            />

            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 flex-1 min-h-fit">
                <AboutSection app={app.data} meta={meta.data} />
                <DetailsCard
                    app={app.data}
                    meta={meta.data}
                    isMember={isMember}
                    price={app.data.price ?? 0}
                    visibility={app.data.visibility}
                    priceLabel={priceLabel}
                    onUse={onUse}
                    onSettings={onSettings}
                    onJoinOrRequest={onJoinOrRequest}
                    onBuy={onBuy} />
            </div>

            {meta.data.long_description && <div className="leading-relaxed mx-2">
                <TextEditor
                    initialContent={meta.data.long_description ?? "No description available."}
                    isMarkdown
                />
            </div>}

            <StoreRecommendations />
        </div>
    );
}