"use client";

import type {
	CreateAppRoute,
	IAppRoute,
	RouteTargetType,
	UpdateAppRoute,
} from "../../state/backend-state/route-state";
import type { Version } from "../../state/backend-state/widget-state";
import type { IMetadata } from "../../types";
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
} from "../ui";
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
import { useCallback, useState } from "react";

export interface PageRouteManagerProps {
	appId: string;
	pages: Array<[string, string | null, string | null, IMetadata | null]>;
	routes: IAppRoute[];
	onDeletePage: (pageId: string, boardId: string | null) => Promise<void>;
	onCreatePage: (name: string, boardId?: string) => Promise<void>;
	onCreateRoute: (route: CreateAppRoute) => Promise<void>;
	onUpdateRoute: (routeId: string, update: UpdateAppRoute) => Promise<void>;
	onDeleteRoute: (routeId: string) => Promise<void>;
	onNavigate: (path: string) => void;
}

export function PageRouteManager({
	appId,
	pages,
	routes,
	onDeletePage,
	onCreatePage,
	onCreateRoute,
	onUpdateRoute,
	onDeleteRoute,
	onNavigate,
}: PageRouteManagerProps) {
	const [activeTab, setActiveTab] = useState<"pages" | "routes">("pages");
	const [createPageDialogOpen, setCreatePageDialogOpen] = useState(false);
	const [newPageName, setNewPageName] = useState("");
	const [selectedBoardId, setSelectedBoardId] = useState<string>("");

	const [createRouteDialogOpen, setCreateRouteDialogOpen] = useState(false);
	const [editRouteDialogOpen, setEditRouteDialogOpen] = useState(false);
	const [selectedRoute, setSelectedRoute] = useState<IAppRoute | null>(null);
	const [routeForm, setRouteForm] = useState<{
		path: string;
		targetType: RouteTargetType;
		pageId: string;
		eventId: string;
		boardId?: string;
		pageVersion?: Version;
	}>({
		path: "",
		targetType: "page",
		pageId: "",
		eventId: "",
	});

	const handleCreatePage = useCallback(async () => {
		if (!newPageName.trim()) return;
		await onCreatePage(newPageName, selectedBoardId || undefined);
		setCreatePageDialogOpen(false);
		setNewPageName("");
		setSelectedBoardId("");
	}, [newPageName, selectedBoardId, onCreatePage]);

	const handleCreateRoute = useCallback(async () => {
		await onCreateRoute({
			path: routeForm.path,
			targetType: routeForm.targetType,
			pageId: routeForm.targetType === "page" ? routeForm.pageId : undefined,
			eventId: routeForm.targetType === "event" ? routeForm.eventId : undefined,
			boardId: routeForm.boardId,
			pageVersion: routeForm.pageVersion,
		});
		setCreateRouteDialogOpen(false);
		setRouteForm({
			path: "",
			targetType: "page",
			pageId: "",
			eventId: "",
		});
	}, [routeForm, onCreateRoute]);

	const handleUpdateRoute = useCallback(async () => {
		if (!selectedRoute) return;
		await onUpdateRoute(selectedRoute.id, {
			path: routeForm.path,
			targetType: routeForm.targetType,
			pageId: routeForm.targetType === "page" ? routeForm.pageId : undefined,
			eventId: routeForm.targetType === "event" ? routeForm.eventId : undefined,
			boardId: routeForm.boardId,
			pageVersion: routeForm.pageVersion,
		});
		setEditRouteDialogOpen(false);
		setSelectedRoute(null);
	}, [selectedRoute, routeForm, onUpdateRoute]);

	const openEditRoute = useCallback((route: IAppRoute) => {
		setSelectedRoute(route);
		setRouteForm({
			path: route.path,
			targetType: route.targetType,
			pageId: route.pageId ?? "",
			eventId: route.eventId ?? "",
			boardId: route.boardId,
			pageVersion: route.pageVersion,
		});
		setEditRouteDialogOpen(true);
	}, []);

	return (
		<div className="space-y-6">
			<div className="flex items-center justify-between">
				<div>
					<h2 className="text-2xl font-bold">Pages & Routes</h2>
					<p className="text-muted-foreground">
						Manage your application pages and routing configuration
					</p>
				</div>
			</div>

			<Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as "pages" | "routes")}>
				<TabsList>
					<TabsTrigger value="pages" className="gap-2">
						<LayoutGrid className="h-4 w-4" />
						Pages
					</TabsTrigger>
					<TabsTrigger value="routes" className="gap-2">
						<RouteIcon className="h-4 w-4" />
						Routes
					</TabsTrigger>
				</TabsList>

				<TabsContent value="pages" className="space-y-4">
					<div className="flex justify-between items-center">
						<p className="text-sm text-muted-foreground">
							{pages.length} page{pages.length !== 1 ? "s" : ""}
						</p>
						<Button onClick={() => setCreatePageDialogOpen(true)}>
							<Plus className="h-4 w-4 mr-2" />
							New Page
						</Button>
					</div>

					{pages.length === 0 ? (
						<EmptyState
							icons={[FileText]}
							title="No pages yet"
							description="Create your first page to get started"
							action={{
								label: "Create Page",
								onClick: () => setCreatePageDialogOpen(true),
							}}
						/>
					) : (
						<div className="grid gap-4">
							{pages.map(([name, pageId, boardId, metadata]) => (
								<Card key={pageId} className="hover:shadow-md transition-shadow">
									<CardContent className="p-4">
										<div className="flex items-center justify-between">
											<div className="flex items-center gap-3">
												<div className="p-2 rounded-lg bg-primary/10">
													<FileText className="h-5 w-5 text-primary" />
												</div>
												<div>
													<h3 className="font-medium">{metadata?.name || name}</h3>
													{boardId && (
														<p className="text-sm text-muted-foreground">
															Board: {boardId.substring(0, 8)}...
														</p>
													)}
												</div>
											</div>
											<div className="flex items-center gap-2">
												<TooltipProvider>
													<Tooltip>
														<TooltipTrigger asChild>
															<Button
																variant="ghost"
																size="icon"
																onClick={() => pageId && onNavigate(`/page-builder?id=${pageId}&app=${appId}${boardId ? `&board=${boardId}` : ""}`)}
															>
																<Pencil className="h-4 w-4" />
															</Button>
														</TooltipTrigger>
														<TooltipContent>Edit Page</TooltipContent>
													</Tooltip>
												</TooltipProvider>
												{boardId && (
													<TooltipProvider>
														<Tooltip>
															<TooltipTrigger asChild>
																<Button
																	variant="ghost"
																	size="icon"
																	onClick={() => onNavigate(`/flow?id=${boardId}&app=${appId}`)}
																>
																	<Workflow className="h-4 w-4" />
																</Button>
															</TooltipTrigger>
															<TooltipContent>Open Flow</TooltipContent>
														</Tooltip>
													</TooltipProvider>
												)}
												<TooltipProvider>
													<Tooltip>
														<TooltipTrigger asChild>
															<Button
																variant="ghost"
																size="icon"
																className="text-destructive hover:text-destructive"
																onClick={() => pageId && onDeletePage(pageId, boardId)}
															>
																<Trash2 className="h-4 w-4" />
															</Button>
														</TooltipTrigger>
														<TooltipContent>Delete Page</TooltipContent>
													</Tooltip>
												</TooltipProvider>
											</div>
										</div>
									</CardContent>
								</Card>
							))}
						</div>
					)}
				</TabsContent>

				<TabsContent value="routes" className="space-y-4">
					<div className="flex justify-between items-center">
						<p className="text-sm text-muted-foreground">
							{routes.length} route{routes.length !== 1 ? "s" : ""}
						</p>
						<Button onClick={() => setCreateRouteDialogOpen(true)}>
							<Plus className="h-4 w-4 mr-2" />
							New Route
						</Button>
					</div>

					{routes.length === 0 ? (
						<EmptyState
							icons={[Globe]}
							title="No routes yet"
							description="Create your first route to map URLs to pages"
							action={{
								label: "Create Route",
								onClick: () => setCreateRouteDialogOpen(true),
							}}
						/>
					) : (
						<div className="space-y-3">
							{routes.map((route) => {
								const targetPage = pages.find(([, id]) => id === route.pageId);
								return (
									<Card key={route.id} className="hover:shadow-md transition-shadow">
										<CardContent className="p-4">
											<div className="flex items-center justify-between">
												<div className="flex items-center gap-4">
													<div className="flex items-center gap-2">
														<code className="px-2 py-1 rounded bg-muted font-mono text-sm">
															{route.path}
														</code>
														<ArrowRight className="h-4 w-4 text-muted-foreground" />
													</div>
													<div className="flex items-center gap-2">
														{route.targetType === "page" ? (
															<>
																<FileText className="h-4 w-4 text-blue-500" />
																<span>{targetPage?.[3]?.name || route.pageId}</span>
															</>
														) : (
															<>
																<Sparkles className="h-4 w-4 text-amber-500" />
																<span>Event: {route.eventId}</span>
															</>
														)}
													</div>
													{route.path === "/" && (
														<Badge variant="secondary">Default</Badge>
													)}
												</div>
												<div className="flex items-center gap-2">
													<TooltipProvider>
														<Tooltip>
															<TooltipTrigger asChild>
																<Button
																	variant="ghost"
																	size="icon"
																	onClick={() => openEditRoute(route)}
																>
																	<Pencil className="h-4 w-4" />
																</Button>
															</TooltipTrigger>
															<TooltipContent>Edit Route</TooltipContent>
														</Tooltip>
													</TooltipProvider>
													<TooltipProvider>
														<Tooltip>
															<TooltipTrigger asChild>
																<Button
																	variant="destructive"
																	size="icon"
																	onClick={() => onDeleteRoute(route.id)}
																>
																	<Trash2 className="h-4 w-4" />
																</Button>
															</TooltipTrigger>
															<TooltipContent>Delete Route</TooltipContent>
														</Tooltip>
													</TooltipProvider>
												</div>
											</div>
										</CardContent>
									</Card>
								);
							})}
						</div>
					)}
				</TabsContent>
			</Tabs>

			{/* Create Page Dialog */}
			<Dialog open={createPageDialogOpen} onOpenChange={setCreatePageDialogOpen}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Create New Page</DialogTitle>
						<DialogDescription>
							Create a new page for your application
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4">
						<div>
							<Label htmlFor="page-name">Page Name</Label>
							<Input
								id="page-name"
								value={newPageName}
								onChange={(e) => setNewPageName(e.target.value)}
								placeholder="My Page"
							/>
						</div>
					</div>
					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => setCreatePageDialogOpen(false)}
						>
							Cancel
						</Button>
						<Button onClick={handleCreatePage}>Create Page</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>

			{/* Create Route Dialog */}
			<Dialog open={createRouteDialogOpen} onOpenChange={setCreateRouteDialogOpen}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Create New Route</DialogTitle>
						<DialogDescription>
							Map a URL path to a page or event
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4">
						<div>
							<Label htmlFor="route-path">Path</Label>
							<Input
								id="route-path"
								value={routeForm.path}
								onChange={(e) =>
									setRouteForm({ ...routeForm, path: e.target.value })
								}
								placeholder="/about"
							/>
							<p className="text-xs text-muted-foreground mt-1">
								Use "/" for the default route
							</p>
						</div>
						<div>
							<Label>Target Type</Label>
							<Select
								value={routeForm.targetType}
								onValueChange={(v) =>
									setRouteForm({ ...routeForm, targetType: v as RouteTargetType })
								}
							>
								<SelectTrigger>
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="page">Page</SelectItem>
									<SelectItem value="event">Event</SelectItem>
								</SelectContent>
							</Select>
						</div>
						{routeForm.targetType === "page" && (
							<div>
								<Label>Page</Label>
								<Select
									value={routeForm.pageId}
									onValueChange={(v) =>
										setRouteForm({ ...routeForm, pageId: v })
									}
								>
									<SelectTrigger>
										<SelectValue placeholder="Select a page" />
									</SelectTrigger>
									<SelectContent>
										{pages.map(([name, pageId, , metadata]) => (
											<SelectItem key={pageId} value={pageId || ""}>
												{metadata?.name || name}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
						)}
						{routeForm.targetType === "event" && (
							<div>
								<Label htmlFor="event-id">Event ID</Label>
								<Input
									id="event-id"
									value={routeForm.eventId}
									onChange={(e) =>
										setRouteForm({ ...routeForm, eventId: e.target.value })
									}
									placeholder="event-id"
								/>
							</div>
						)}
					</div>
					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => setCreateRouteDialogOpen(false)}
						>
							Cancel
						</Button>
						<Button onClick={handleCreateRoute}>Create Route</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>

			{/* Edit Route Dialog */}
			<Dialog open={editRouteDialogOpen} onOpenChange={setEditRouteDialogOpen}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Edit Route</DialogTitle>
						<DialogDescription>
							Update route configuration
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4">
						<div>
							<Label htmlFor="edit-route-path">Path</Label>
							<Input
								id="edit-route-path"
								value={routeForm.path}
								onChange={(e) =>
									setRouteForm({ ...routeForm, path: e.target.value })
								}
								placeholder="/about"
							/>
						</div>
						<div>
							<Label>Target Type</Label>
							<Select
								value={routeForm.targetType}
								onValueChange={(v) =>
									setRouteForm({ ...routeForm, targetType: v as RouteTargetType })
								}
							>
								<SelectTrigger>
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="page">Page</SelectItem>
									<SelectItem value="event">Event</SelectItem>
								</SelectContent>
							</Select>
						</div>
						{routeForm.targetType === "page" && (
							<div>
								<Label>Page</Label>
								<Select
									value={routeForm.pageId}
									onValueChange={(v) =>
										setRouteForm({ ...routeForm, pageId: v })
									}
								>
									<SelectTrigger>
										<SelectValue placeholder="Select a page" />
									</SelectTrigger>
									<SelectContent>
										{pages.map(([name, pageId, , metadata]) => (
											<SelectItem key={pageId} value={pageId || ""}>
												{metadata?.name || name}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
						)}
						{routeForm.targetType === "event" && (
							<div>
								<Label htmlFor="edit-event-id">Event ID</Label>
								<Input
									id="edit-event-id"
									value={routeForm.eventId}
									onChange={(e) =>
										setRouteForm({ ...routeForm, eventId: e.target.value })
									}
									placeholder="event-id"
								/>
							</div>
						)}
					</div>
					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => setEditRouteDialogOpen(false)}
						>
							Cancel
						</Button>
						<Button onClick={handleUpdateRoute}>Save Changes</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
