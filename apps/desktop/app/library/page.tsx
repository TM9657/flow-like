"use client";

import { invoke } from "@tauri-apps/api/core";
import {
	AppCard,
	Button,
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	EmptyState,
	type IApp,
	type IMetadata,
	Input,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Separator,
	useBackend,
	useInvoke,
	useMiniSearch,
} from "@tm9657/flow-like-ui";
import {
	ArrowUpDown,
	FilesIcon,
	Grid3X3,
	ImportIcon,
	LayoutGridIcon,
	LibraryIcon,
	Link2,
	List,
	Search,
	SearchIcon,
	Sparkles,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

export default function YoursPage() {
	const backend = useBackend();
	const apps = useInvoke(backend.appState.getApps, backend.appState, []);
	const router = useRouter();
	const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
	const [searchQuery, setSearchQuery] = useState("");
	const [joinDialogOpen, setJoinDialogOpen] = useState(false);
	const [inviteLink, setInviteLink] = useState("");
	const [sortBy, setSortBy] = useState<
		"created" | "updated" | "visibility" | "name"
	>("created");

	const allItems = useMemo(() => {
		const map = new Map<string, IMetadata & { id: string; app: IApp }>();
		apps.data?.forEach(([app, meta]) => {
			if (meta) map.set(app.id, { ...meta, id: app.id, app });
		});
		return Array.from(map.values());
	}, [apps.data]);

	const sortItems = useCallback(
		(items: Array<IMetadata & { id: string; app: IApp }>) => {
			return items.toSorted((a, b) => {
				switch (sortBy) {
					case "created":
						return (
							(b?.created_at?.secs_since_epoch ?? 0) -
							(a?.created_at?.secs_since_epoch ?? 0)
						);
					case "updated":
						return (
							(b?.updated_at?.secs_since_epoch ?? 0) -
							(a?.updated_at?.secs_since_epoch ?? 0)
						);
					case "visibility":
						const aVisibility = a?.app.visibility;
						const bVisibility = b?.app.visibility;
						return aVisibility.localeCompare(bVisibility);
					case "name":
						return (a?.name ?? "").localeCompare(b?.name ?? "");
					default:
						return 0;
				}
			});
		},
		[sortBy],
	);

	const handleJoin = useCallback(async () => {
		const url = new URL(inviteLink);
		const queryParams = url.searchParams;
		const appId = queryParams.get("appId");
		if (!appId) {
			toast.error("Invalid invite link. Please check the link and try again.");
			return;
		}
		const token = queryParams.get("token");
		if (!token) {
			toast.error("Invalid invite link. Please check the link and try again.");
			return;
		}
		router.push(`/join?appId=${appId}&token=${token}`);
		setJoinDialogOpen(false);
		setInviteLink("");
	}, [inviteLink, router]);

	const { addAll, removeAll, clearSearch, search, searchResults } =
		useMiniSearch(allItems, {
			fields: [
				"name",
				"description",
				"long_description",
				"tags",
				"category",
				"id",
			],
		});

	useEffect(() => {
		if (allItems.length > 0) {
			removeAll();
			addAll(allItems);
		}
		return () => {
			removeAll();
			clearSearch();
		};
	}, [allItems]);

	const renderAppCards = (items: any[]) => {
		if (viewMode === "grid") {
			return (
				<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4 px-2">
					{items.map((meta) => (
						<div key={viewMode + meta.id} className="group w-full">
							<AppCard
								apps={items}
								app={meta.app}
								metadata={meta as IMetadata}
								variant="extended"
								onClick={() => router.push(`/use?id=${meta.id}`)}
								className="w-full"
							/>
						</div>
					))}
				</div>
			);
		}

		return (
			<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-2 px-2">
				{items.map((meta) => (
					<div key={`left${meta.id}`} className="group">
						<AppCard
							apps={items}
							app={meta.app}
							metadata={meta as IMetadata}
							variant="small"
							onClick={() => router.push(`/use?id=${meta.id}`)}
							className="w-full"
						/>
					</div>
				))}
			</div>
		);
	};

	return (
		<main className="min-h-dvh max-h-dvh flex flex-col w-full p-6 bg-gradient-to-br from-background to-muted/20">
			{/* Header Section */}
			<div className="flex flex-col space-y-6 mb-8">
				<div className="flex items-center justify-between">
					<div className="flex items-center space-x-3">
						<div className="p-2 rounded-xl bg-primary/10 text-primary">
							<LibraryIcon className="h-8 w-8" />
						</div>
						<div>
							<h1 className="text-4xl font-bold tracking-tight bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text">
								Library
							</h1>
							<p className="text-muted-foreground mt-1">
								Manage and create your custom applications
							</p>
						</div>
					</div>
					<div className="flex items-center space-x-2">
						<Button
							size="lg"
							variant="outline"
							className="shadow-lg hover:shadow-xl transition-all duration-200"
							onClick={async () => {
								await invoke("import_app");
								await apps.refetch();
							}}
						>
							<ImportIcon className="mr-2 h-4 w-4" />
							Import Project
						</Button>
						<Button
							size="lg"
							variant="outline"
							className="shadow-lg hover:shadow-xl transition-all duration-200"
							onClick={() => setJoinDialogOpen(true)}
						>
							<Link2 className="mr-2 h-4 w-4" />
							Join Project
						</Button>
						<Link href={"/library/new"}>
							<Button
								size="lg"
								className="shadow-lg hover:shadow-xl transition-all duration-200"
							>
								<Sparkles className="mr-2 h-4 w-4" />
								Create App
							</Button>
						</Link>
					</div>
				</div>

				{/* Join Project Dialog */}
				<Dialog open={joinDialogOpen} onOpenChange={setJoinDialogOpen}>
					<DialogContent className="sm:max-w-md animate-in fade-in-0 slide-in-from-top-8 rounded-2xl shadow-2xl border-none bg-background/95 backdrop-blur-lg">
						<DialogHeader className="space-y-3">
							<div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
								<Link2 className="h-6 w-6 text-primary" />
							</div>
							<DialogTitle className="text-center text-2xl font-bold">
								Join a Project
							</DialogTitle>
							<DialogDescription className="text-center text-muted-foreground">
								Paste your invite link below to join a project.
								<br />
								You’ll instantly get access if the link is valid.
							</DialogDescription>
						</DialogHeader>
						<div className="flex flex-col gap-4 py-2">
							<Input
								autoFocus
								placeholder="Paste invite link here…"
								value={inviteLink}
								onChange={(e) => setInviteLink(e.target.value)}
								className="w-full"
							/>
							<p className="text-xs text-muted-foreground text-center">
								Ask a teammate for an invite link if you don’t have one.
							</p>
						</div>
						<DialogFooter className="flex flex-row gap-1 justify-center pt-2">
							<DialogClose asChild>
								<Button variant="outline">Cancel</Button>
							</DialogClose>
							<Button onClick={handleJoin} disabled={!inviteLink.trim()}>
								<Link2 className="mr-2 h-4 w-4" />
								Join
							</Button>
						</DialogFooter>
					</DialogContent>
				</Dialog>

				{/* Search and Filter Bar */}
				<div className="flex items-center justify-between space-x-4">
					<div className="relative flex-1 max-w-md">
						<SearchIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 text-foreground h-4 w-4 z-10" />
						<Input
							placeholder="Search apps..."
							value={searchQuery}
							onChange={(e) => {
								search(e.target.value);
								setSearchQuery(e.target.value);
							}}
							className="pl-10 bg-background/50 backdrop-blur-sm border-border/50"
						/>
					</div>
					<div className="flex items-center space-x-2">
						<Select
							value={sortBy}
							onValueChange={(value: typeof sortBy) => setSortBy(value)}
						>
							<SelectTrigger className="w-[140px]">
								<ArrowUpDown className="h-4 w-4 mr-2" />
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="created">Created</SelectItem>
								<SelectItem value="updated">Updated</SelectItem>
								<SelectItem value="visibility">Visibility</SelectItem>
								<SelectItem value="name">Name</SelectItem>
							</SelectContent>
						</Select>
						<Button
							variant={"outline"}
							size="sm"
							onClick={() =>
								setViewMode((old) => (old === "grid" ? "list" : "grid"))
							}
						>
							{viewMode === "grid" ? (
								<List className="h-4 w-4" />
							) : (
								<Grid3X3 className="h-4 w-4" />
							)}
						</Button>
					</div>
				</div>
			</div>

			<Separator className="mb-8" />

			{/* Content Section */}
			<div className="flex-1 overflow-auto">
				{allItems.length === 0 && (
					<EmptyState
						action={{
							label: "Create Your First App",
							onClick: () => {
								router.push("/library/new");
							},
						}}
						icons={[Sparkles, LayoutGridIcon, FilesIcon]}
						className="min-w-full min-h-full flex-grow h-full border-2 border-dashed border-border/50 rounded-xl bg-muted/20"
						title="Welcome to Your Library"
						description="Create powerful custom applications based on your data. Get started with your first app today - it's free and secure."
					/>
				)}

				{searchQuery === "" &&
					allItems.length > 0 &&
					renderAppCards(sortItems(allItems))}

				{searchQuery !== "" &&
					(searchResults?.length ?? 0) > 0 &&
					renderAppCards(sortItems(searchResults ?? []))}

				{searchResults && searchResults.length === 0 && searchQuery && (
					<div className="flex flex-col items-center justify-center h-64 text-center">
						<Search className="h-12 w-12 text-muted-foreground mb-4" />
						<h3 className="text-lg font-semibold mb-2">No apps found</h3>
						<p className="text-muted-foreground">
							Try adjusting your search terms or create a new app.
						</p>
					</div>
				)}
			</div>
		</main>
	);
}
