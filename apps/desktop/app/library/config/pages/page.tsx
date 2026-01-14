"use client";
import {
	Badge,
	Button,
	Card,
	CardContent,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	EmptyState,
	Input,
	Label,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
	useBackend,
	useInvoke,
} from "@tm9657/flow-like-ui";
import type { IOAuthProvider, IStoredOAuthToken } from "@tm9657/flow-like-ui";
import type { PageListItem } from "@tm9657/flow-like-ui/state/backend-state/page-state";
import EventsPage from "@tm9657/flow-like-ui/components/settings/events/events-page";
import { cn } from "@tm9657/flow-like-ui/lib/utils";
import type {
	CreateAppRoute,
	IAppRoute,
	RouteTargetType,
	UpdateAppRoute,
} from "@tm9657/flow-like-ui/state/backend-state/route-state";
import type { Version } from "@tm9657/flow-like-ui/state/backend-state/widget-state";
import {
	ArrowRight,
	FileText,
	Globe,
	LayoutGrid,
	Pencil,
	Plus,
	Route as RouteIcon,
	Sparkles,
	Trash2,
	Workflow,
} from "lucide-react";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useState } from "react";
import { EVENT_CONFIG } from "../../../../lib/event-config";
import { oauthConsentStore, oauthTokenStore } from "../../../../lib/oauth-db";
import { oauthService } from "../../../../lib/oauth-service";

export default function Page() {
	const searchParams = useSearchParams();
	const router = useRouter();
	const backend = useBackend();
	const id = searchParams.get("id");

	const pages = useInvoke(
		backend.pageState.getPages,
		backend.pageState,
		[id ?? ""],
		!!id,
		[id],
	);

	const routes = useInvoke(
		backend.routeState.getRoutes,
		backend.routeState,
		[id ?? ""],
		!!id,
		[id],
	);

	const events = useInvoke(
		backend.eventState.getEvents,
		backend.eventState,
		[id ?? ""],
		!!id,
		[id],
	);

	const handleDeletePage = useCallback(
		async (pageId: string, boardId: string | null) => {
			if (!id || !boardId) return;
			try {
				await backend.pageState.deletePage(id, pageId, boardId);
				pages.refetch();
			} catch (error) {
				console.error("Failed to delete page:", error);
			}
		},
		[id, backend.pageState, pages],
	);

	const handleDeleteRoute = useCallback(
		async (routeId: string) => {
			if (!id) return;
			try {
				await backend.routeState.deleteRoute(id, routeId);
				routes.refetch();
			} catch (error) {
				console.error("Failed to delete route:", error);
			}
		},
		[id, backend.routeState, routes],
	);

	const handleCreateRoute = useCallback(
		async (route: CreateAppRoute) => {
			if (!id) return;
			try {
				await backend.routeState.createRoute(id, route);
				routes.refetch();
			} catch (error) {
				console.error("Failed to create route:", error);
			}
		},
		[id, backend.routeState, routes],
	);

	const handleUpdateRoute = useCallback(
		async (routeId: string, updates: UpdateAppRoute) => {
			if (!id) return;
			try {
				await backend.routeState.updateRoute(id, routeId, updates);
				routes.refetch();
			} catch (error) {
				console.error("Failed to update route:", error);
			}
		},
		[id, backend.routeState, routes],
	);

	const openPageEditor = useCallback(
		(pageId: string, boardId?: string) => {
			if (!id) return;
			const url = boardId
				? `/page-builder?id=${pageId}&app=${id}&board=${boardId}`
				: `/page-builder?id=${pageId}&app=${id}`;
			router.push(url);
		},
		[id, router],
	);

	const openBoard = useCallback(
		(boardId: string) => {
			router.push(`/flow?id=${boardId}&app=${id}`);
		},
		[id, router],
	);

	return (
		<TooltipProvider>
			<main className="h-full flex flex-col max-h-full overflow-auto md:overflow-visible min-h-0">
				<div className="container mx-auto px-6 pb-4 flex flex-col h-full gap-6">
					{/* Header */}
					<div className="flex flex-col gap-2 pt-2">
						<h1 className="text-3xl font-bold tracking-tight">UI & Events</h1>
						<p className="text-muted-foreground">
							Design pages and configure how users interact with your app
						</p>
					</div>

					<Tabs defaultValue="routes" className="flex-1 flex flex-col">
						<TabsList className="w-fit">
							<TabsTrigger value="routes" className="gap-2">
								<RouteIcon className="h-4 w-4" />
								Routes
							</TabsTrigger>
							<TabsTrigger value="pages" className="gap-2">
								<LayoutGrid className="h-4 w-4" />
								Pages
							</TabsTrigger>
							<TabsTrigger value="events" className="gap-2">
								<Sparkles className="h-4 w-4" />
								Events
							</TabsTrigger>
						</TabsList>

						<TabsContent value="routes" className="mt-6 flex-1">
							<RoutesSection
								appId={id ?? ""}
								routes={routes.data || []}
								pages={pages.data || []}
								events={events.data || []}
								onCreate={handleCreateRoute}
								onUpdate={handleUpdateRoute}
								onDelete={handleDeleteRoute}
								onOpenPage={openPageEditor}
								onOpenBoard={openBoard}
							/>
						</TabsContent>

						<TabsContent value="pages" className="mt-6 flex-1">
							<PagesSection
								pages={pages.data || []}
								onOpenPage={openPageEditor}
								onOpenBoard={openBoard}
								onDelete={handleDeletePage}
							/>
						</TabsContent>

						<TabsContent value="events" className="mt-6 flex-1">
							<EventsSection />
						</TabsContent>
					</Tabs>
				</div>
			</main>
		</TooltipProvider>
	);
}

// ============================================================================
// ROUTES SECTION
// ============================================================================

interface RouteTreeNode {
	segment: string;
	path: string;
	route?: IAppRoute;
	children: RouteTreeNode[];
}

function buildRouteTree(routes: IAppRoute[]): RouteTreeNode {
	const root: RouteTreeNode = { segment: "", path: "/", children: [], route: routes.find(r => r.path === "/") };

	for (const route of routes) {
		if (route.path === "/") continue;

		const segments = route.path.split("/").filter(Boolean);
		let current = root;
		let currentPath = "";

		for (let i = 0; i < segments.length; i++) {
			const segment = segments[i];
			currentPath += `/${segment}`;

			let child = current.children.find(c => c.segment === segment);
			if (!child) {
				child = { segment, path: currentPath, children: [] };
				current.children.push(child);
			}

			if (i === segments.length - 1) {
				child.route = route;
			}
			current = child;
		}
	}

	// Sort children alphabetically
	const sortChildren = (node: RouteTreeNode) => {
		node.children.sort((a, b) => a.segment.localeCompare(b.segment));
		node.children.forEach(sortChildren);
	};
	sortChildren(root);

	return root;
}

function RoutesSection({
	appId,
	routes,
	pages,
	events,
	onCreate,
	onUpdate,
	onDelete,
	onOpenPage,
	onOpenBoard,
}: Readonly<{
	appId: string;
	routes: IAppRoute[];
	pages: PageListItem[];
	events: { id: string; name: string; event_type: string }[];
	onCreate: (route: CreateAppRoute) => Promise<void>;
	onUpdate: (routeId: string, updates: UpdateAppRoute) => Promise<void>;
	onDelete: (routeId: string) => Promise<void>;
	onOpenPage: (pageId: string, boardId?: string) => void;
	onOpenBoard: (boardId: string) => void;
}>) {
	const [isCreateOpen, setIsCreateOpen] = useState(false);
	const routeTree = buildRouteTree(routes);

	return (
		<div className="space-y-4">
			<div className="flex justify-between items-start">
				<div className="space-y-1">
					<h2 className="text-lg font-semibold">URL Routes</h2>
					<p className="text-sm text-muted-foreground">
						Map URL paths to pages or events
					</p>
				</div>
				<Button onClick={() => setIsCreateOpen(true)} size="sm">
					<Plus className="h-4 w-4 mr-2" />
					Add Route
				</Button>
			</div>

			{routes.length > 0 ? (
				<div className="border rounded-lg overflow-hidden bg-card">
					<RouteTreeView
						node={routeTree}
						pages={pages}
						events={events}
						onDelete={onDelete}
						onOpenPage={onOpenPage}
						onOpenBoard={onOpenBoard}
						depth={0}
						isRoot
					/>
				</div>
			) : (
				<EmptyState
					icons={[Globe]}
					title="No routes configured"
					description="Routes map URL paths to pages or events. Create your first route to define how users navigate your app."
					action={{
						label: "Create Route",
						onClick: () => setIsCreateOpen(true),
					}}
					className="w-full"
				/>
			)}

			<CreateRouteDialog
				open={isCreateOpen}
				onOpenChange={setIsCreateOpen}
				pages={pages}
				events={events}
				onCreate={onCreate}
			/>
		</div>
	);
}

function RouteTreeView({
	node,
	pages,
	events,
	onDelete,
	onOpenPage,
	onOpenBoard,
	depth,
	isRoot,
	isLast = false,
}: Readonly<{
	node: RouteTreeNode;
	pages: PageListItem[];
	events: { id: string; name: string; event_type: string }[];
	onDelete: (routeId: string) => Promise<void>;
	onOpenPage: (pageId: string, boardId?: string) => void;
	onOpenBoard: (boardId: string) => void;
	depth: number;
	isRoot?: boolean;
	isLast?: boolean;
}>) {
	const hasRoute = !!node.route;
	const hasChildren = node.children.length > 0;

	return (
		<>
			{/* Current node */}
			{(isRoot || hasRoute || hasChildren) && (
				<RouteRow
					node={node}
					pages={pages}
					events={events}
					onDelete={onDelete}
					onOpenPage={onOpenPage}
					onOpenBoard={onOpenBoard}
					depth={depth}
					isRoot={isRoot}
				/>
			)}

			{/* Children */}
			{node.children.map((child, index) => (
				<RouteTreeView
					key={child.path}
					node={child}
					pages={pages}
					events={events}
					onDelete={onDelete}
					onOpenPage={onOpenPage}
					onOpenBoard={onOpenBoard}
					depth={isRoot ? 0 : depth + 1}
					isLast={index === node.children.length - 1}
				/>
			))}
		</>
	);
}

function RouteRow({
	node,
	pages,
	events,
	onDelete,
	onOpenPage,
	onOpenBoard,
	depth,
	isRoot,
}: Readonly<{
	node: RouteTreeNode;
	pages: PageListItem[];
	events: { id: string; name: string; event_type: string }[];
	onDelete: (routeId: string) => Promise<void>;
	onOpenPage: (pageId: string, boardId?: string) => void;
	onOpenBoard: (boardId: string) => void;
	depth: number;
	isRoot?: boolean;
}>) {
	const route = node.route;
	const pageInfo = route?.targetType === "page"
		? pages.find((p) => p.pageId === route.pageId)
		: null;
	const eventInfo = route?.targetType === "event"
		? events.find((e) => e.id === route.eventId)
		: null;

	const targetName = pageInfo?.name || eventInfo?.name;
	const isHomePage = node.path === "/";

	return (
		<div className={cn(
			"group flex items-center gap-2 px-3 py-2 hover:bg-muted/50 transition-colors border-b border-border/40 last:border-b-0",
			!route && "text-muted-foreground"
		)}>
			{/* Indentation & tree lines */}
			<div className="flex items-center" style={{ width: depth * 20 }}>
				{Array.from({ length: depth }).map((_, i) => (
					<div key={i} className="w-5 h-full flex justify-center">
						<div className="w-px h-full bg-border/60" />
					</div>
				))}
			</div>

			{/* Path segment */}
			<div className="flex items-center gap-2 min-w-[140px]">
				{depth > 0 && (
					<div className="text-muted-foreground/40">└</div>
				)}
				<code className={cn(
					"text-sm font-mono",
					isHomePage && "font-semibold",
					!route && "text-muted-foreground"
				)}>
					{isRoot ? "/" : `/${node.segment}`}
				</code>
				{isHomePage && route && (
					<Badge variant="secondary" className="text-[10px] px-1.5 py-0">
						Home
					</Badge>
				)}
			</div>

			{/* Target info */}
			<div className="flex-1 flex items-center gap-2 min-w-0">
				{route ? (
					<>
						<ArrowRight className="h-3.5 w-3.5 text-muted-foreground/50 shrink-0" />
						<div className={cn(
							"flex items-center gap-1.5 px-2 py-0.5 rounded-md text-xs",
							route.targetType === "page"
								? "bg-blue-500/10 text-blue-600 dark:text-blue-400"
								: "bg-amber-500/10 text-amber-600 dark:text-amber-400"
						)}>
							{route.targetType === "page" ? (
								<FileText className="h-3 w-3" />
							) : (
								<Sparkles className="h-3 w-3" />
							)}
							<span className="truncate max-w-[200px]">{targetName}</span>
						</div>
						{route.targetType === "page" && route.pageVersion && (
							<Badge variant="outline" className="text-[10px] px-1.5 py-0 font-mono">
								v{route.pageVersion.join(".")}
							</Badge>
						)}
					</>
				) : (
					<span className="text-xs text-muted-foreground italic">No target</span>
				)}
			</div>

			{/* Actions */}
			{route && (
				<div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
					{route.targetType === "page" && pageInfo && (
						<>
							<Tooltip>
								<TooltipTrigger asChild>
									<Button
										variant="ghost"
										size="icon"
										className="h-7 w-7"
										onClick={() => onOpenPage(route.pageId!, pageInfo?.boardId ?? undefined)}
									>
										<Pencil className="h-3.5 w-3.5" />
									</Button>
								</TooltipTrigger>
								<TooltipContent side="top">Edit Page</TooltipContent>
							</Tooltip>
							{pageInfo?.boardId && (
								<Tooltip>
									<TooltipTrigger asChild>
										<Button
											variant="ghost"
											size="icon"
											className="h-7 w-7"
											onClick={() => onOpenBoard(pageInfo.boardId!)}
										>
											<Workflow className="h-3.5 w-3.5" />
										</Button>
									</TooltipTrigger>
									<TooltipContent side="top">Open Flow</TooltipContent>
								</Tooltip>
							)}
						</>
					)}
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="destructive"
								size="icon"
								className="h-7 w-7"
								onClick={() => onDelete(route.id)}
							>
								<Trash2 className="h-3.5 w-3.5" />
							</Button>
						</TooltipTrigger>
						<TooltipContent side="top">Delete Route</TooltipContent>
					</Tooltip>
				</div>
			)}
		</div>
	);
}

function CreateRouteDialog({
	open,
	onOpenChange,
	pages,
	events,
	onCreate,
}: Readonly<{
	open: boolean;
	onOpenChange: (open: boolean) => void;
	pages: PageListItem[];
	events: { id: string; name: string; event_type: string }[];
	onCreate: (route: CreateAppRoute) => Promise<void>;
}>) {
	const backend = useBackend();
	const searchParams = useSearchParams();
	const appId = searchParams.get("id") ?? "";

	const [path, setPath] = useState("/");
	const [targetType, setTargetType] = useState<RouteTargetType>("page");
	const [pageId, setPageId] = useState("");
	const [boardId, setBoardId] = useState<string | undefined>(undefined);
	const [eventId, setEventId] = useState("");
	const [selectedVersion, setSelectedVersion] = useState<string>("latest");
	const [isLoading, setIsLoading] = useState(false);

	const boardVersions = useInvoke(
		backend.boardState.getBoardVersions,
		backend.boardState,
		[appId, boardId ?? ""],
		!!boardId && !!appId,
		[boardId, appId],
	);

	const handleCreate = async () => {
		setIsLoading(true);
		try {
			const pageVersion = targetType === "page" && selectedVersion !== "latest"
				? (selectedVersion.split(".").map(Number) as Version)
				: undefined;
			await onCreate({
				path,
				targetType,
				pageId: targetType === "page" ? pageId : undefined,
				boardId: targetType === "page" ? boardId : undefined,
				pageVersion,
				eventId: targetType === "event" ? eventId : undefined,
				priority: 0,
			});
			onOpenChange(false);
			resetForm();
		} finally {
			setIsLoading(false);
		}
	};

	const resetForm = () => {
		setPath("/");
		setTargetType("page");
		setPageId("");
		setBoardId(undefined);
		setEventId("");
		setSelectedVersion("latest");
	};

	const handlePageSelect = (value: string) => {
		setPageId(value);
		setSelectedVersion("latest");
		const page = pages.find((p) => p.pageId === value);
		if (page) {
			setBoardId(page.boardId ?? undefined);
		}
	};

	const isValid =
		path &&
		((targetType === "page" && pageId) || (targetType === "event" && eventId));

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle>Create Route</DialogTitle>
					<DialogDescription>
						Define a URL path and what it displays
					</DialogDescription>
				</DialogHeader>
				<div className="space-y-4 py-4">
					<div className="space-y-2">
						<Label htmlFor="path">Path</Label>
						<Input
							id="path"
							value={path}
							onChange={(e) => setPath(e.target.value)}
							placeholder="/about"
							className="font-mono"
						/>
						<p className="text-xs text-muted-foreground">
							Use "/" for the home page
						</p>
					</div>

					<div className="space-y-2">
						<Label>Target Type</Label>
						<Select
							value={targetType}
							onValueChange={(v) => setTargetType(v as RouteTargetType)}
						>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="page">
									<div className="flex items-center gap-2">
										<FileText className="h-4 w-4" />
										Page
									</div>
								</SelectItem>
								<SelectItem value="event">
									<div className="flex items-center gap-2">
										<Sparkles className="h-4 w-4" />
										Event
									</div>
								</SelectItem>
							</SelectContent>
						</Select>
					</div>

					{targetType === "page" && (
						<>
							<div className="space-y-2">
								<Label>Page</Label>
								<Select value={pageId} onValueChange={handlePageSelect}>
									<SelectTrigger>
										<SelectValue placeholder="Select a page" />
									</SelectTrigger>
									<SelectContent>
										{pages.length === 0 ? (
											<div className="p-2 text-sm text-muted-foreground text-center">
												No pages available. Create one in a flow first.
											</div>
										) : (
											pages.map((p) => (
												<SelectItem key={p.pageId} value={p.pageId}>
													{p.name}
												</SelectItem>
											))
										)}
									</SelectContent>
								</Select>
							</div>
							{boardId && (
								<div className="space-y-2">
									<Label>Version</Label>
									<Select
										value={selectedVersion}
										onValueChange={setSelectedVersion}
									>
										<SelectTrigger>
											<SelectValue />
										</SelectTrigger>
										<SelectContent>
											<SelectItem value="latest">Latest</SelectItem>
											{(boardVersions.data ?? []).map((v) => (
												<SelectItem key={v.join(".")} value={v.join(".")}>
													v{v.join(".")}
												</SelectItem>
											))}
										</SelectContent>
									</Select>
									<p className="text-xs text-muted-foreground">
										{selectedVersion === "latest"
											? "Always use the latest published version"
											: `Lock to version ${selectedVersion}`}
									</p>
								</div>
							)}
						</>
					)}

					{targetType === "event" && (
						<div className="space-y-2">
							<Label>Event</Label>
							<Select value={eventId} onValueChange={setEventId}>
								<SelectTrigger>
									<SelectValue placeholder="Select an event" />
								</SelectTrigger>
								<SelectContent>
									{events.length === 0 ? (
										<div className="p-2 text-sm text-muted-foreground text-center">
											No events available
										</div>
									) : (
										events.map((event) => (
											<SelectItem key={event.id} value={event.id}>
												{event.name}
											</SelectItem>
										))
									)}
								</SelectContent>
							</Select>
						</div>
					)}
				</div>
				<DialogFooter>
					<Button variant="outline" onClick={() => onOpenChange(false)}>
						Cancel
					</Button>
					<Button onClick={handleCreate} disabled={isLoading || !isValid}>
						{isLoading ? "Creating..." : "Create Route"}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

// ============================================================================
// PAGES SECTION
// ============================================================================

function PagesSection({
	pages,
	onOpenPage,
	onOpenBoard,
	onDelete,
}: Readonly<{
	pages: PageListItem[];
	onOpenPage: (pageId: string, boardId?: string) => void;
	onOpenBoard: (boardId: string) => void;
	onDelete: (pageId: string, boardId: string | null) => Promise<void>;
}>) {
	if (pages.length === 0) {
		return (
			<EmptyState
				icons={[LayoutGrid]}
				title="No pages yet"
				description="Pages are created from within a flow. Open a flow and use the Pages panel to create your first page."
			/>
		);
	}

	return (
		<div className="space-y-4">
			<div className="space-y-1">
				<h2 className="text-lg font-semibold">All Pages</h2>
				<p className="text-sm text-muted-foreground">
					Manage your app's visual interfaces
				</p>
			</div>

			<div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
				{pages.map((pageInfo) => (
					<PageCard
						key={pageInfo.pageId}
						pageInfo={pageInfo}
						onOpen={() => onOpenPage(pageInfo.pageId, pageInfo.boardId ?? undefined)}
						onOpenBoard={pageInfo.boardId ? () => onOpenBoard(pageInfo.boardId!) : undefined}
						onDelete={() => onDelete(pageInfo.pageId, pageInfo.boardId ?? null)}
					/>
				))}
			</div>
		</div>
	);
}

function PageCard({
	pageInfo,
	onOpen,
	onOpenBoard,
	onDelete,
}: Readonly<{
	pageInfo: PageListItem;
	onOpen: () => void;
	onOpenBoard?: () => void;
	onDelete: () => void;
}>) {
	return (
		<Card className="group hover:shadow-lg transition-all duration-200 border-border/60 hover:border-primary/30 overflow-hidden">
			{/* Preview Area */}
			<div
				className="h-32 bg-linear-to-br from-muted/50 to-muted flex items-center justify-center cursor-pointer relative"
				onClick={onOpen}
			>
				<div className="absolute inset-0 bg-grid-pattern opacity-5" />
				<FileText className="h-12 w-12 text-muted-foreground/30" />
				<div className="absolute inset-0 bg-primary/5 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
					<Button variant="secondary" size="sm" className="gap-2">
						<Pencil className="h-4 w-4" />
						Edit Page
					</Button>
				</div>
			</div>

			<CardContent className="p-4">
				<div className="flex items-start justify-between gap-2">
					<div className="min-w-0 flex-1">
						<h3 className="font-semibold truncate">{pageInfo.name}</h3>
						{pageInfo.description && (
							<p className="text-sm text-muted-foreground line-clamp-1 mt-0.5">
								{pageInfo.description}
							</p>
						)}
					</div>
				</div>

				<div className="flex items-center justify-between mt-4 pt-3 border-t border-border/40">
					<div className="flex items-center gap-2">
						{onOpenBoard && (
							<Tooltip>
								<TooltipTrigger asChild>
									<Button
										variant="outline"
										size="sm"
										className="h-8 gap-1.5"
										onClick={onOpenBoard}
									>
										<Workflow className="h-3.5 w-3.5" />
										Flow
									</Button>
								</TooltipTrigger>
								<TooltipContent>Open connected flow</TooltipContent>
							</Tooltip>
						)}
					</div>
					<div className="flex items-center gap-1">
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="ghost"
									size="icon"
									className="h-8 w-8"
									onClick={onOpen}
								>
									<Pencil className="h-4 w-4" />
								</Button>
							</TooltipTrigger>
							<TooltipContent>Edit Page</TooltipContent>
						</Tooltip>
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="ghost"
									size="icon"
									className="h-8 w-8 text-destructive hover:text-destructive hover:bg-destructive/10"
									onClick={onDelete}
								>
									<Trash2 className="h-4 w-4" />
								</Button>
							</TooltipTrigger>
							<TooltipContent>Delete Page</TooltipContent>
						</Tooltip>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}

// ============================================================================
// EVENTS SECTION
// ============================================================================

function EventsSection() {
	const handleStartOAuth = useCallback(async (provider: IOAuthProvider) => {
		await oauthService.startAuthorization(provider);
	}, []);

	const handleRefreshToken = useCallback(
		async (provider: IOAuthProvider, token: IStoredOAuthToken) => {
			return oauthService.refreshToken(provider, token);
		},
		[],
	);

	return (
		<EventsPage
			eventMapping={EVENT_CONFIG}
			tokenStore={oauthTokenStore}
			consentStore={oauthConsentStore}
			onStartOAuth={handleStartOAuth}
			onRefreshToken={handleRefreshToken}
			basePath="/library/config/pages"
		/>
	);
}
