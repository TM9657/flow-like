"use client";

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
	Badge,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Input,
	Skeleton,
	Tooltip,
	TooltipContent,
	TooltipTrigger,
	useBackend,
} from "@tm9657/flow-like-ui";
import type {
	InstalledPackage,
	PackageUpdate,
} from "@tm9657/flow-like-ui/lib/schema/wasm";
import { motion } from "framer-motion";
import {
	AlertTriangle,
	Check,
	FolderOpen,
	Loader2,
	Package,
	RefreshCw,
	Search,
	Trash2,
	Upload,
} from "lucide-react";
import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

function InstalledPackageCard({
	pkg,
	hasUpdate,
	latestVersion,
	onUninstall,
	onUpdate,
	isLoading,
}: {
	pkg: InstalledPackage;
	hasUpdate?: boolean;
	latestVersion?: string;
	onUninstall: (id: string) => void;
	onUpdate: (id: string) => void;
	isLoading: boolean;
}) {
	return (
		<motion.div
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			transition={{ duration: 0.2 }}
		>
			<Card className="hover:shadow-md transition-shadow">
				<CardHeader className="pb-2">
					<div className="flex items-start justify-between gap-2">
						<div className="flex items-center gap-2 min-w-0">
							<Package className="h-5 w-5 flex-shrink-0 text-muted-foreground" />
							<CardTitle className="text-base truncate">
								{pkg.manifest.name}
							</CardTitle>
						</div>
						<div className="flex items-center gap-1 flex-shrink-0">
							<Badge variant="outline">v{pkg.version}</Badge>
							{hasUpdate && (
								<Badge variant="default" className="gap-1">
									<AlertTriangle className="h-3 w-3" />
									Update
								</Badge>
							)}
						</div>
					</div>
					<CardDescription className="line-clamp-2">
						{pkg.manifest.description}
					</CardDescription>
				</CardHeader>
				<CardContent>
					<div className="flex flex-wrap gap-1 mb-3">
						{pkg.manifest.keywords.slice(0, 3).map((keyword) => (
							<Badge key={keyword} variant="outline" className="text-xs">
								{keyword}
							</Badge>
						))}
					</div>
					<div className="flex items-center justify-between">
						<span className="text-xs text-muted-foreground">
							Installed {new Date(pkg.installedAt).toLocaleDateString()}
						</span>
						<div className="flex items-center gap-2">
							{hasUpdate && (
								<Button
									size="sm"
									variant="secondary"
									onClick={() => onUpdate(pkg.id)}
									disabled={isLoading}
								>
									{isLoading ? (
										<Loader2 className="h-4 w-4 animate-spin" />
									) : (
										<>
											<RefreshCw className="h-4 w-4 mr-1" />
											Update to v{latestVersion}
										</>
									)}
								</Button>
							)}
							<Button
								size="sm"
								variant="destructive"
								onClick={() => onUninstall(pkg.id)}
								disabled={isLoading}
							>
								{isLoading ? (
									<Loader2 className="h-4 w-4 animate-spin" />
								) : (
									<Trash2 className="h-4 w-4" />
								)}
							</Button>
						</div>
					</div>
				</CardContent>
			</Card>
		</motion.div>
	);
}

function LocalPackageCard({
	pkg,
	onRemove,
	isLoading,
}: {
	pkg: InstalledPackage;
	onRemove: (id: string) => void;
	isLoading: boolean;
}) {
	return (
		<motion.div
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			transition={{ duration: 0.2 }}
		>
			<Card className="hover:shadow-md transition-shadow border-dashed border-primary/50">
				<CardHeader className="pb-2">
					<div className="flex items-start justify-between gap-2">
						<div className="flex items-center gap-2 min-w-0">
							<FolderOpen className="h-5 w-5 flex-shrink-0 text-primary" />
							<CardTitle className="text-base truncate">
								{pkg.manifest.name}
							</CardTitle>
						</div>
						<Badge variant="secondary" className="gap-1">
							<Check className="h-3 w-3" />
							Local
						</Badge>
					</div>
					<CardDescription className="line-clamp-2">
						{pkg.manifest.description}
					</CardDescription>
				</CardHeader>
				<CardContent>
					<div className="flex items-center justify-between">
						<span className="text-xs text-muted-foreground">
							v{pkg.version}
						</span>
						<Button
							size="sm"
							variant="destructive"
							onClick={() => onRemove(pkg.id)}
							disabled={isLoading}
						>
							{isLoading ? (
								<Loader2 className="h-4 w-4 animate-spin" />
							) : (
								<Trash2 className="h-4 w-4" />
							)}
						</Button>
					</div>
				</CardContent>
			</Card>
		</motion.div>
	);
}

function PackageCardSkeleton() {
	return (
		<Card>
			<CardHeader className="pb-2">
				<div className="flex items-start justify-between gap-2">
					<Skeleton className="h-5 w-32" />
					<Skeleton className="h-5 w-16" />
				</div>
				<Skeleton className="h-4 w-full mt-2" />
				<Skeleton className="h-4 w-3/4" />
			</CardHeader>
			<CardContent>
				<div className="flex gap-1 mb-3">
					<Skeleton className="h-5 w-12" />
					<Skeleton className="h-5 w-16" />
				</div>
				<div className="flex items-center justify-between">
					<Skeleton className="h-4 w-24" />
					<Skeleton className="h-8 w-20" />
				</div>
			</CardContent>
		</Card>
	);
}

export default function InstalledPackagesPage() {
	const backend = useBackend();
	const [isInitialized, setIsInitialized] = useState(false);
	const [isInitializing, setIsInitializing] = useState(false);
	const [searchQuery, setSearchQuery] = useState("");

	const [installedPackages, setInstalledPackages] = useState<
		InstalledPackage[]
	>([]);
	const [localPackages, setLocalPackages] = useState<InstalledPackage[]>([]);
	const [updates, setUpdates] = useState<PackageUpdate[]>([]);
	const [isLoading, setIsLoading] = useState(false);
	const [loadingPackage, setLoadingPackage] = useState<string | null>(null);
	const [isLoadingLocal, setIsLoadingLocal] = useState(false);

	const initRegistry = useCallback(async () => {
		if (!backend?.registryState || isInitialized || isInitializing) return;
		setIsInitializing(true);
		try {
			await backend.registryState.init();
			setIsInitialized(true);
		} catch (err) {
			console.error("Failed to initialize registry:", err);
		} finally {
			setIsInitializing(false);
		}
	}, [backend?.registryState, isInitialized, isInitializing]);

	useEffect(() => {
		initRegistry();
	}, [initRegistry]);

	const fetchInstalled = useCallback(async () => {
		if (!backend?.registryState || !isInitialized) return;
		setIsLoading(true);
		try {
			const [packages, updateList] = await Promise.all([
				backend.registryState.getInstalledPackages(),
				backend.registryState.checkForUpdates(),
			]);
			const registry = packages.filter((p) => !p.id.startsWith("local."));
			const local = packages.filter((p) => p.id.startsWith("local."));
			setInstalledPackages(registry);
			setLocalPackages(local);
			setUpdates(updateList);
		} catch (err) {
			console.error("Failed to fetch installed packages:", err);
		} finally {
			setIsLoading(false);
		}
	}, [backend?.registryState, isInitialized]);

	useEffect(() => {
		if (isInitialized) {
			fetchInstalled();
		}
	}, [isInitialized, fetchInstalled]);

	const handleLoadLocal = async () => {
		try {
			const selected = await open({
				multiple: false,
				filters: [{ name: "WASM Files", extensions: ["wasm"] }],
			});

			if (!selected) return;

			setIsLoadingLocal(true);
			await invoke("registry_load_local", { path: selected });

			const baseName =
				selected.split(/[/\\]/).pop()?.replace(".wasm", "") ?? "package";
			toast.success(`Loaded ${baseName}`, {
				description: "Package loaded for development testing",
			});
			await fetchInstalled();
		} catch (err) {
			console.error("Failed to load local package:", err);
			toast.error(`Failed to load package: ${err}`);
		} finally {
			setIsLoadingLocal(false);
		}
	};

	const handleUninstall = async (packageId: string) => {
		if (!backend?.registryState) return;
		setLoadingPackage(packageId);
		try {
			await backend.registryState.uninstallPackage(packageId);
			toast.success("Package uninstalled");
			await fetchInstalled();
		} catch (err) {
			console.error("Failed to uninstall package:", err);
			toast.error(`Failed to uninstall: ${err}`);
		} finally {
			setLoadingPackage(null);
		}
	};

	const handleUpdate = async (packageId: string) => {
		if (!backend?.registryState) return;
		setLoadingPackage(packageId);
		try {
			await backend.registryState.updatePackage(packageId);
			toast.success("Package updated");
			await fetchInstalled();
		} catch (err) {
			console.error("Failed to update package:", err);
			toast.error(`Failed to update: ${err}`);
		} finally {
			setLoadingPackage(null);
		}
	};

	const updateMap = new Map(updates.map((u) => [u.packageId, u.latestVersion]));

	const filteredPackages = installedPackages.filter((pkg) => {
		if (!searchQuery) return true;
		const query = searchQuery.toLowerCase();
		return (
			pkg.manifest.name.toLowerCase().includes(query) ||
			pkg.manifest.description.toLowerCase().includes(query)
		);
	});

	const filteredLocalPackages = localPackages.filter((pkg) => {
		if (!searchQuery) return true;
		const query = searchQuery.toLowerCase();
		return (
			pkg.manifest.name.toLowerCase().includes(query) ||
			pkg.manifest.description.toLowerCase().includes(query)
		);
	});

	if (isInitializing) {
		return (
			<div className="flex items-center justify-center h-full">
				<div className="text-center">
					<Loader2 className="h-8 w-8 animate-spin mx-auto mb-4" />
					<p className="text-muted-foreground">Initializing registry...</p>
				</div>
			</div>
		);
	}

	return (
		<div className="flex flex-col h-full space-y-4">
			<div className="flex items-center justify-between">
				<div>
					<h1 className="text-2xl font-bold">Installed Packages</h1>
					<p className="text-sm text-muted-foreground">
						Manage your installed WASM node packages
					</p>
				</div>
				<div className="flex items-center gap-2">
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="outline"
								size="sm"
								onClick={handleLoadLocal}
								disabled={isLoadingLocal}
							>
								{isLoadingLocal ? (
									<Loader2 className="h-4 w-4 mr-1 animate-spin" />
								) : (
									<Upload className="h-4 w-4 mr-1" />
								)}
								Load Local
							</Button>
						</TooltipTrigger>
						<TooltipContent side="bottom" className="max-w-xs">
							<p className="text-sm">
								Select a <code>.wasm</code> file. If a <code>.toml</code>{" "}
								manifest exists with the same name, it will be used for package
								metadata.
							</p>
						</TooltipContent>
					</Tooltip>
					<Button
						variant="outline"
						size="sm"
						onClick={fetchInstalled}
						disabled={isLoading}
					>
						<RefreshCw
							className={`h-4 w-4 mr-1 ${isLoading ? "animate-spin" : ""}`}
						/>
						Refresh
					</Button>
				</div>
			</div>

			<div className="relative max-w-sm">
				<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
				<Input
					placeholder="Search installed packages..."
					value={searchQuery}
					onChange={(e) => setSearchQuery(e.target.value)}
					className="pl-9"
				/>
			</div>

			{updates.length > 0 && (
				<div className="flex items-center gap-2 p-3 bg-yellow-500/10 border border-yellow-500/20 rounded-md">
					<AlertTriangle className="h-4 w-4 text-yellow-500" />
					<span className="text-sm">
						{updates.length} update{updates.length > 1 ? "s" : ""} available
					</span>
				</div>
			)}

			<div className="flex-1 overflow-y-auto space-y-6">
				{/* Local Packages Section */}
				{filteredLocalPackages.length > 0 && (
					<div className="space-y-3">
						<h2 className="text-lg font-semibold flex items-center gap-2">
							<FolderOpen className="h-5 w-5" />
							Local Development Packages
						</h2>
						<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
							{filteredLocalPackages.map((pkg) => (
								<LocalPackageCard
									key={pkg.id}
									pkg={pkg}
									onRemove={handleUninstall}
									isLoading={loadingPackage === pkg.id}
								/>
							))}
						</div>
					</div>
				)}

				{/* Registry Packages Section */}
				<div className="space-y-3">
					{localPackages.length > 0 && (
						<h2 className="text-lg font-semibold flex items-center gap-2">
							<Package className="h-5 w-5" />
							Registry Packages
						</h2>
					)}
					{isLoading ? (
						<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
							{Array.from({ length: 6 }).map((_, i) => (
								<PackageCardSkeleton key={i} />
							))}
						</div>
					) : filteredPackages.length > 0 ? (
						<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
							{filteredPackages.map((pkg) => (
								<InstalledPackageCard
									key={pkg.id}
									pkg={pkg}
									hasUpdate={updateMap.has(pkg.id)}
									latestVersion={updateMap.get(pkg.id)}
									onUninstall={handleUninstall}
									onUpdate={handleUpdate}
									isLoading={loadingPackage === pkg.id}
								/>
							))}
						</div>
					) : localPackages.length === 0 ? (
						<div className="flex flex-col items-center justify-center py-12 text-center">
							<Package className="h-12 w-12 text-muted-foreground mb-4" />
							<p className="text-muted-foreground">No packages installed</p>
							<p className="text-sm text-muted-foreground mt-2">
								Browse the registry to find and install custom nodes, or load a
								local package for testing
							</p>
							<div className="flex gap-2 mt-4">
								<Link href="/settings/registry/explore">
									<Button variant="outline">Browse Packages</Button>
								</Link>
								<Button variant="secondary" onClick={handleLoadLocal}>
									<Upload className="h-4 w-4 mr-1" />
									Load Local
								</Button>
							</div>
						</div>
					) : null}
				</div>
			</div>
		</div>
	);
}
