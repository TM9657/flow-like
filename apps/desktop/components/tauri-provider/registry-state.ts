import { invoke } from "@tauri-apps/api/core";
import type {
	CachedPackage,
	InstalledPackage,
	PackageUpdate,
	SearchFilters,
	SearchResults,
} from "@tm9657/flow-like-ui/lib/schema/wasm";
import type { IRegistryState } from "@tm9657/flow-like-ui/state/backend-state/registry-state";
import type { TauriBackend } from "../tauri-provider";

export class RegistryState implements IRegistryState {
	constructor(private readonly backend: TauriBackend) {}

	async init(registryUrl?: string): Promise<void> {
		const config = registryUrl ? { registry_url: registryUrl } : null;
		return invoke("registry_init", { config });
	}

	async searchPackages(filters?: SearchFilters): Promise<SearchResults> {
		return invoke("registry_search_packages", { filters: filters ?? {} });
	}

	async getPackage(packageId: string): Promise<InstalledPackage | null> {
		return invoke("registry_get_package", { packageId });
	}

	async installPackage(
		packageId: string,
		version?: string,
	): Promise<CachedPackage> {
		return invoke("registry_install_package", { packageId, version });
	}

	async uninstallPackage(packageId: string): Promise<void> {
		return invoke("registry_uninstall_package", { packageId });
	}

	async getInstalledPackages(): Promise<InstalledPackage[]> {
		return invoke("registry_get_installed_packages");
	}

	async isPackageInstalled(packageId: string): Promise<boolean> {
		return invoke("registry_is_package_installed", { packageId });
	}

	async getInstalledVersion(packageId: string): Promise<string | null> {
		return invoke("registry_get_installed_version", { packageId });
	}

	async updatePackage(
		packageId: string,
		version?: string,
	): Promise<CachedPackage> {
		return invoke("registry_update_package", { packageId, version });
	}

	async checkForUpdates(): Promise<PackageUpdate[]> {
		return invoke("registry_check_for_updates");
	}
}
