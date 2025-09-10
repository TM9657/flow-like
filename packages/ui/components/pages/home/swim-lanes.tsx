"use client";
import { useQuery } from "@tanstack/react-query";
import { ArrowRight, ExternalLink } from "lucide-react";
import { useInvoke } from "../../../hooks";
import type { IApp, IAppCategory } from "../../../lib";
import type { IAppSearchSort } from "../../../lib/schema/app/app-search-query";
import { type IBackendState, useBackend } from "../../../state/backend-state";
import {
	Alert,
	AlertDescription,
	AlertTitle,
	BitCard,
	DynamicImage,
	Skeleton,
} from "../../ui";
import { AppCard } from "../../ui/app-card";

export interface ISwimlaneItem {
	id: string;
	type: "app" | "model" | "static";
	appId?: string;
	modelId?: string;
	hub?: string;
	title?: string;
	description?: string;
	image?: string;
	link?: string;
	badge?: string;
	icon?: string;
	gradient?: string;
}

export interface ISearchQuery {
	id?: string;
	type: "search";
	query?: string;
	limit?: number;
	offset?: number;
	category?: IAppCategory;
	author?: string;
	sort?: IAppSearchSort;
	tag?: string;
}

export interface ISwimlane {
	id: string;
	title: string;
	subtitle?: string;
	size: "large" | "medium" | "small";
	items?: (ISwimlaneItem | ISwimlaneItem[] | ISearchQuery)[];
	viewAllLink?: string;
}

const swimlanesUrl = "https://cdn.flow-like.com/swimlanes.json";

function useSwimlanes() {
	return useQuery<ISwimlane[]>({
		queryKey: ["swimlanes"],
		queryFn: async () => {
			const res = await fetch(swimlanesUrl, {
				cache: "no-cache"
			});
			if (!res.ok) throw new Error("Failed to fetch swimlanes");
			return res.json();
		},
		retry: 1,
		refetchOnWindowFocus: true,
		refetchOnReconnect: true,
		refetchOnMount: "always",
		staleTime: 1000 * 60 * 60,
		gcTime: 1000 * 60 * 60 * 24 * 7,
		placeholderData: (prev) => prev,
		networkMode: "offlineFirst",
	});
}

export function HomeSwimlanes() {
	const backend = useBackend();
	const apps = useInvoke(backend.appState.getApps, backend.appState, []);
	const { data, error } = useSwimlanes();

	if (error) {
		return (
			<main className="min-h-screen items-center w-full max-h-dvh overflow-auto p-4 grid grid-cols-6 justify-start gap-2">
				<div className="col-span-6">
					<Alert variant="destructive">
						<ExternalLink className="h-4 w-4" />
						<AlertTitle>Connection Error</AlertTitle>
						<AlertDescription>
							Failed to load swimlanes. Please check your internet connection or
							try again later.
							{error.message && (
								<details className="mt-2">
									<summary className="cursor-pointer text-sm opacity-80 hover:opacity-100">
										Technical details
									</summary>
									<code className="text-xs bg-background/50 px-2 py-1 rounded mt-1 block">
										{error.message}
									</code>
								</details>
							)}
						</AlertDescription>
					</Alert>
				</div>
				<Skeleton className="col-span-6 h-full min-h-[30dvh]" />
				<Skeleton className="col-span-3 h-full min-h-[20dvh]" />
				<Skeleton className="col-span-3 h-full" />
				<Skeleton className="col-span-2 h-full" />
				<Skeleton className="col-span-2 h-full" />
				<Skeleton className="col-span-2 h-full" />
			</main>
		);
	}

	if (!data)
		return (
			<main className="min-h-screen items-center w-full max-h-dvh overflow-auto p-4 grid grid-cols-6 justify-start gap-2">
				<Skeleton className="col-span-6 h-full min-h-[30dvh]" />
				<Skeleton className="col-span-3 h-full min-h-[20dvh]" />
				<Skeleton className="col-span-3 h-full" />
				<Skeleton className="col-span-2 h-full" />
				<Skeleton className="col-span-2 h-full" />
				<Skeleton className="col-span-2 h-full" />
			</main>
		);

	return (
		<main className="min-h-screen w-full max-h-dvh overflow-auto bg-background flex flex-col items-center">
			<div className="w-full space-y-8 p-6 max-w-[1800px]">
				{data?.map((swimlane) => (
					<SwimlaneSection
						key={swimlane.id}
						swimlane={swimlane}
						apps={apps.data?.map((i) => i[0]) ?? []}
					/>
				))}
			</div>
		</main>
	);
}

function SwimlaneSection({
	swimlane,
	apps,
}: Readonly<{ swimlane: ISwimlane; apps: IApp[] }>) {
	const getGridCols = () => {
		switch (swimlane.size) {
			case "large":
				return "grid-cols-1 lg:grid-cols-2";
			case "medium":
				return "grid-cols-1 md:grid-cols-2 xl:grid-cols-3";
			case "small":
				return "grid-cols-1 md:grid-cols-2 lg:grid-cols-3";
		}
	};

	const getItemSize = () => {
		switch (swimlane.size) {
			case "large":
				return "extended";
			case "medium":
			case "small":
				return "small";
		}
	};

	return (
		<section className="space-y-4">
			<SwimlaneHeader swimlane={swimlane} apps={apps} />
			<div className={`grid ${getGridCols()} gap-4`}>
				{swimlane.items?.map((item, index) => (
					<SwimlaneSlot
						key={`slot-${index}`}
						items={Array.isArray(item) ? item : [item]}
						size={swimlane.size}
						variant={getItemSize()}
						apps={apps}
					/>
				))}
			</div>
		</section>
	);
}

function SwimlaneSlot({
	items,
	size,
	variant,
	apps,
}: Readonly<{
	items: (ISwimlaneItem | ISearchQuery)[];
	size: "large" | "medium" | "small";
	variant: "extended" | "small";
	apps: IApp[];
}>) {
	if (items.length === 1) {
		return (
			<SwimlaneItemOrSearch
				item={items[0]}
				size={size}
				variant={variant}
				apps={apps}
			/>
		);
	}

	const isHorizontal = size === "large" || size === "medium";
	const scrollClass = isHorizontal
		? "flex gap-3 overflow-hidden scrollbar-hide pb-2 w-full flex-row justify-stretch"
		: "flex flex-col gap-3 overflow-y-auto scrollbar-hide max-h-[600px]";

	return (
		<div className={scrollClass}>
			{items.map((item) => (
				<div key={item.id} className={isHorizontal ? "grow w-full" : ""}>
					<SwimlaneItemOrSearch
						item={item}
						size={size}
						variant={variant}
						apps={apps}
					/>
				</div>
			))}
		</div>
	);
}

function SwimlaneItemOrSearch({
	item,
	size,
	variant,
	apps,
}: Readonly<{
	item: ISwimlaneItem | ISearchQuery;
	size: "large" | "medium" | "small";
	variant: "extended" | "small";
	apps: IApp[];
}>) {
	if (item.type === "search") {
		return (
			<SearchResults
				searchQuery={item}
				size={size}
				variant={variant}
				apps={apps}
			/>
		);
	}

	return <SwimlaneItem item={item} size={size} variant={variant} apps={apps} />;
}

function SearchResults({
	searchQuery,
	size,
	variant,
	apps,
}: Readonly<{
	searchQuery: ISearchQuery;
	size: "large" | "medium" | "small";
	variant: "extended" | "small";
	apps: IApp[];
}>) {
	const backend = useBackend();
	const searchResults = useInvoke(
		backend.appState.searchApps,
		backend.appState,
		[
			searchQuery.id,
			searchQuery.query,
			undefined,
			searchQuery.category,
			searchQuery.author,
			searchQuery.sort,
			searchQuery.tag,
			searchQuery.offset,
			searchQuery.limit,
		],
	);

	const getMaxSearchItems = () => {
		switch (size) {
			case "large":
				return 3;
			case "medium":
				return 4;
			case "small":
				return 5;
		}
	};

	if (!searchResults.data || searchResults.data.length === 0) {
		return (
			<div className="flex items-center justify-center h-32 text-muted-foreground">
				<span>No results found</span>
			</div>
		);
	}

	const searchItems = searchResults.data
		.slice(0, getMaxSearchItems())
		.map(([app, metadata], index) => ({
			id: `search-${searchQuery.id}-${app.id}-${index}`,
			type: "app" as const,
			appId: app.id,
		}));

	const isHorizontal = size === "large" || size === "medium";
	const scrollClass = isHorizontal
		? "flex gap-3 overflow-hidden scrollbar-hide pb-2 w-full flex-row justify-stretch"
		: "flex flex-col gap-3 overflow-y-auto scrollbar-hide max-h-[600px]";

	return (
		<div className={scrollClass}>
			{searchItems.map((item) => (
				<div key={item.id} className={isHorizontal ? "grow w-full" : ""}>
					<SwimlaneItem item={item} size={size} variant={variant} apps={apps} />
				</div>
			))}
		</div>
	);
}

function SwimlaneHeader({
	swimlane,
	apps,
}: Readonly<{ swimlane: ISwimlane; apps: IApp[] }>) {
	return (
		<div className="flex items-center justify-between">
			<div className="space-y-1">
				<h2 className="text-2xl font-bold text-foreground">{swimlane.title}</h2>
				{swimlane.subtitle && (
					<p className="text-muted-foreground">{swimlane.subtitle}</p>
				)}
			</div>
			{swimlane.viewAllLink && (
				<a href={swimlane.viewAllLink}>
					<button
						type="button"
						className="flex items-center gap-2 text-sm font-medium text-primary hover:text-primary/80 transition-colors"
					>
						View All
						<ArrowRight className="w-4 h-4" />
					</button>
				</a>
			)}
		</div>
	);
}

function SwimlaneItem({
	item,
	size,
	variant,
	apps,
}: Readonly<{
	item: ISwimlaneItem;
	size: "large" | "medium" | "small";
	variant: "extended" | "small";
	apps: IApp[];
}>) {
	const backend = useBackend();

	if (item.type === "app" && item.appId) {
		return (
			<AppCardLoading
				appId={item.appId}
				variant={variant}
				backend={backend}
				apps={apps}
			/>
		);
	}

	if (item.type === "model" && item.modelId && item.hub) {
		return (
			<BitCardLoading backend={backend} bitId={item.modelId} hub={item.hub} />
		);
	}

	if (item.type === "static") {
		return <StaticCard item={item} size={size} />;
	}

	return null;
}

function StaticCard({
	item,
	size,
}: Readonly<{
	item: ISwimlaneItem;
	size: "large" | "medium" | "small";
}>) {
	const isLarge = size === "large";
	const cardHeight = isLarge ? "h-[375px]" : "min-h-[200px]";

	return (
		<a
			type="button"
			href={item.link}
			target={item.link?.startsWith("http") ? "_blank" : "_self"}
			className={`group relative overflow-hidden rounded-xl border border-border/40 bg-card shadow-sm hover:shadow-xl hover:border-primary/30 transition-all duration-300 ${cardHeight} w-full`}
		>
			<div className="absolute inset-0">
				{item.image ? (
					<img
						src={item.image}
						alt={item.title}
						className="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
					/>
				) : (
					<div
						className={`w-full h-full bg-linear-to-br ${
							item.gradient || "from-primary/20 to-primary/40"
						}`}
					/>
				)}
				<div className="absolute inset-0 bg-linear-to-t from-black/20 via-black/5 dark:from-black/60 dark:via-black/20 to-transparent" />
			</div>

			<div className="relative z-10 flex flex-col justify-between h-full p-6">
				{item.badge && (
					<div className="self-start">
						<div className="bg-white/90 backdrop-blur-xs text-gray-900 rounded-full px-3 py-1 text-xs font-bold shadow-lg">
							{item.badge}
						</div>
					</div>
				)}

				<div className="space-y-3">
					<div className="flex items-center gap-2">
						{item.icon && (
							<div className="p-2 bg-white/20 backdrop-blur-xs rounded-full text-white">
								<DynamicImage url={item.icon} className="w-5 h-5 bg-white" />
							</div>
						)}
						<h3 className="font-bold text-white text-left text-lg leading-tight">
							{item.title}
						</h3>
					</div>
					{item.description && (
						<p className="text-white/90 text-left text-sm leading-relaxed max-w-md">
							{item.description}
						</p>
					)}
				</div>

				{item.link && size === "large" && (
					<div className="flex items-center gap-2 text-white/80 group-hover:text-white transition-colors">
						<span className="text-sm font-medium">Learn More</span>
						<ExternalLink className="w-4 h-4" />
					</div>
				)}
			</div>
		</a>
	);
}

function BitCardLoading({
	bitId,
	hub,
	backend,
}: Readonly<{ bitId: string; hub: string; backend: IBackendState }>) {
	const bit = useInvoke(backend.bitState.getBit, backend.bitState, [
		bitId,
		hub,
	]);

	if (!bit.data) {
		return <Skeleton className="w-full h-full rounded-lg" />;
	}

	return <BitCard bit={bit.data} wide={false} />;
}

function AppCardLoading({
	appId,
	variant,
	backend,
	apps,
}: Readonly<{
	appId: string;
	backend: IBackendState;
	variant: "small" | "extended";
	apps: IApp[];
}>) {
	const app = useInvoke(backend.appState.searchApps, backend.appState, [appId]);

	if (!app.data || (app.data?.length ?? 0) <= 0) {
		return (
			<Skeleton
				className={`w-full h-full rounded-lg ${variant === "extended" ? "min-w-72 h-[375px]" : "h-[60px] min-w-1/3 w-full"}`}
			/>
		);
	}

	const meta = app.data[0][1];
	const data = app.data[0][0];

	return (
		<AppCard
			apps={apps}
			app={data}
			metadata={meta}
			variant={variant}
			className={"w-full max-w-full h-full flex grow"}
		/>
	);
}
