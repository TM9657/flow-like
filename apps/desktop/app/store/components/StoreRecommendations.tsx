"use client";

import {
	Alert,
	AlertDescription,
	AppCard,
	Button,
	Card,
	CardContent,
	Skeleton,
	useBackend,
	useInfiniteInvoke,
} from "@tm9657/flow-like-ui";
import { IAppSearchSort } from "@tm9657/flow-like-ui/lib/schema/app/app-search-query";
import { motion } from "framer-motion";
import { AlertCircle, Loader2, Package, SparklesIcon } from "lucide-react";
import { useRouter } from "next/navigation";
import { memo, useMemo } from "react";

const itemVariants = {
	hidden: { opacity: 0, y: 20 },
	visible: { opacity: 1, y: 0 },
};

export const StoreRecommendations = memo(function StoreRecommendations() {
	const backend = useBackend();
	const router = useRouter();

	const {
		data: apps,
		hasNextPage,
		fetchNextPage,
		isFetchingNextPage,
		isLoading: isAppsLoading,
		error: appsError,
	} = useInfiniteInvoke(backend.appState.searchApps, backend.appState, [
		undefined,
		undefined,
		undefined,
		undefined,
		undefined,
		IAppSearchSort.BestRated,
		undefined,
	]);

	const combinedApps = useMemo(() => {
		if (!apps) return [];
		return apps.pages.flat();
	}, [apps]);

	if (!combinedApps) return null;

	return (
		<section className="space-y-4">
			{/* Apps Section */}
			<motion.div variants={itemVariants} className="relative">
				<div className="absolute inset-0" />
				<Card className="relative bg-transparent border-0 shadow-none">
					<CardContent className="p-4">
						<h2 className="text-2xl font-semibold mb-6 flex items-center gap-2">
							<SparklesIcon className="w-6 h-6 text-primary" />
							You might also like
						</h2>

						{appsError && (
							<Alert className="mb-6 border-destructive/20 bg-destructive/5">
								<AlertCircle className="h-4 w-4" />
								<AlertDescription>
									Failed to load apps: {appsError.message}
								</AlertDescription>
							</Alert>
						)}

						{isAppsLoading ? (
							<div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
								{[...Array(6)].map((_, i) => (
									<div key={i} className="space-y-3">
										<Skeleton className="h-48 w-full rounded-lg" />
										<Skeleton className="h-4 w-3/4" />
										<Skeleton className="h-4 w-1/2" />
									</div>
								))}
							</div>
						) : combinedApps.length === 0 ? (
							<div className="text-center py-12">
								<Package className="w-16 h-16 mx-auto text-muted-foreground/50 mb-4" />
								<p className="text-lg text-muted-foreground">
									Could not find any apps to recommend right now.
								</p>
							</div>
						) : (
							<>
								<div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
									{combinedApps.map(([app, metadata]) => (
										<AppCard
											key={app.id}
											apps={combinedApps.map(([a]) => a)}
											app={app}
											variant="extended"
											metadata={metadata}
											className="w-full"
											onClick={() => router.push(`/store?id=${app.id}`)}
										/>
									))}
								</div>

								{hasNextPage && (
									<div className="flex justify-center mt-8">
										<Button
											onClick={() => fetchNextPage()}
											disabled={isFetchingNextPage}
											variant="outline"
											size="lg"
											className="px-8"
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
						)}
					</CardContent>
				</Card>
			</motion.div>
		</section>
	);
});
