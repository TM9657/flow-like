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
		<div className="relative overflow-hidden rounded-2xl border bg-card group min-h-[220px] lg:min-h-[260px]">
			{/* Background image with subtle Ken Burns animation */}
			<div className="absolute inset-0 z-0 pointer-events-none">
				<img
					src={coverUrl}
					alt=""
					loading="lazy"
					decoding="async"
					className="h-full w-full object-cover scale-105 animate-kenburns"
				/>
				<div className="absolute inset-0 bg-gradient-to-tr from-background/60 via-background/40 to-background/20" />
				<div aria-hidden className="absolute -top-10 -left-10 h-48 w-48 bg-primary/30 blur-3xl rounded-full animate-float-slow" />
				<div aria-hidden className="absolute -bottom-10 -right-10 h-48 w-48 bg-secondary/30 blur-3xl rounded-full animate-float-slow" />
			</div>

			{/* Foreground content */}
			<div className="relative z-10 p-3 pb-8 sm:p-4 md:p-5 lg:p-6 lg:pb-8">
				<div className="rounded-xl border bg-background/60 supports-[backdrop-filter]:bg-background/40 backdrop-blur-md shadow-sm transition-colors sheen">
					<div className="p-3 sm:p-4 md:p-4 lg:p-6">
						<div className="flex flex-col md:flex-row items-start md:items-center gap-4 md:gap-5 lg:gap-6">
							<Avatar className="h-16 w-16 md:h-20 md:w-20 lg:h-24 lg:w-24 shadow-lg border ring-1 ring-border transition-transform duration-300 group-hover:-translate-y-0.5">
								<AvatarImage src={iconUrl} alt={appName} />
								<AvatarFallback className="font-bold">{appName.slice(0, 2).toUpperCase()}</AvatarFallback>
							</Avatar>

							<div className="flex-1 min-w-0">
								<div className="flex flex-wrap items-center gap-3">
									<h1 className="text-xl md:text-2xl lg:text-3xl font-bold tracking-tight truncate">{appName}</h1>
									<Badge>{category}</Badge>
									<Badge variant="outline">{ageRating}</Badge>
									{isMember ? (
										<Badge variant="secondary" className="flex items-center gap-1"><Heart className="h-3 w-3" /> Yours</Badge>
									) : (
										<Badge variant="secondary">{priceLabel}</Badge>
									)}
								</div>

								<div className="mt-2 flex flex-wrap items-center gap-4 text-muted-foreground">
									<div className="flex items-center gap-1 rounded-full border bg-background/50 px-2 py-1 text-foreground">
										<Star className="h-4 w-4 text-yellow-500 fill-yellow-500" />
										<span className="text-xs md:text-sm font-medium">
											{ratingCount > 0 ? `${avgRating.toFixed(1)} (${ratingCount.toLocaleString()})` : "No ratings yet"}
										</span>
									</div>
									<div className="flex items-center gap-1 rounded-full border bg-background/50 px-2 py-1 text-foreground">
										{visibilityIcon(visibility)}
										<span className="text-xs md:text-sm capitalize">{visibilityLabel(visibility)}</span>
									</div>
									{authors?.length ? (
										<div className="text-xs md:text-sm truncate">By {authors.join(", ")}</div>
									) : null}
								</div>
							</div>
						</div>
					</div>

					<style jsx>{`
						@keyframes kenburns {
							0% { transform: scale(1.05) translateY(0); }
							100% { transform: scale(1.15) translateY(-2%); }
						}
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
		</div>
	);
}
