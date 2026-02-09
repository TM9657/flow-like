"use client";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Badge,
	type IAppVisibility,
	ShareButton,
} from "@tm9657/flow-like-ui";
import { Heart, Star } from "lucide-react";
import { visibilityIcon, visibilityLabel } from "./Visibility";

export function StoreHero({
	appId,
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
	appId: string;
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
		<div className="relative overflow-hidden rounded-2xl border bg-card group min-h-[240px] md:min-h-[260px]">
			{/* Background image with subtle Ken Burns animation */}
			<div className="absolute inset-0 z-0 pointer-events-none">
				<img
					src={coverUrl}
					alt={`${appName} cover`}
					loading="lazy"
					decoding="async"
					className="h-full w-full object-cover md:scale-105 md:animate-kenburns"
					onError={(e) => {
						(e.currentTarget as HTMLImageElement).style.display = "none";
					}}
				/>
				{/* Global dim to guarantee contrast over arbitrary uploads */}
				<div className="absolute inset-0 bg-background/30 md:bg-background/20" />
				{/* Bottom scrim stronger on mobile for text legibility */}
				<div className="absolute inset-x-0 bottom-0 h-full bg-gradient-to-t from-background/90 via-background/40 to-background/20 md:from-background/70 md:via-background/50" />
				{/* Soft ambience */}
				<div
					aria-hidden
					className="absolute -top-10 -left-10 h-48 w-48 bg-primary/25 blur-3xl rounded-full animate-float-slow"
				/>
				<div
					aria-hidden
					className="absolute -bottom-10 -right-10 h-48 w-48 bg-secondary/25 blur-3xl rounded-full animate-float-slow"
				/>
			</div>

			{/* Foreground content */}
			<div className="relative z-10 w-full h-full p-3 sm:p-4 md:p-5 lg:p-6">
				{/* Glass panel grows stronger on small screens to guarantee contrast over busy images */}
				<div className="absolute inset-0 md:inset-auto md:relative rounded-md bg-background/60 md:bg-background/25 supports-[backdrop-filter]:bg-background/40 md:supports-[backdrop-filter]:bg-background/20 backdrop-blur-md md:backdrop-blur-sm shadow-sm transition-colors sheen" />

				<div className="relative grid grid-cols-1 md:grid-cols-[auto,1fr] items-start md:items-center gap-3 sm:gap-4 md:gap-5 lg:gap-6">
					<Avatar className="h-16 w-16 md:h-20 md:w-20 lg:h-24 lg:w-24 shadow-lg ring-1 ring-background/40">
						<AvatarImage src={iconUrl} alt={appName} />
						<AvatarFallback className="font-bold">
							{appName.slice(0, 2).toUpperCase()}
						</AvatarFallback>
					</Avatar>

					<div className="min-w-0 flex flex-col">
						<div className="flex flex-wrap items-center gap-2 sm:gap-3">
							<h1 className="text-2xl md:text-3xl lg:text-4xl font-bold tracking-tight truncate max-w-full">
								{appName}
							</h1>
							<Badge>{category}</Badge>
							<Badge variant="secondary">{ageRating}</Badge>
							{isMember ? (
								<Badge variant="secondary" className="flex items-center gap-1">
									<Heart className="h-3 w-3" /> Yours
								</Badge>
							) : (
								<Badge variant="secondary">{priceLabel}</Badge>
							)}
						</div>

						<div className="mt-2 flex flex-wrap items-center gap-2 sm:gap-3 text-muted-foreground">
							<div className="flex items-center gap-1 rounded-full border bg-background/60 px-2 py-1 text-foreground">
								<Star className="h-4 w-4 text-yellow-500 fill-yellow-500" />
								<span className="text-xs md:text-sm font-medium">
									{ratingCount > 0
										? `${avgRating.toFixed(1)} (${ratingCount.toLocaleString()})`
										: "No ratings yet"}
								</span>
							</div>
							<div className="flex items-center gap-1 rounded-full border bg-background/60 px-2 py-1 text-foreground">
								{visibilityIcon(visibility)}
								<span className="text-xs md:text-sm capitalize">
									{visibilityLabel(visibility)}
								</span>
							</div>
							{authors?.length ? (
								<div className="text-xs md:text-sm truncate">
									By {authors.join(", ")}
								</div>
							) : null}
							<ShareButton
								appId={appId}
								appName={appName}
								variant="outline"
								className="ml-auto"
							/>
						</div>
					</div>
				</div>

				<style jsx>{`
					@keyframes kenburns {
						0% { transform: scale(1.05) translateY(0); }
						100% { transform: scale(1.15) translateY(-2%); }
					}
					/* Disable on mobile to save resources; enabled from md via class usage above */
					.animate-kenburns { animation: kenburns 22s ease-in-out infinite alternate; }

					@keyframes floatSlow {
						0%, 100% { transform: translateY(0) translateX(0); opacity: .6; }
						50% { transform: translateY(-10px) translateX(10px); opacity: .9; }
					}
					.animate-float-slow { animation: floatSlow 14s ease-in-out infinite; }

					/* Subtle animated sheen across header content */
					.sheen { position: relative; overflow: hidden; }
					@keyframes sheenMove { 0% { transform: translateX(-100%); } 100% { transform: translateX(100%); } }
					.sheen:before {
						content: '';
						position: absolute;
						inset: 0;
						background: linear-gradient(120deg, transparent, rgba(255,255,255,0.08), transparent);
						transform: translateX(-100%);
						animation: sheenMove 9s linear infinite;
						pointer-events: none;
					}
				`}</style>
			</div>
		</div>
	);
}
