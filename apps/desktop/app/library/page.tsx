"use client";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
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
	useMobileHeader,
	useNetworkStatus,
	useQueryClient,
	useSpotlightStore,
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
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import ImportEncryptedDialog from "./components/ImportEncryptedDialog";

export default function YoursPage() {
	const backend = useBackend();
	const queryClient = useQueryClient();
	const isOnline = useNetworkStatus();
	const currentProfile = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
	);
	const apps = useInvoke(backend.appState.getApps, backend.appState, []);
	const router = useRouter();
	const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
	const [searchQuery, setSearchQuery] = useState("");
	const [joinDialogOpen, setJoinDialogOpen] = useState(false);
	const [importDialogOpen, setImportDialogOpen] = useState(false);
	const [encryptedImportPath, setEncryptedImportPath] = useState<string | null>(
		null,
	);
	const [inviteLink, setInviteLink] = useState("");
	const [sortBy, setSortBy] = useState<
		"created" | "updated" | "visibility" | "name"
	>("created");

	// Handle app click with force refetch when online
	const handleAppClick = useCallback(
		(appId: string) => {
			if (isOnline) {
				// Force refetch all queries when clicking on an app (if online)
				queryClient.invalidateQueries();
			}
			router.push(`/use?id=${appId}`);
		},
		[isOnline, queryClient, router],
	);

	const isMobileDevice = useCallback(() => {
		if (typeof navigator === "undefined") return false;
		const ua = navigator.userAgent.toLowerCase();
		if (/android|iphone|ipad|ipod/.test(ua)) return true;
		if (
			"userAgentData" in navigator &&
			typeof (navigator as any).userAgentData?.mobile === "boolean" &&
			(navigator as any).userAgentData.mobile
		)
			return true;
		const platform = navigator.platform?.toLowerCase() ?? "";
		const maxTouchPoints =
			(navigator as Navigator & { maxTouchPoints?: number }).maxTouchPoints ??
			0;
		return /mac/.test(platform) && maxTouchPoints > 1;
	}, []);

	const normalizePickerPath = (input: string): string => {
		if (!input.startsWith("file://")) {
			return input;
		}

		try {
			const url = new URL(input);
			let pathname = decodeURIComponent(url.pathname);
			if (/^[A-Za-z]:/.test(pathname.slice(1, 3))) {
				pathname = pathname.slice(1);
			}
			return pathname || input;
		} catch {
			const withoutScheme = input.replace(/^file:\/\//, "");
			if (withoutScheme.startsWith("/")) {
				return withoutScheme;
			}
			return `/${withoutScheme}`;
		}
	};

	const resolveSelectedPath = (selected: unknown): string | null => {
		if (!selected) return null;
		if (typeof selected === "string") return selected;
		if (Array.isArray(selected)) return resolveSelectedPath(selected[0]);
		if (typeof selected === "object") {
			const candidate = selected as { path?: unknown; uri?: unknown };
			if (typeof candidate.path === "string")
				return normalizePickerPath(candidate.path);
			if (typeof candidate.uri === "string")
				return normalizePickerPath(candidate.uri);
		}
		return null;
	};

	const importApp = useCallback(
		async (path: string) => {
			if (path.toLowerCase().endsWith(".enc.flow-app")) {
				setEncryptedImportPath(path);
				setImportDialogOpen(true);
				return;
			}
			const toastId = toast.loading("Importing app...", {
				description: "Please wait.",
			});
			try {
				await invoke("import_app_from_file", { path });
				toast.success("App imported successfully!", { id: toastId });
				await apps.refetch();
			} catch (err) {
				console.error(err);
				toast.error("Failed to import app", { id: toastId });
			}
		},
		[apps],
	);

	const pickImportFile = useCallback(async () => {
		type Filter = { name: string; extensions: string[] };
		const isMobile = isMobileDevice();
		const filtersOption: Filter[] | undefined = isMobile
			? undefined
			: [
					{
						name: "Flow App",
						extensions: ["flow-app"],
					},
				];

		const selection = await open({
			multiple: false,
			directory: false,
			...(filtersOption ? { filters: filtersOption } : {}),
		});
		const path = resolveSelectedPath(selection);
		if (!path) {
			toast.error("Unable to open selected file.");
			return;
		}
		await importApp(path);
	}, [importApp, isMobileDevice]);

	const allItems = useMemo(() => {
		if (!currentProfile.data) return [];
		const currentProfileApps = new Set(
			(currentProfile.data.hub_profile.apps ?? []).map((a) => a.app_id),
		);
		const map = new Map<string, IMetadata & { id: string; app: IApp }>();
		apps.data?.forEach(([app, meta]) => {
			if (meta) map.set(app.id, { ...meta, id: app.id, app });
		});
		return Array.from(map.values()).filter((item) =>
			currentProfileApps.has(item.id),
		);
	}, [apps.data, currentProfile.data]);

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
	}, [allItems, removeAll, addAll, clearSearch]);

	const menuActions = useMemo(
		() => [
			<Button
				key="import"
				size="icon"
				variant="outline"
				onClick={pickImportFile}
			>
				<ImportIcon className="h-4 w-4" />
			</Button>,
			<Button
				key={"join"}
				size="icon"
				variant="outline"
				onClick={() => setJoinDialogOpen(true)}
			>
				<Link2 className="h-4 w-4" />
			</Button>,
		],
		[pickImportFile],
	);

	// Listen for import/file events (e.g., from iOS when a file is opened with the app)
	useEffect(() => {
		const unlistenPromise = listen<{ path: string }>(
			"import/file",
			async (event) => {
				const path = event.payload.path;
				if (!path) return;
				await importApp(path);
			},
		);

		return () => {
			unlistenPromise.then((unsub) => unsub()).catch(() => void 0);
		};
	}, [importApp]);

	useMobileHeader({
		right: menuActions,
		title: "Library",
	});

	const renderAppCards = (items: any[]) => {
		if (viewMode === "grid") {
			return (
				<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4 px-2">
					{items.map((meta) => (
						<div key={viewMode + meta.id} className="group w-full">
							<AppCard
								isOwned
								app={meta.app}
								metadata={meta as IMetadata}
								variant="extended"
								onClick={() => handleAppClick(meta.id)}
								onSettingsClick={() =>
									router.push(`/library/config?id=${meta.id}`)
								}
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
							isOwned
							app={meta.app}
							metadata={meta as IMetadata}
							variant="small"
							onClick={() => handleAppClick(meta.id)}
							className="w-full"
						/>
					</div>
				))}
			</div>
		);
	};

	return (
		<main className="flex flex-col w-full p-6 bg-gradient-to-br from-background to-muted/20 flex-1 min-h-0">
			{/* Header Section */}
			<div className="flex flex-col space-y-6 mb-8">
				<div className="hidden flex-col gap-4 sm:flex-row sm:items-center sm:justify-between md:flex">
					<div className="flex items-center space-x-3">
						<div className="p-1.5 sm:p-2 rounded-xl bg-primary/10 text-primary">
							<LibraryIcon className="h-6 w-6 sm:h-8 sm:w-8" />
						</div>
						<div>
							<h1 className="text-2xl sm:text-4xl leading-tight font-bold tracking-tight bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text">
								Library
							</h1>
							<p className="text-muted-foreground mt-1 text-sm sm:text-base">
								Manage and create your custom applications
							</p>
						</div>
					</div>
					<div className="flex flex-wrap items-center gap-2 w-full sm:w-auto">
						<Button
							size="lg"
							variant="outline"
							className="w-full sm:w-auto h-9 px-3 text-sm sm:h-11 sm:px-5 sm:text-base shadow-lg hover:shadow-xl transition-all duration-200 hidden md:flex"
							onClick={async () => {
								type Filter = { name: string; extensions: string[] };
								let filtersOption: Filter[] | undefined;
								// Use UA-based detection to avoid plugin availability issues
								const ua =
									typeof navigator !== "undefined"
										? navigator.userAgent.toLowerCase()
										: "";
								const isMobile = /android|iphone|ipad|ipod/.test(ua);
								filtersOption = isMobile
									? undefined
									: [
											{
												name: "Flow App",
												extensions: ["flow-app", "enc.flow-app"],
											},
										];

								const file = await open({
									multiple: false,
									directory: false,
									...(filtersOption ? { filters: filtersOption } : undefined),
								});
								if (!file) return;
								const path = String(file);
								if (path.toLowerCase().endsWith(".enc.flow-app")) {
									setEncryptedImportPath(path);
									setImportDialogOpen(true);
									return;
								}
								const toastId = toast.loading("Importing app...", {
									description: "Please wait.",
								});
								try {
									await invoke("import_app_from_file", { path });
									toast.success("App imported successfully!", { id: toastId });
									await apps.refetch();
								} catch (err) {
									console.error(err);
									toast.error("Failed to import app", { id: toastId });
								}
							}}
						>
							<ImportIcon className="mr-2 h-4 w-4" />
							Import App
						</Button>
						<Button
							size="lg"
							variant="outline"
							className="w-full sm:w-auto h-9 px-3 text-sm sm:h-11 sm:px-5 sm:text-base shadow-lg hover:shadow-xl transition-all duration-200 hidden md:flex"
							onClick={() => setJoinDialogOpen(true)}
						>
							<Link2 className="mr-2 h-4 w-4" />
							Join Project
						</Button>
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

				{/* Encrypted Import Dialog */}
				<ImportEncryptedDialog
					open={importDialogOpen}
					onOpenChange={(o) => {
						setImportDialogOpen(o);
						if (!o) setEncryptedImportPath(null);
					}}
					path={encryptedImportPath}
					onImported={async () => {
						await apps.refetch();
					}}
				/>

				{/* Search and Filter Bar */}
				<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between sm:space-x-4">
					<div className="flex items-center gap-3 sm:gap-4 flex-1 w-full">
						<div className="relative w-full sm:flex-1 sm:max-w-md">
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
						<a
							href="/library/visibility"
							className="text-sm text-primary hover:underline"
						>
							Missing Apps?{" "}
						</a>
					</div>
					<div className="flex items-center gap-2 w-full sm:w-auto justify-between sm:justify-end">
						<Select
							value={sortBy}
							onValueChange={(value: typeof sortBy) => setSortBy(value)}
						>
							<SelectTrigger className="w-full sm:w-[140px]">
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
						action={[
							{
								label: "Create Your First App",
								onClick: () => {
									useSpotlightStore.getState().open();
									useSpotlightStore.getState().setMode("quick-create");
								},
							},
							{
								label: "Missing Apps?",
								onClick: () => {
									router.push("/library/visibility");
								},
							},
						]}
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
