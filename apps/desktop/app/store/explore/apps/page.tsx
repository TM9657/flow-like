"use client";

import {
	Alert,
	AlertDescription,
	AppCard,
	Badge,
	Button,
	Input,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Skeleton,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	useBackend,
	useInfiniteInvoke,
	useInvoke,
} from "@tm9657/flow-like-ui";
import {
	IAppCategory,
	IAppSearchSort,
} from "@tm9657/flow-like-ui/lib/schema/app/app-search-query";
import { motion } from "framer-motion";
import {
	AlertCircle,
	ArrowUpDown,
	Briefcase,
	Gamepad2,
	GraduationCap,
	Grid3X3,
	Heart,
	List,
	Loader2,
	MessageCircle,
	Music,
	Package,
	Search,
	Sparkles,
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
	[IAppCategory.Music]: { label: "Music", icon: Music, color: "text-pink-500" },
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

const containerVariants = {
	hidden: { opacity: 0 },
	visible: {
		opacity: 1,
		transition: { staggerChildren: 0.05 },
	},
};

const itemVariants = {
	hidden: { opacity: 0, y: 20 },
	visible: { opacity: 1, y: 0 },
};

export default function ExploreAppsPage() {
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
	const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
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
				<div className="w-full max-w-7xl mx-auto px-4 py-6 space-y-6">
					<header className="space-y-2">
						<div className="flex items-center gap-3">
							<div className="p-2 rounded-xl bg-primary/10">
								<Sparkles className="w-6 h-6 text-primary" />
							</div>
							<div>
								<h1 className="text-3xl font-bold tracking-tight">
									Explore Apps
								</h1>
								<p className="text-muted-foreground">
									Discover apps created by the community
								</p>
							</div>
						</div>
					</header>

					<Tabs
						value={activeTab}
						onValueChange={(v) => setActiveTab(v as "explore" | "yours")}
						className="w-full"
					>
						<div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
							<TabsList className="grid w-full sm:w-auto grid-cols-2">
								<TabsTrigger value="explore" className="gap-2">
									<TrendingUp className="w-4 h-4" />
									Explore
								</TabsTrigger>
								<TabsTrigger value="yours" className="gap-2">
									<User className="w-4 h-4" />
									Your Apps
									{userApps.data && userApps.data.length > 0 && (
										<Badge variant="secondary" className="ml-1">
											{userApps.data.length}
										</Badge>
									)}
								</TabsTrigger>
							</TabsList>

							<div className="flex items-center gap-2">
								<Button
									variant={viewMode === "grid" ? "secondary" : "ghost"}
									size="icon"
									onClick={() => setViewMode("grid")}
								>
									<Grid3X3 className="w-4 h-4" />
								</Button>
								<Button
									variant={viewMode === "list" ? "secondary" : "ghost"}
									size="icon"
									onClick={() => setViewMode("list")}
								>
									<List className="w-4 h-4" />
								</Button>
							</div>
						</div>

						<TabsContent value="explore" className="mt-6 space-y-6">
							<div className="flex flex-col lg:flex-row gap-4">
								<div className="relative flex-1">
									<Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
									<Input
										value={searchQuery}
										onChange={(e) => handleSearch(e.target.value)}
										placeholder="Search apps by name, description, or tags..."
										className="pl-10 pr-10"
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
												value === "all" ? undefined : (value as IAppCategory),
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
								viewMode={viewMode}
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

						<TabsContent value="yours" className="mt-6 space-y-6">
							<UserAppsSection
								apps={userApps.data ?? []}
								isLoading={userApps.isLoading}
								viewMode={viewMode}
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

				const iconClassName = `w-3.5 h-3.5 ${isSelected ? "" : config.color}`;

				return (
					<Badge
						key={category}
						variant={isSelected ? "default" : "outline"}
						className="cursor-pointer hover:bg-primary/20 transition-all duration-200 gap-1.5 py-1.5 px-3"
						onClick={() => onSelect(isSelected ? undefined : category)}
					>
						<Icon className={iconClassName} />
						{config.label}
					</Badge>
				);
			})}
		</div>
	);
}

function AppGrid({
	apps,
	userAppIds,
	viewMode,
	isLoading,
	error,
	hasNextPage,
	isFetchingNextPage,
	onFetchNextPage,
	onAppClick,
	emptyMessage,
}: {
	apps: [
		import("@tm9657/flow-like-ui").IApp,
		import("@tm9657/flow-like-ui").IMetadata | undefined,
	][];
	userAppIds: Set<string>;
	viewMode: "grid" | "list";
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
			<div
				className={
					viewMode === "grid"
						? "grid gap-6 md:grid-cols-2 lg:grid-cols-3"
						: "flex flex-col gap-4"
				}
			>
				{[...Array(6)].map((_, i) => (
					<Skeleton
						key={i}
						className={viewMode === "grid" ? "h-48 w-full" : "h-20 w-full"}
					/>
				))}
			</div>
		);
	}

	if (apps.length === 0) {
		return (
			<div className="text-center py-16">
				<Package className="w-16 h-16 mx-auto text-muted-foreground/50 mb-4" />
				<h3 className="text-lg font-semibold mb-2">{emptyMessage}</h3>
				<p className="text-muted-foreground">
					Try adjusting your search or filters to find what you're looking for.
				</p>
			</div>
		);
	}

	return (
		<>
			<motion.div
				variants={containerVariants}
				initial="hidden"
				animate="visible"
				className={
					viewMode === "grid"
						? "grid gap-6 md:grid-cols-2 lg:grid-cols-3"
						: "flex flex-col gap-4"
				}
			>
				{apps.map(([app, metadata]) => (
					<motion.div key={app.id} variants={itemVariants}>
						<AppCard
							isOwned={userAppIds.has(app.id)}
							app={app}
							metadata={metadata}
							variant={viewMode === "grid" ? "extended" : "small"}
							className="w-full h-full"
							onClick={() => onAppClick(app.id)}
						/>
					</motion.div>
				))}
			</motion.div>

			{hasNextPage && (
				<div className="flex justify-center mt-8">
					<Button
						onClick={onFetchNextPage}
						disabled={isFetchingNextPage}
						variant="outline"
						size="lg"
					>
						{isFetchingNextPage ? (
							<>
								<Loader2 className="w-4 h-4 mr-2 animate-spin" />
								Loading more...
							</>
						) : (
							"Load More Apps"
						)}
					</Button>
				</div>
			)}
		</>
	);
}

function UserAppsSection({
	apps,
	isLoading,
	viewMode,
	onAppClick,
	onSettingsClick,
}: {
	apps: [
		import("@tm9657/flow-like-ui").IApp,
		import("@tm9657/flow-like-ui").IMetadata | undefined,
	][];
	isLoading: boolean;
	viewMode: "grid" | "list";
	onAppClick: (appId: string) => void;
	onSettingsClick: (appId: string) => void;
}) {
	if (isLoading) {
		return (
			<div
				className={
					viewMode === "grid"
						? "grid gap-6 md:grid-cols-2 lg:grid-cols-3"
						: "flex flex-col gap-4"
				}
			>
				{[...Array(3)].map((_, i) => (
					<Skeleton
						key={i}
						className={viewMode === "grid" ? "h-48 w-full" : "h-20 w-full"}
					/>
				))}
			</div>
		);
	}

	if (apps.length === 0) {
		return (
			<div className="text-center py-16">
				<Package className="w-16 h-16 mx-auto text-muted-foreground/50 mb-4" />
				<h3 className="text-lg font-semibold mb-2">No apps yet</h3>
				<p className="text-muted-foreground mb-6">
					You haven't joined any apps yet. Explore the marketplace to find apps
					that interest you!
				</p>
			</div>
		);
	}

	return (
		<motion.div
			variants={containerVariants}
			initial="hidden"
			animate="visible"
			className={
				viewMode === "grid"
					? "grid gap-6 md:grid-cols-2 lg:grid-cols-3"
					: "flex flex-col gap-4"
			}
		>
			{apps.map(([app, metadata]) => (
				<motion.div key={app.id} variants={itemVariants}>
					<AppCard
						isOwned
						app={app}
						metadata={metadata}
						variant={viewMode === "grid" ? "extended" : "small"}
						className="w-full h-full"
						onClick={() => onAppClick(app.id)}
						onSettingsClick={() => onSettingsClick(app.id)}
					/>
				</motion.div>
			))}
		</motion.div>
	);
}
