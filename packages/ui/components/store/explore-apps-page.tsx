"use client";

import { Alert, AlertDescription } from "../ui/alert";
import { AppCard } from "../ui/app-card";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import { Skeleton } from "../ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import { useBackend } from "../../state/backend-state";
import { useInfiniteInvoke, useInvoke } from "../../hooks/use-invoke";
import {
	IAppCategory,
	IAppSearchSort,
} from "../../lib/schema/app/app-search-query";
import type { IApp } from "../../lib/schema/app/app";
import type { IMetadata } from "../../lib/schema/bit/bit-pack";
import { motion } from "framer-motion";
import {
	AlertCircle,
	ArrowUpDown,
	Briefcase,
	Gamepad2,
	GraduationCap,
	Heart,
	Loader2,
	MessageCircle,
	Music,
	Package,
	Search,
	Star,
	TrendingUp,
	User,
	Wrench,
	X,
	Zap,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";

const CATEGORY_CONFIG: Record<
	string,
	{
		label: string;
		icon: React.ComponentType<{ className?: string }>;
		color: string;
	}
> = {
	[IAppCategory.Productivity]: {
		label: "Productivity",
		icon: Zap,
		color: "text-yellow-500",
	},
	[IAppCategory.Business]: {
		label: "Business",
		icon: Briefcase,
		color: "text-blue-500",
	},
	[IAppCategory.Education]: {
		label: "Education",
		icon: GraduationCap,
		color: "text-green-500",
	},
	[IAppCategory.Entertainment]: {
		label: "Entertainment",
		icon: Star,
		color: "text-purple-500",
	},
	[IAppCategory.Games]: {
		label: "Games",
		icon: Gamepad2,
		color: "text-red-500",
	},
	[IAppCategory.Communication]: {
		label: "Communication",
		icon: MessageCircle,
		color: "text-cyan-500",
	},
	[IAppCategory.Utilities]: {
		label: "Utilities",
		icon: Wrench,
		color: "text-orange-500",
	},
	[IAppCategory.Music]: {
		label: "Music",
		icon: Music,
		color: "text-pink-500",
	},
	[IAppCategory.Health]: {
		label: "Health",
		icon: Heart,
		color: "text-rose-500",
	},
};

const CATEGORY_LABELS: Record<IAppCategory, string> = {
	[IAppCategory.Anime]: "Anime",
	[IAppCategory.Business]: "Business",
	[IAppCategory.Communication]: "Communication",
	[IAppCategory.Education]: "Education",
	[IAppCategory.Entertainment]: "Entertainment",
	[IAppCategory.Finance]: "Finance",
	[IAppCategory.FoodAndDrink]: "Food & Drink",
	[IAppCategory.Games]: "Games",
	[IAppCategory.Health]: "Health",
	[IAppCategory.Lifestyle]: "Lifestyle",
	[IAppCategory.Music]: "Music",
	[IAppCategory.News]: "News",
	[IAppCategory.Other]: "Other",
	[IAppCategory.Photography]: "Photography",
	[IAppCategory.Productivity]: "Productivity",
	[IAppCategory.Shopping]: "Shopping",
	[IAppCategory.Social]: "Social",
	[IAppCategory.Sports]: "Sports",
	[IAppCategory.Travel]: "Travel",
	[IAppCategory.Utilities]: "Utilities",
	[IAppCategory.Weather]: "Weather",
};

const FEATURED_CATEGORIES = [
	IAppCategory.Productivity,
	IAppCategory.Business,
	IAppCategory.Education,
	IAppCategory.Entertainment,
	IAppCategory.Games,
	IAppCategory.Communication,
	IAppCategory.Utilities,
	IAppCategory.Music,
	IAppCategory.Health,
];

const SORT_LABELS: Record<IAppSearchSort, string> = {
	[IAppSearchSort.BestRated]: "Best Rated",
	[IAppSearchSort.MostPopular]: "Most Popular",
	[IAppSearchSort.NewestCreated]: "Newest",
	[IAppSearchSort.NewestUpdated]: "Recently Updated",
	[IAppSearchSort.MostRelevant]: "Most Relevant",
	[IAppSearchSort.LeastPopular]: "Least Popular",
	[IAppSearchSort.LeastRelevant]: "Least Relevant",
	[IAppSearchSort.OldestCreated]: "Oldest",
	[IAppSearchSort.OldestUpdated]: "Oldest Updated",
	[IAppSearchSort.WorstRated]: "Worst Rated",
};

export function ExploreAppsPage() {
	const router = useRouter();
	const backend = useBackend();

	const [searchQuery, setSearchQuery] = useState("");
	const [debouncedQuery, setDebouncedQuery] = useState("");
	const [selectedCategory, setSelectedCategory] = useState<
		IAppCategory | undefined
	>();
	const [sortBy, setSortBy] = useState<IAppSearchSort>(
		IAppSearchSort.MostPopular,
	);
	const [activeTab, setActiveTab] = useState<"explore" | "yours">("explore");

	const userApps = useInvoke(backend.appState.getApps, backend.appState, []);

	useEffect(() => {
		const timeout = setTimeout(() => {
			setDebouncedQuery(searchQuery);
		}, 300);
		return () => clearTimeout(timeout);
	}, [searchQuery]);

	const {
		data: searchResults,
		hasNextPage,
		fetchNextPage,
		isFetchingNextPage,
		isLoading,
		error,
	} = useInfiniteInvoke(backend.appState.searchApps, backend.appState, [
		undefined,
		debouncedQuery || undefined,
		undefined,
		selectedCategory,
		undefined,
		sortBy,
		undefined,
	]);

	const handleSearch = useCallback((value: string) => {
		setSearchQuery(value);
	}, []);

	const clearFilters = useCallback(() => {
		setSearchQuery("");
		setSelectedCategory(undefined);
		setSortBy(IAppSearchSort.MostPopular);
	}, []);

	const combinedApps = useMemo(() => {
		if (!searchResults) return [];
		return searchResults.pages.flat();
	}, [searchResults]);

	const userAppIds = useMemo(
		() => new Set(userApps.data?.map(([app]) => app.id) ?? []),
		[userApps.data],
	);

	const hasActiveFilters =
		!!debouncedQuery ||
		!!selectedCategory ||
		sortBy !== IAppSearchSort.MostPopular;

	return (
		<main className="flex flex-col flex-1 w-full min-h-0 overflow-hidden">
			<div className="flex-1 min-h-0 overflow-auto">
				<div className="w-full max-w-7xl mx-auto px-4 py-6 space-y-8">
					<header className="space-y-1">
						<h1 className="text-2xl font-semibold tracking-tight">Explore</h1>
						<p className="text-sm text-muted-foreground">
							Discover apps created by the community
						</p>
					</header>

					<Tabs
						value={activeTab}
						onValueChange={(v) => setActiveTab(v as "explore" | "yours")}
						className="w-full"
					>
						<TabsList className="grid w-full sm:w-auto grid-cols-2">
							<TabsTrigger value="explore" className="gap-2">
								<TrendingUp className="w-4 h-4" />
								Explore
							</TabsTrigger>
							<TabsTrigger value="yours" className="gap-2">
								<User className="w-4 h-4" />
								Your Apps
								{userApps.data && userApps.data.length > 0 && (
									<span className="ml-1 text-xs text-muted-foreground">
										{userApps.data.length}
									</span>
								)}
							</TabsTrigger>
						</TabsList>

						<TabsContent value="explore" className="mt-6 space-y-8">
							<div className="flex flex-col lg:flex-row gap-4">
								<div className="relative flex-1">
									<Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
									<Input
										value={searchQuery}
										onChange={(e) => handleSearch(e.target.value)}
										placeholder="Search apps by name, description, or tags..."
										className="rounded-full bg-muted/30 border-border/20 pl-10 pr-10 focus-visible:ring-1"
									/>
									{searchQuery && (
										<Button
											variant="ghost"
											size="icon"
											className="absolute right-1 top-1/2 -translate-y-1/2 h-8 w-8"
											onClick={() => setSearchQuery("")}
										>
											<X className="w-4 h-4" />
										</Button>
									)}
								</div>

								<div className="flex gap-2 flex-wrap">
									<Select
										value={selectedCategory ?? "all"}
										onValueChange={(value) =>
											setSelectedCategory(
												value === "all"
													? undefined
													: (value as IAppCategory),
											)
										}
									>
										<SelectTrigger className="w-40">
											<SelectValue placeholder="Category" />
										</SelectTrigger>
										<SelectContent>
											<SelectItem value="all">All Categories</SelectItem>
											{Object.entries(CATEGORY_LABELS).map(([key, label]) => (
												<SelectItem key={key} value={key}>
													{label}
												</SelectItem>
											))}
										</SelectContent>
									</Select>

									<Select
										value={sortBy}
										onValueChange={(value) =>
											setSortBy(value as IAppSearchSort)
										}
									>
										<SelectTrigger className="w-40">
											<ArrowUpDown className="w-4 h-4 mr-2" />
											<SelectValue placeholder="Sort by" />
										</SelectTrigger>
										<SelectContent>
											{Object.entries(SORT_LABELS)
												.filter(
													([key]) =>
														![
															IAppSearchSort.LeastPopular,
															IAppSearchSort.LeastRelevant,
															IAppSearchSort.OldestCreated,
															IAppSearchSort.OldestUpdated,
															IAppSearchSort.WorstRated,
														].includes(key as IAppSearchSort),
												)
												.map(([key, label]) => (
													<SelectItem key={key} value={key}>
														{label}
													</SelectItem>
												))}
										</SelectContent>
									</Select>

									{hasActiveFilters && (
										<Button variant="ghost" onClick={clearFilters}>
											<X className="w-4 h-4 mr-2" />
											Clear filters
										</Button>
									)}
								</div>
							</div>

							<CategoryChips
								selected={selectedCategory}
								onSelect={setSelectedCategory}
							/>

							{!isLoading && combinedApps.length > 0 && (
								<div className="flex items-center justify-between">
									<p className="text-sm text-muted-foreground">
										{hasActiveFilters ? (
											<>
												Found{" "}
												<span className="font-medium text-foreground">
													{combinedApps.length}
												</span>{" "}
												apps
												{selectedCategory && (
													<>
														{" "}
														in{" "}
														<span className="font-medium text-foreground">
															{CATEGORY_LABELS[selectedCategory]}
														</span>
													</>
												)}
											</>
										) : (
											<>
												Showing{" "}
												<span className="font-medium text-foreground">
													{combinedApps.length}
												</span>{" "}
												popular apps
											</>
										)}
									</p>
								</div>
							)}

							<AppGrid
								apps={combinedApps}
								userAppIds={userAppIds}
								isLoading={isLoading}
								error={error}
								hasNextPage={hasNextPage}
								isFetchingNextPage={isFetchingNextPage}
								onFetchNextPage={fetchNextPage}
								onAppClick={(appId) => {
									if (userAppIds.has(appId)) {
										router.push(`/use?id=${appId}`);
									} else {
										router.push(`/store?id=${appId}`);
									}
								}}
								emptyMessage={
									hasActiveFilters
										? "No apps match your filters"
										: "No apps found"
								}
							/>
						</TabsContent>

						<TabsContent value="yours" className="mt-6 space-y-8">
							<UserAppsSection
								apps={userApps.data ?? []}
								isLoading={userApps.isLoading}
								onAppClick={(appId) => router.push(`/use?id=${appId}`)}
								onSettingsClick={(appId) =>
									router.push(`/library/config?id=${appId}`)
								}
							/>
						</TabsContent>
					</Tabs>
				</div>
			</div>
		</main>
	);
}

function CategoryChips({
	selected,
	onSelect,
}: {
	selected?: IAppCategory;
	onSelect: (category?: IAppCategory) => void;
}) {
	return (
		<div className="flex flex-wrap gap-2">
			{FEATURED_CATEGORIES.map((category) => {
				const config = CATEGORY_CONFIG[category];
				if (!config) return null;
				const Icon = config.icon;
				const isSelected = selected === category;

				return (
					<button
						key={category}
						type="button"
						className={`rounded-full px-3 py-1.5 text-xs transition-colors flex items-center gap-1.5 ${
							isSelected
								? "bg-primary text-primary-foreground"
								: "bg-muted/30 text-muted-foreground hover:bg-muted/50"
						}`}
						onClick={() => onSelect(isSelected ? undefined : category)}
					>
						<Icon
							className={`w-3.5 h-3.5 ${isSelected ? "" : config.color}`}
						/>
						{config.label}
					</button>
				);
			})}
		</div>
	);
}

function AppGrid({
	apps,
	userAppIds,
	isLoading,
	error,
	hasNextPage,
	isFetchingNextPage,
	onFetchNextPage,
	onAppClick,
	emptyMessage,
}: {
	apps: [IApp, IMetadata | undefined][];
	userAppIds: Set<string>;
	isLoading: boolean;
	error: Error | null;
	hasNextPage?: boolean;
	isFetchingNextPage: boolean;
	onFetchNextPage: () => void;
	onAppClick: (appId: string) => void;
	emptyMessage: string;
}) {
	if (error) {
		return (
			<Alert variant="destructive">
				<AlertCircle className="h-4 w-4" />
				<AlertDescription>
					Failed to load apps: {error.message}
				</AlertDescription>
			</Alert>
		);
	}

	if (isLoading) {
		return (
			<div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
				{[...Array(6)].map((_, i) => (
					<Skeleton key={i} className="h-48 w-full" />
				))}
			</div>
		);
	}

	if (apps.length === 0) {
		return (
			<div className="text-center py-16">
				<Package className="w-12 h-12 mx-auto text-muted-foreground/30 mb-3" />
				<h3 className="text-lg font-semibold mb-2">{emptyMessage}</h3>
				<p className="text-sm text-muted-foreground">
					Try adjusting your search or filters to find what you're looking for.
				</p>
			</div>
		);
	}

	return (
		<>
			<motion.div
				initial={{ opacity: 0 }}
				animate={{ opacity: 1 }}
				className="grid gap-6 md:grid-cols-2 lg:grid-cols-3"
			>
				{apps.map(([app, metadata]) => (
					<motion.div
						key={app.id}
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
					>
						<AppCard
							isOwned={userAppIds.has(app.id)}
							app={app}
							metadata={metadata}
							variant="extended"
							className="w-full h-full"
							onClick={() => onAppClick(app.id)}
						/>
					</motion.div>
				))}
			</motion.div>

			{hasNextPage && (
				<div className="flex justify-center mt-8">
					<button
						type="button"
						onClick={onFetchNextPage}
						disabled={isFetchingNextPage}
						className="rounded-full text-sm text-muted-foreground/60 border border-border/30 hover:bg-muted/30 px-6 py-2 transition-colors disabled:opacity-50"
					>
						{isFetchingNextPage ? (
							<span className="flex items-center gap-2">
								<Loader2 className="w-4 h-4 animate-spin" />
								Loading more...
							</span>
						) : (
							"Load More Apps"
						)}
					</button>
				</div>
			)}
		</>
	);
}

function UserAppsSection({
	apps,
	isLoading,
	onAppClick,
	onSettingsClick,
}: {
	apps: [IApp, IMetadata | undefined][];
	isLoading: boolean;
	onAppClick: (appId: string) => void;
	onSettingsClick: (appId: string) => void;
}) {
	if (isLoading) {
		return (
			<div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
				{[...Array(3)].map((_, i) => (
					<Skeleton key={i} className="h-48 w-full" />
				))}
			</div>
		);
	}

	if (apps.length === 0) {
		return (
			<div className="text-center py-16">
				<Package className="w-12 h-12 mx-auto text-muted-foreground/30 mb-3" />
				<h3 className="text-lg font-semibold mb-2">No apps yet</h3>
				<p className="text-sm text-muted-foreground">
					You haven't joined any apps yet. Explore the marketplace to find apps
					that interest you!
				</p>
			</div>
		);
	}

	return (
		<motion.div
			initial={{ opacity: 0 }}
			animate={{ opacity: 1 }}
			className="grid gap-6 md:grid-cols-2 lg:grid-cols-3"
		>
			{apps.map(([app, metadata]) => (
				<motion.div
					key={app.id}
					initial={{ opacity: 0 }}
					animate={{ opacity: 1 }}
				>
					<AppCard
						isOwned
						app={app}
						metadata={metadata}
						variant="extended"
						className="w-full h-full"
						onClick={() => onAppClick(app.id)}
						onSettingsClick={() => onSettingsClick(app.id)}
					/>
				</motion.div>
			))}
		</motion.div>
	);
}
