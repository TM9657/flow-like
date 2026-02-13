export interface PackageAuthor {
	name: string;
	email?: string;
	url?: string;
}

export interface NetworkPermissions {
	httpEnabled: boolean;
	allowedHosts: string[];
	websocketEnabled: boolean;
}

export interface FileSystemPermissions {
	nodeStorage: boolean;
	userStorage: boolean;
	uploadDir: boolean;
	cacheDir: boolean;
}

export interface OAuthScopeRequirement {
	provider: string;
	scopes: string[];
	reason: string;
	required: boolean;
}

export enum MemoryTier {
	Minimal = "minimal",
	Light = "light",
	Standard = "standard",
	Heavy = "heavy",
	Intensive = "intensive",
}

export enum TimeoutTier {
	Quick = "quick",
	Standard = "standard",
	Extended = "extended",
	LongRunning = "long_running",
}

export interface PackagePermissions {
	memory: MemoryTier;
	timeout: TimeoutTier;
	network: NetworkPermissions;
	filesystem: FileSystemPermissions;
	oauthScopes: OAuthScopeRequirement[];
	variables: boolean;
	cache: boolean;
	streaming: boolean;
	a2ui: boolean;
	models: boolean;
}

export interface PackageNodeEntry {
	id: string;
	name: string;
	description: string;
	category: string;
	icon?: string;
	oauthProviders: string[];
	metadata: Record<string, unknown>;
}

export interface PackageManifest {
	manifestVersion: number;
	id: string;
	name: string;
	version: string;
	description: string;
	authors: PackageAuthor[];
	license?: string;
	repository?: string;
	homepage?: string;
	permissions: PackagePermissions;
	nodes: PackageNodeEntry[];
	keywords: string[];
	minFlowLikeVersion?: string;
	wasmPath?: string;
	wasmHash?: string;
	metadata: Record<string, unknown>;
}

export enum PackageStatus {
	Active = "active",
	Deprecated = "deprecated",
	Yanked = "yanked",
}

export interface PackageSource {
	type: "local" | "remote";
	path?: string;
	registryUrl?: string;
	downloadUrl?: string;
}

export interface PackageVersion {
	version: string;
	wasmHash: string;
	wasmSize: number;
	downloadUrl?: string;
	publishedAt: string;
	minFlowLikeVersion?: string;
	releaseNotes?: string;
	yanked: boolean;
}

export interface RegistryEntry {
	id: string;
	manifest: PackageManifest;
	versions: PackageVersion[];
	status: PackageStatus;
	downloadCount: number;
	createdAt: string;
	updatedAt: string;
	source: PackageSource;
	verified: boolean;
}

export interface CachedPackage {
	entry: RegistryEntry;
	wasmData: number[];
	cachedAt: string;
	expiresAt?: string;
}

export interface PackageSummary {
	id: string;
	name: string;
	description: string;
	latestVersion: string;
	downloadCount: number;
	status: PackageStatus;
	keywords: string[];
	verified: boolean;
}

export interface SearchResults {
	packages: PackageSummary[];
	totalCount: number;
	offset: number;
	limit: number;
}

export interface InstalledPackage {
	id: string;
	version: string;
	source: PackageSource;
	installedAt: string;
	wasmPath: string;
	manifest: PackageManifest;
}

export interface SearchFilters {
	query?: string;
	category?: string;
	keywords?: string[];
	author?: string;
	verifiedOnly?: boolean;
	includeDeprecated?: boolean;
	sortBy?: "relevance" | "name" | "downloads" | "updated_at" | "created_at";
	sortDesc?: boolean;
	offset?: number;
	limit?: number;
}

export interface PackageUpdate {
	packageId: string;
	currentVersion: string;
	latestVersion: string;
}

// Admin types for package management

export type PackageAdminStatus =
	| "pending_review"
	| "active"
	| "rejected"
	| "deprecated"
	| "disabled";

export interface PackageDetails {
	id: string;
	name: string;
	description: string;
	version: string;
	authors: string[];
	license?: string;
	homepage?: string;
	repository?: string;
	keywords: string[];
	status: PackageAdminStatus;
	verified: boolean;
	downloadCount: number;
	wasmSize: number;
	nodes: PackageNodeEntry[];
	permissions: PackagePermissions;
	createdAt: string;
	updatedAt: string;
	publishedAt?: string;
	submitterId?: string;
}

export type ReviewAction =
	| "submitted"
	| "approve"
	| "reject"
	| "request_changes"
	| "comment"
	| "flag";

export interface PackageReview {
	id: string;
	packageId: string;
	reviewerId: string;
	action: ReviewAction;
	comment?: string;
	securityScore?: number;
	codeQualityScore?: number;
	documentationScore?: number;
	createdAt: string;
}

export interface ReviewRequest {
	action: "approve" | "reject" | "request_changes" | "comment" | "flag";
	comment?: string;
	internalNote?: string;
	securityScore?: number;
	codeQualityScore?: number;
	documentationScore?: number;
}

export interface RegistryStats {
	totalPackages: number;
	totalVersions: number;
	totalDownloads: number;
	pendingReview: number;
	activePackages: number;
	rejectedPackages: number;
	verifiedPackages: number;
}

export interface AdminPackageListResponse {
	packages: PackageDetails[];
	totalCount: number;
	offset: number;
	limit: number;
}

export interface AdminPackageDetailResponse {
	package: PackageDetails;
	reviews: PackageReview[];
}
