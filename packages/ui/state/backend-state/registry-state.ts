import type {
	CachedPackage,
	InstalledPackage,
	PackageUpdate,
	SearchFilters,
	SearchResults,
} from "../../lib/schema/wasm";

export interface IRegistryState {
	init(registryUrl?: string): Promise<void>;
	searchPackages(filters?: SearchFilters): Promise<SearchResults>;
	getPackage(packageId: string): Promise<InstalledPackage | null>;
	installPackage(packageId: string, version?: string): Promise<CachedPackage>;
	uninstallPackage(packageId: string): Promise<void>;
	getInstalledPackages(): Promise<InstalledPackage[]>;
	isPackageInstalled(packageId: string): Promise<boolean>;
	getInstalledVersion(packageId: string): Promise<string | null>;
	updatePackage(packageId: string, version?: string): Promise<CachedPackage>;
	checkForUpdates(): Promise<PackageUpdate[]>;
}
