import type { IRegistryState } from "@tm9657/flow-like-ui";
import type {
	CachedPackage,
	InstalledPackage,
	PackageUpdate,
	SearchFilters,
	SearchResults,
} from "@tm9657/flow-like-ui/lib/schema/wasm";
import { apiGet, apiPost, apiDelete, type WebBackendRef } from "./api-utils";

export class WebRegistryState implements IRegistryState {
	constructor(private readonly backend: WebBackendRef) {}

	async init(registryUrl?: string): Promise<void> {
		// In web mode, registry is managed server-side
	}

	async searchPackages(filters?: SearchFilters): Promise<SearchResults> {
		try {
			return await apiPost<SearchResults>(
				"registry/search",
				filters ?? {},
				this.backend.auth,
			);
		} catch {
			return { packages: [], totalCount: 0, offset: 0, limit: 20 };
		}
	}

	async getPackage(packageId: string): Promise<InstalledPackage | null> {
		try {
			return await apiGet<InstalledPackage>(
				`registry/packages/${packageId}`,
				this.backend.auth,
			);
		} catch {
			return null;
		}
	}

	async installPackage(packageId: string, version?: string): Promise<CachedPackage> {
		const params = version ? `?version=${version}` : "";
		return apiPost<CachedPackage>(
			`registry/packages/${packageId}/install${params}`,
			undefined,
			this.backend.auth,
		);
	}

	async uninstallPackage(packageId: string): Promise<void> {
		await apiDelete(
			`registry/packages/${packageId}`,
			this.backend.auth,
		);
	}

	async getInstalledPackages(): Promise<InstalledPackage[]> {
		try {
			return await apiGet<InstalledPackage[]>(
				"registry/installed",
				this.backend.auth,
			);
		} catch {
			return [];
		}
	}

	async isPackageInstalled(packageId: string): Promise<boolean> {
		try {
			const result = await apiGet<{ installed: boolean }>(
				`registry/packages/${packageId}/installed`,
				this.backend.auth,
			);
			return result?.installed ?? false;
		} catch {
			return false;
		}
	}

	async getInstalledVersion(packageId: string): Promise<string | null> {
		try {
			const result = await apiGet<{ version: string | null }>(
				`registry/packages/${packageId}/version`,
				this.backend.auth,
			);
			return result?.version ?? null;
		} catch {
			return null;
		}
	}

	async updatePackage(packageId: string, version?: string): Promise<CachedPackage> {
		const params = version ? `?version=${version}` : "";
		return apiPost<CachedPackage>(
			`registry/packages/${packageId}/update${params}`,
			undefined,
			this.backend.auth,
		);
	}

	async checkForUpdates(): Promise<PackageUpdate[]> {
		try {
			return await apiGet<PackageUpdate[]>(
				"registry/updates",
				this.backend.auth,
			);
		} catch {
			return [];
		}
	}
}
