"use client";

import {
	type PackageSummary,
	type SearchFilters,
	type SearchResults,
	useBackend,
	useInvoke,
	useQuery,
} from "@tm9657/flow-like-ui";
import {
	Badge,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	Input,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Skeleton,
	StorePackageDetail,
} from "@tm9657/flow-like-ui";
import { useDebounce } from "@uidotdev/usehooks";
import {
	Download,
	ExternalLink,
	Package,
	Search,
	Shield,
	SlidersHorizontal,
} from "lucide-react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useCallback, useState } from "react";
import { useAuth } from "react-oidc-context";
import { toast } from "sonner";
import { fetcher } from "../../../lib/api";

type SortOption =
	| "relevance"
	| "name"
	| "downloads"
	| "updated_at"
	| "created_at";

function PackageCard({ pkg }: { pkg: PackageSummary }) {
	return (
		<Card className="flex flex-col h-full hover:shadow-md transition-shadow">
			<CardHeader className="pb-2">
				<div className="flex items-start justify-between">
					<div className="flex items-center gap-2">
						<Package className="h-5 w-5 text-muted-foreground" />
						<CardTitle className="text-base">{pkg.name}</CardTitle>
					</div>
					{pkg.verified && (
						<Badge variant="secondary" className="gap-1">
							<Shield className="h-3 w-3" />
							Verified
						</Badge>
					)}
				</div>
				<CardDescription className="line-clamp-2 text-sm">
					{pkg.description}
				</CardDescription>
			</CardHeader>
			<CardContent className="flex-1 pb-2">
				<div className="flex flex-wrap gap-1 mb-3">
					{pkg.keywords.slice(0, 3).map((keyword) => (
						<Badge key={keyword} variant="outline" className="text-xs">
							{keyword}
						</Badge>
					))}
					{pkg.keywords.length > 3 && (
						<Badge variant="outline" className="text-xs">
							+{pkg.keywords.length - 3}
						</Badge>
					)}
				</div>
				<div className="flex items-center gap-4 text-xs text-muted-foreground">
					<span className="flex items-center gap-1">
						<Download className="h-3 w-3" />
						{pkg.downloadCount.toLocaleString()}
					</span>
					<span>v{pkg.latestVersion}</span>
				</div>
			</CardContent>
			<CardFooter className="pt-2">
				<Link href={`/store/packages?id=${pkg.id}`} className="w-full">
					<Button variant="outline" className="w-full">
						View Package
						<ExternalLink className="ml-2 h-4 w-4" />
					</Button>
				</Link>
			</CardFooter>
		</Card>
	);
}

function PackageCardSkeleton() {
	return (
		<Card className="flex flex-col h-full">
			<CardHeader className="pb-2">
				<div className="flex items-start justify-between">
					<Skeleton className="h-5 w-32" />
					<Skeleton className="h-5 w-16" />
				</div>
				<Skeleton className="h-8 w-full mt-2" />
			</CardHeader>
			<CardContent className="flex-1 pb-2">
				<div className="flex gap-1 mb-3">
					<Skeleton className="h-5 w-12" />
					<Skeleton className="h-5 w-16" />
					<Skeleton className="h-5 w-10" />
				</div>
				<Skeleton className="h-4 w-24" />
			</CardContent>
			<CardFooter className="pt-2">
				<Skeleton className="h-9 w-full" />
			</CardFooter>
		</Card>
	);
}

function PackageDetailWrapper() {
	const searchParams = useSearchParams();
	const router = useRouter();
	const auth = useAuth();
	const packageId = searchParams.get("id") ?? "";

	const handleBack = useCallback(() => router.back(), [router]);

	return (
		<StorePackageDetail
			packageId={packageId}
			onBack={handleBack}
			onInstallSuccess={() => toast.success("Package installed successfully")}
			onUninstallSuccess={() =>
				toast.success("Package uninstalled successfully")
			}
			onInstallError={(error) =>
				toast.error(`Failed to install package: ${error.message}`)
			}
			onUninstallError={(error) =>
				toast.error(`Failed to uninstall package: ${error.message}`)
			}
			fetcher={fetcher}
			auth={auth}
		/>
	);
}

function PackageListContent() {
	const auth = useAuth();
	const backend = useBackend();
	const profile = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
	);

	const [searchQuery, setSearchQuery] = useState("");
	const [sortBy, setSortBy] = useState<SortOption>("downloads");
	const [verifiedOnly, setVerifiedOnly] = useState(false);
	const [offset, setOffset] = useState(0);
	const limit = 12;

	const debouncedQuery = useDebounce(searchQuery, 300);

	const buildFilters = useCallback((): SearchFilters => {
		return {
			query: debouncedQuery || undefined,
			sortBy,
			sortDesc: true,
			verifiedOnly,
			offset,
			limit,
		};
	}, [debouncedQuery, sortBy, verifiedOnly, offset, limit]);

	const searchResults = useQuery({
		queryKey: ["registry-search", debouncedQuery, sortBy, verifiedOnly, offset],
		queryFn: async () => {
			if (!profile.data) return null;
			const params = new URLSearchParams();
			if (debouncedQuery) params.set("query", debouncedQuery);
			params.set("sort_by", sortBy);
			params.set("sort_desc", "true");
			params.set("verified_only", String(verifiedOnly));
			params.set("offset", String(offset));
			params.set("limit", String(limit));

			return fetcher<SearchResults>(
				profile.data.hub_profile,
				`registry/search?${params.toString()}`,
				{ method: "GET" },
				auth,
			);
		},
		enabled: !!profile.data,
	});

	const totalPages = Math.ceil((searchResults.data?.totalCount ?? 0) / limit);
	const currentPage = Math.floor(offset / limit) + 1;

	return (
		<main className="flex-col flex grow max-h-full p-6 overflow-auto min-h-0 w-full">
			<div className="mx-auto w-full max-w-7xl space-y-6">
				{/* Header */}
				<div className="space-y-2">
					<h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
						<Package className="h-8 w-8" />
						Package Registry
					</h1>
					<p className="text-muted-foreground">
						Discover and install WASM node packages to extend your workflows
					</p>
				</div>

				{/* Search and Filters */}
				<div className="flex flex-col sm:flex-row gap-4">
					<div className="relative flex-1">
						<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
						<Input
							placeholder="Search packages..."
							value={searchQuery}
							onChange={(e) => {
								setSearchQuery(e.target.value);
								setOffset(0);
							}}
							className="pl-10"
						/>
					</div>

					<div className="flex gap-2">
						<Select
							value={sortBy}
							onValueChange={(val) => {
								setSortBy(val as SortOption);
								setOffset(0);
							}}
						>
							<SelectTrigger className="w-[150px]">
								<SlidersHorizontal className="mr-2 h-4 w-4" />
								<SelectValue placeholder="Sort by" />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="downloads">Most Downloads</SelectItem>
								<SelectItem value="relevance">Relevance</SelectItem>
								<SelectItem value="name">Name</SelectItem>
								<SelectItem value="updated_at">Recently Updated</SelectItem>
								<SelectItem value="created_at">Newest</SelectItem>
							</SelectContent>
						</Select>

						<Button
							variant={verifiedOnly ? "default" : "outline"}
							onClick={() => {
								setVerifiedOnly(!verifiedOnly);
								setOffset(0);
							}}
							className="gap-2"
						>
							<Shield className="h-4 w-4" />
							Verified
						</Button>
					</div>
				</div>

				{/* Results count */}
				{searchResults.data && (
					<p className="text-sm text-muted-foreground">
						{searchResults.data.totalCount.toLocaleString()} packages found
					</p>
				)}

				{/* Package Grid */}
				{searchResults.isLoading ? (
					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
						{Array.from({ length: 8 }).map((_, i) => (
							<PackageCardSkeleton key={i} />
						))}
					</div>
				) : searchResults.data?.packages.length === 0 ? (
					<Card className="p-12 text-center">
						<Package className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
						<h3 className="text-lg font-semibold">No packages found</h3>
						<p className="text-muted-foreground mt-1">
							Try adjusting your search or filters
						</p>
					</Card>
				) : (
					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
						{searchResults.data?.packages.map((pkg) => (
							<PackageCard key={pkg.id} pkg={pkg} />
						))}
					</div>
				)}

				{/* Pagination */}
				{totalPages > 1 && (
					<div className="flex items-center justify-center gap-2">
						<Button
							variant="outline"
							onClick={() => setOffset(Math.max(0, offset - limit))}
							disabled={offset === 0}
						>
							Previous
						</Button>
						<span className="text-sm text-muted-foreground">
							Page {currentPage} of {totalPages}
						</span>
						<Button
							variant="outline"
							onClick={() => setOffset(offset + limit)}
							disabled={currentPage >= totalPages}
						>
							Next
						</Button>
					</div>
				)}
			</div>
		</main>
	);
}

function PageContent() {
	const searchParams = useSearchParams();
	const packageId = searchParams.get("id");

	if (packageId) {
		return (
			<Suspense fallback={<Skeleton className="h-full w-full" />}>
				<PackageDetailWrapper />
			</Suspense>
		);
	}

	return <PackageListContent />;
}

export default function PackagesStorePage() {
	return (
		<Suspense fallback={<Skeleton className="h-full w-full" />}>
			<PageContent />
		</Suspense>
	);
}
