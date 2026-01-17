"use client";

import { formatDistanceToNow } from "date-fns";
import {
	ArrowLeft,
	Check,
	Download,
	ExternalLink,
	FileCode,
	Github,
	Globe,
	Package,
	RefreshCw,
	Shield,
	Tag,
	User,
} from "lucide-react";
import type { RegistryEntry } from "../../lib/schema/wasm";
import {
	Badge,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Skeleton,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../ui";

function PermissionBadge({
	label,
	enabled,
}: { label: string; enabled: boolean }) {
	return (
		<Badge variant={enabled ? "default" : "outline"} className="gap-1">
			{enabled ? <Check className="h-3 w-3" /> : null}
			{label}
		</Badge>
	);
}

function NodeCard({
	node,
}: {
	node: { id: string; name: string; description: string; category: string };
}) {
	return (
		<Card>
			<CardHeader className="pb-2">
				<div className="flex items-start justify-between">
					<CardTitle className="text-sm font-medium">{node.name}</CardTitle>
					<Badge variant="outline" className="text-xs">
						{node.category}
					</Badge>
				</div>
			</CardHeader>
			<CardContent>
				<p className="text-xs text-muted-foreground">{node.description}</p>
			</CardContent>
		</Card>
	);
}

function VersionRow({
	version,
	isLatest,
}: {
	version: {
		version: string;
		publishedAt: string;
		releaseNotes?: string;
		yanked: boolean;
	};
	isLatest: boolean;
}) {
	return (
		<div className="flex items-center justify-between py-2 border-b last:border-0">
			<div className="flex items-center gap-2">
				<code className="text-sm font-mono">{version.version}</code>
				{isLatest && <Badge variant="secondary">Latest</Badge>}
				{version.yanked && <Badge variant="destructive">Yanked</Badge>}
			</div>
			<span className="text-sm text-muted-foreground">
				{formatDistanceToNow(new Date(version.publishedAt), {
					addSuffix: true,
				})}
			</span>
		</div>
	);
}

export interface PackageDetailViewProps {
	pkg: RegistryEntry | null | undefined;
	isLoading: boolean;
	installedVersion: string | null | undefined;
	onBack: () => void;
	onInstall: (version?: string) => void;
	onUninstall: () => void;
	isInstalling?: boolean;
	isUninstalling?: boolean;
}

export function PackageDetailView({
	pkg,
	isLoading,
	installedVersion,
	onBack,
	onInstall,
	onUninstall,
	isInstalling,
	isUninstalling,
}: PackageDetailViewProps) {
	if (isLoading || !pkg) {
		return (
			<main className="flex-col flex grow max-h-full p-6 overflow-auto min-h-0 w-full">
				<div className="mx-auto w-full max-w-5xl space-y-6">
					<div className="flex items-center gap-4">
						<Skeleton className="h-9 w-24" />
					</div>
					<Skeleton className="h-32 w-full" />
					<Skeleton className="h-64 w-full" />
				</div>
			</main>
		);
	}

	const manifest = pkg.manifest;
	const latestVersion =
		pkg.versions.find((v) => !v.yanked)?.version ?? pkg.versions[0]?.version;
	const isInstalled = !!installedVersion;
	const hasUpdate = isInstalled && installedVersion !== latestVersion;

	return (
		<main className="flex-col flex grow max-h-full p-6 overflow-auto min-h-0 w-full">
			<div className="mx-auto w-full max-w-5xl space-y-6">
				{/* Back Button */}
				<Button variant="ghost" onClick={onBack} className="gap-2">
					<ArrowLeft className="h-4 w-4" />
					Back
				</Button>

				{/* Header Card */}
				<Card>
					<CardHeader>
						<div className="flex flex-col md:flex-row md:items-start md:justify-between gap-4">
							<div className="flex items-start gap-4">
								<div className="p-3 rounded-lg bg-muted">
									<Package className="h-8 w-8" />
								</div>
								<div>
									<div className="flex items-center gap-2 flex-wrap">
										<CardTitle className="text-2xl">{manifest.name}</CardTitle>
										{pkg.verified && (
											<Badge variant="secondary" className="gap-1">
												<Shield className="h-3 w-3" />
												Verified
											</Badge>
										)}
									</div>
									<CardDescription className="mt-1">
										{manifest.description}
									</CardDescription>
									<div className="flex items-center gap-4 mt-2 text-sm text-muted-foreground">
										<span className="flex items-center gap-1">
											<Tag className="h-4 w-4" />v{latestVersion}
										</span>
										<span className="flex items-center gap-1">
											<Download className="h-4 w-4" />
											{pkg.downloadCount.toLocaleString()} downloads
										</span>
									</div>
								</div>
							</div>

							<div className="flex flex-col gap-2">
								{isInstalled ? (
									<>
										<div className="flex items-center gap-2 text-sm text-muted-foreground">
											<Check className="h-4 w-4 text-green-500" />
											Installed v{installedVersion}
										</div>
										{hasUpdate && (
											<Button
												onClick={() => onInstall(latestVersion)}
												disabled={isInstalling}
											>
												{isInstalling ? (
													<RefreshCw className="mr-2 h-4 w-4 animate-spin" />
												) : (
													<RefreshCw className="mr-2 h-4 w-4" />
												)}
												Update to v{latestVersion}
											</Button>
										)}
										<Button
											variant="destructive"
											onClick={onUninstall}
											disabled={isUninstalling}
										>
											Uninstall
										</Button>
									</>
								) : (
									<Button
										onClick={() => onInstall(undefined)}
										disabled={isInstalling}
									>
										{isInstalling ? (
											<RefreshCw className="mr-2 h-4 w-4 animate-spin" />
										) : (
											<Download className="mr-2 h-4 w-4" />
										)}
										Install
									</Button>
								)}
							</div>
						</div>
					</CardHeader>
				</Card>

				{/* Main Content */}
				<Tabs defaultValue="overview" className="w-full">
					<TabsList>
						<TabsTrigger value="overview">Overview</TabsTrigger>
						<TabsTrigger value="nodes">
							Nodes ({manifest.nodes.length})
						</TabsTrigger>
						<TabsTrigger value="permissions">Permissions</TabsTrigger>
						<TabsTrigger value="versions">
							Versions ({pkg.versions.length})
						</TabsTrigger>
					</TabsList>

					<TabsContent value="overview" className="space-y-4">
						<div className="grid grid-cols-1 md:grid-cols-3 gap-4">
							{/* Info Card */}
							<Card className="md:col-span-2">
								<CardHeader>
									<CardTitle className="text-base">
										Package Information
									</CardTitle>
								</CardHeader>
								<CardContent className="space-y-4">
									{manifest.keywords.length > 0 && (
										<div>
											<h4 className="text-sm font-medium mb-2">Keywords</h4>
											<div className="flex flex-wrap gap-1">
												{manifest.keywords.map((kw) => (
													<Badge key={kw} variant="outline">
														{kw}
													</Badge>
												))}
											</div>
										</div>
									)}

									{manifest.authors.length > 0 && (
										<div>
											<h4 className="text-sm font-medium mb-2">Authors</h4>
											<div className="flex flex-wrap gap-2">
												{manifest.authors.map((author, idx) => (
													<div
														key={idx}
														className="flex items-center gap-1 text-sm"
													>
														<User className="h-4 w-4 text-muted-foreground" />
														{author.url ? (
															<a
																href={author.url}
																target="_blank"
																rel="noopener noreferrer"
																className="hover:underline"
															>
																{author.name}
															</a>
														) : (
															<span>{author.name}</span>
														)}
													</div>
												))}
											</div>
										</div>
									)}

									{manifest.license && (
										<div>
											<h4 className="text-sm font-medium mb-2">License</h4>
											<Badge variant="outline">{manifest.license}</Badge>
										</div>
									)}
								</CardContent>
							</Card>

							{/* Links Card */}
							<Card>
								<CardHeader>
									<CardTitle className="text-base">Links</CardTitle>
								</CardHeader>
								<CardContent className="space-y-3">
									{manifest.repository && (
										<a
											href={manifest.repository}
											target="_blank"
											rel="noopener noreferrer"
											className="flex items-center gap-2 text-sm hover:underline"
										>
											<Github className="h-4 w-4" />
											Repository
											<ExternalLink className="h-3 w-3" />
										</a>
									)}
									{manifest.homepage && (
										<a
											href={manifest.homepage}
											target="_blank"
											rel="noopener noreferrer"
											className="flex items-center gap-2 text-sm hover:underline"
										>
											<Globe className="h-4 w-4" />
											Homepage
											<ExternalLink className="h-3 w-3" />
										</a>
									)}
									{!manifest.repository && !manifest.homepage && (
										<p className="text-sm text-muted-foreground">
											No external links provided
										</p>
									)}
								</CardContent>
							</Card>
						</div>

						{/* Stats Card */}
						<Card>
							<CardHeader>
								<CardTitle className="text-base">Statistics</CardTitle>
							</CardHeader>
							<CardContent>
								<div className="grid grid-cols-2 md:grid-cols-4 gap-4">
									<div>
										<p className="text-2xl font-bold">
											{pkg.downloadCount.toLocaleString()}
										</p>
										<p className="text-sm text-muted-foreground">
											Total Downloads
										</p>
									</div>
									<div>
										<p className="text-2xl font-bold">{pkg.versions.length}</p>
										<p className="text-sm text-muted-foreground">Versions</p>
									</div>
									<div>
										<p className="text-2xl font-bold">
											{manifest.nodes.length}
										</p>
										<p className="text-sm text-muted-foreground">Nodes</p>
									</div>
									<div>
										<p className="text-sm text-muted-foreground">Created</p>
										<p className="text-sm">
											{formatDistanceToNow(new Date(pkg.createdAt), {
												addSuffix: true,
											})}
										</p>
									</div>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="nodes" className="space-y-4">
						{manifest.nodes.length === 0 ? (
							<Card className="p-8 text-center">
								<FileCode className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
								<h3 className="font-semibold">No nodes declared</h3>
								<p className="text-muted-foreground text-sm">
									This package doesn&apos;t declare any nodes in its manifest
								</p>
							</Card>
						) : (
							<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
								{manifest.nodes.map((node) => (
									<NodeCard key={node.id} node={node} />
								))}
							</div>
						)}
					</TabsContent>

					<TabsContent value="permissions" className="space-y-4">
						<Card>
							<CardHeader>
								<CardTitle className="text-base">Resource Limits</CardTitle>
							</CardHeader>
							<CardContent className="space-y-4">
								<div className="grid grid-cols-2 gap-4">
									<div>
										<p className="text-sm font-medium">Memory</p>
										<Badge variant="outline" className="mt-1">
											{manifest.permissions.memory}
										</Badge>
									</div>
									<div>
										<p className="text-sm font-medium">Timeout</p>
										<Badge variant="outline" className="mt-1">
											{manifest.permissions.timeout}
										</Badge>
									</div>
								</div>
							</CardContent>
						</Card>

						<Card>
							<CardHeader>
								<CardTitle className="text-base">Capabilities</CardTitle>
							</CardHeader>
							<CardContent>
								<div className="flex flex-wrap gap-2">
									<PermissionBadge
										label="HTTP Requests"
										enabled={manifest.permissions.network.httpEnabled}
									/>
									<PermissionBadge
										label="WebSocket"
										enabled={manifest.permissions.network.websocketEnabled}
									/>
									<PermissionBadge
										label="Node Storage"
										enabled={manifest.permissions.filesystem.nodeStorage}
									/>
									<PermissionBadge
										label="User Storage"
										enabled={manifest.permissions.filesystem.userStorage}
									/>
									<PermissionBadge
										label="Variables"
										enabled={manifest.permissions.variables}
									/>
									<PermissionBadge
										label="Cache"
										enabled={manifest.permissions.cache}
									/>
									<PermissionBadge
										label="Streaming"
										enabled={manifest.permissions.streaming}
									/>
									<PermissionBadge
										label="A2UI"
										enabled={manifest.permissions.a2ui}
									/>
									<PermissionBadge
										label="Models/LLM"
										enabled={manifest.permissions.models}
									/>
								</div>

								{manifest.permissions.network.httpEnabled &&
									manifest.permissions.network.allowedHosts.length > 0 && (
										<div className="mt-4">
											<p className="text-sm font-medium mb-2">Allowed Hosts</p>
											<div className="flex flex-wrap gap-1">
												{manifest.permissions.network.allowedHosts.map(
													(host) => (
														<Badge
															key={host}
															variant="outline"
															className="font-mono text-xs"
														>
															{host}
														</Badge>
													),
												)}
											</div>
										</div>
									)}

								{manifest.permissions.oauthScopes.length > 0 && (
									<div className="mt-4">
										<p className="text-sm font-medium mb-2">OAuth Scopes</p>
										{manifest.permissions.oauthScopes.map((oauth, idx) => (
											<div key={idx} className="p-3 rounded-lg bg-muted mt-2">
												<div className="flex items-center gap-2">
													<Badge>{oauth.provider}</Badge>
													{oauth.required && (
														<Badge variant="destructive">Required</Badge>
													)}
												</div>
												<p className="text-sm mt-1">{oauth.reason}</p>
												<div className="flex flex-wrap gap-1 mt-2">
													{oauth.scopes.map((scope) => (
														<Badge
															key={scope}
															variant="outline"
															className="font-mono text-xs"
														>
															{scope}
														</Badge>
													))}
												</div>
											</div>
										))}
									</div>
								)}
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="versions" className="space-y-4">
						<Card>
							<CardHeader>
								<CardTitle className="text-base">Version History</CardTitle>
							</CardHeader>
							<CardContent>
								{pkg.versions.length === 0 ? (
									<p className="text-sm text-muted-foreground">
										No versions available
									</p>
								) : (
									<div className="divide-y">
										{pkg.versions.map((v, idx) => (
											<VersionRow
												key={v.version}
												version={v}
												isLatest={idx === 0}
											/>
										))}
									</div>
								)}
							</CardContent>
						</Card>
					</TabsContent>
				</Tabs>
			</div>
		</main>
	);
}
