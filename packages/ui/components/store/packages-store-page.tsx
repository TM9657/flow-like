"use client";

import { useQuery } from "@tanstack/react-query";
import { useDebounce } from "@uidotdev/usehooks";
import {
	Download,
	Package,
	Search,
	Shield,
	SlidersHorizontal,
} from "lucide-react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useCallback, useState } from "react";
import type { AuthContextProps } from "react-oidc-context";
import { toast } from "sonner";
import { useInvoke } from "../../hooks/use-invoke";
import type {
	PackageSummary,
	SearchFilters,
	SearchResults,
} from "../../lib/schema/wasm";
import { useBackend } from "../../state/backend-state";
import { Input } from "../ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import { Skeleton } from "../ui/skeleton";
import {
	StorePackageDetail,
	type GenericFetcher,
} from "../pages/store/store-package-detail";
import type { CompileStatus } from "../ui/package-status-badge";

type SortOption =
	| "relevance"
	| "name"
	| "downloads"
	| "updated_at"
	| "created_at";

interface PackagesStorePageProps {
	fetcher: GenericFetcher;
	auth: AuthContextProps;
	getPackageStatus?: (packageId: string) => CompileStatus | undefined;
}

function PackageCard({ pkg }: { pkg: PackageSummary }) {
	return (
		<Link
			href={`/store/packages?id=${pkg.id}`}
			className="group block rounded-2xl bg-card/60 border border-border/20 p-5 transition-all hover:bg-card/80 hover:border-border/40 hover:shadow-sm"
		>
			<div className="flex items-start justify-between mb-2">
				<div className="flex items-center gap-2">
					<Package className="h-4 w-4 text-muted-foreground/50" />
					<h3 className="text-sm font-semibold group-hover:text-primary transition-colors">
						{pkg.name}
					</h3>
				</div>
				{pkg.verified && (
					<span className="flex items-center gap-1 text-xs text-muted-foreground">
						<Shield className="h-3 w-3" />
						Verified
					</span>
				)}
			</div>
			<p className="text-xs text-muted-foreground line-clamp-2 mb-3">
				{pkg.description}
			</p>
			<div className="flex flex-wrap gap-1.5 mb-3">
				{pkg.keywords.slice(0, 3).map((kw) => (
					<span
						key={kw}
						className="rounded-full bg-muted/40 px-2 py-0.5 text-[10px] capitalize"
					>
						{kw}
					</span>
				))}
				{pkg.keywords.length > 3 && (
					<span className="rounded-full bg-muted/40 px-2 py-0.5 text-[10px]">
						+{pkg.keywords.length - 3}
					</span>
				)}
			</div>
			<div className="flex items-center gap-4 text-xs text-muted-foreground/60">
				<span className="flex items-center gap-1">
					<Download className="h-3 w-3" />
					{pkg.downloadCount.toLocaleString()}
				</span>
				<span>v{pkg.latestVersion}</span>
			</div>
		</Link>
	);
}

function PackageCardSkeleton() {
	return (
		<div className="rounded-2xl bg-card/60 border border-border/20 p-5 space-y-3">
			<div className="flex items-start justify-between">
				<Skeleton className="h-4 w-32 rounded-full" />
				<Skeleton className="h-4 w-14 rounded-full" />
			</div>
			<Skeleton className="h-8 w-full rounded-lg" />
			<div className="flex gap-1.5">
				<Skeleton className="h-4 w-12 rounded-full" />
				<Skeleton className="h-4 w-16 rounded-full" />
				<Skeleton className="h-4 w-10 rounded-full" />
			</div>
			<Skeleton className="h-3 w-24 rounded-full" />
		</div>
	);
}

function PackageDetailWrapper({
	fetcher,
	auth,
	getPackageStatus,
}: {
	fetcher: GenericFetcher;
	auth: AuthContextProps;
	getPackageStatus?: (packageId: string) => CompileStatus | undefined;
}) {
	const searchParams = useSearchParams();
	const router = useRouter();
	const packageId = searchParams.get("id") ?? "";

	const handleBack = useCallback(() => router.back(), [router]);
	const compileStatus = getPackageStatus?.(packageId);

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
			compileStatus={compileStatus}
		/>
	);
}

function PackageListContent({
	fetcher,
	auth,
}: { fetcher: GenericFetcher; auth: AuthContextProps }) {
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
			<div className="mx-auto w-full max-w-7xl space-y-8">
				<div className="space-y-2">
					<h1 className="text-2xl font-semibold tracking-tight">Packages</h1>
					<p className="text-sm text-muted-foreground">
						Discover and install WASM node packages
					</p>
				</div>

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
							className="rounded-full bg-muted/30 border-border/20 pl-10"
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

						<button
							type="button"
							onClick={() => {
								setVerifiedOnly(!verifiedOnly);
								setOffset(0);
							}}
							className={`rounded-full text-sm border gap-2 px-4 py-2 flex items-center transition-colors ${
								verifiedOnly
									? "bg-primary text-primary-foreground border-primary"
									: "bg-transparent text-muted-foreground border-border/30 hover:bg-muted/30"
							}`}
						>
							<Shield className="h-4 w-4" />
							Verified
						</button>
					</div>
				</div>

				{searchResults.data && (
					<p className="text-xs text-muted-foreground/60">
						{searchResults.data.totalCount.toLocaleString()} packages found
					</p>
				)}

				{searchResults.isLoading ? (
					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
						{Array.from({ length: 8 }).map((_, i) => (
							<PackageCardSkeleton key={i} />
						))}
					</div>
				) : searchResults.data?.packages.length === 0 ? (
					<div className="flex flex-col items-center justify-center py-20 text-center">
						<Package className="w-12 h-12 text-muted-foreground/30 mb-3" />
						<h3 className="text-lg font-semibold">No packages found</h3>
						<p className="text-sm text-muted-foreground mt-1">
							Try adjusting your search or filters
						</p>
					</div>
				) : (
					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
						{searchResults.data?.packages.map((pkg) => (
							<PackageCard key={pkg.id} pkg={pkg} />
						))}
					</div>
				)}

				{totalPages > 1 && (
					<div className="flex items-center justify-center gap-3">
						<button
							type="button"
							onClick={() => setOffset(Math.max(0, offset - limit))}
							disabled={offset === 0}
							className="rounded-full text-sm text-muted-foreground/60 border border-border/30 hover:bg-muted/30 px-5 py-2 transition-colors disabled:opacity-40"
						>
							Previous
						</button>
						<span className="text-xs text-muted-foreground/60">
							{currentPage} / {totalPages}
						</span>
						<button
							type="button"
							onClick={() => setOffset(offset + limit)}
							disabled={currentPage >= totalPages}
							className="rounded-full text-sm text-muted-foreground/60 border border-border/30 hover:bg-muted/30 px-5 py-2 transition-colors disabled:opacity-40"
						>
							Next
						</button>
					</div>
				)}
			</div>
		</main>
	);
}

function PageContent({ fetcher, auth, getPackageStatus }: PackagesStorePageProps) {
	const searchParams = useSearchParams();
	const packageId = searchParams.get("id");

	if (packageId) {
		return (
			<Suspense fallback={<Skeleton className="h-full w-full" />}>
				<PackageDetailWrapper
					fetcher={fetcher}
					auth={auth}
					getPackageStatus={getPackageStatus}
				/>
			</Suspense>
		);
	}

	return <PackageListContent fetcher={fetcher} auth={auth} />;
}

export function PackagesStorePage({
	fetcher,
	auth,
	getPackageStatus,
}: PackagesStorePageProps) {
	return (
		<Suspense fallback={<Skeleton className="h-full w-full" />}>
			<PageContent
				fetcher={fetcher}
				auth={auth}
				getPackageStatus={getPackageStatus}
			/>
		</Suspense>
	);
}
