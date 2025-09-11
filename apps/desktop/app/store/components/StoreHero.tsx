"use client"
import { Avatar, AvatarFallback, AvatarImage, Badge, IAppVisibility } from "@tm9657/flow-like-ui";
import { Heart, Star } from "lucide-react";
import { visibilityIcon, visibilityLabel } from "./Visibility";

export function StoreHero({
    coverUrl,
    iconUrl,
    appName,
    priceLabel,
    category,
    ageRating,
    isMember,
    ratingCount,
    avgRating,
    visibility,
    authors,
}: Readonly<{
    coverUrl: string;
    iconUrl: string;
    appName: string;
    priceLabel: string;
    category: string;
    ageRating: string | number;
    isMember: boolean;
    ratingCount: number;
    avgRating: number;
    visibility: IAppVisibility;
    authors: string[];
}>) {
    return (
        <div className="relative overflow-hidden rounded-2xl border bg-card">
            <div className="absolute inset-0 bg-cover bg-center" style={{ backgroundImage: `url(${coverUrl})` }} />
            <div className="absolute inset-0 bg-gradient-to-r from-background/80 via-background/60 to-background/30" />

            <div className="relative p-6 md:p-8 flex flex-col md:flex-row items-start md:items-center gap-6">
                <Avatar className="h-20 w-20 md:h-24 md:w-24 shadow-lg border">
                    <AvatarImage src={iconUrl} alt={appName} />
                    <AvatarFallback className="font-bold">{appName.slice(0, 2).toUpperCase()}</AvatarFallback>
                </Avatar>

                <div className="flex-1 min-w-0">
                    <div className="flex flex-wrap items-center gap-3">
                        <h1 className="text-2xl md:text-3xl font-bold tracking-tight truncate">{appName}</h1>
                        <Badge>{category}</Badge>
                        <Badge variant="outline">{ageRating}</Badge>
                        {isMember ? (
                            <Badge variant="secondary" className="flex items-center gap-1"><Heart className="h-3 w-3" /> Yours</Badge>
                        ) : (
                            <Badge variant="secondary">{priceLabel}</Badge>
                        )}
                    </div>

                    <div className="mt-2 flex flex-wrap items-center gap-4 text-muted-foreground">
                        <div className="flex items-center gap-1">
                            <Star className="h-4 w-4 text-yellow-500 fill-yellow-500" />
                            <span className="text-sm font-medium">
                                {ratingCount > 0 ? `${avgRating.toFixed(1)} (${ratingCount.toLocaleString()})` : "No ratings yet"}
                            </span>
                        </div>
                        <div className="flex items-center gap-1">
                            {visibilityIcon(visibility)}
                            <span className="text-sm capitalize">{visibilityLabel(visibility)}</span>
                        </div>
                        {authors?.length ? (
                            <div className="text-sm truncate">By {authors.join(", ")}</div>
                        ) : null}
                    </div>
                </div>
            </div>
        </div>
    );
}
