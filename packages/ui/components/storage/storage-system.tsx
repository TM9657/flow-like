"use client";

import {
	FilesIcon,
	FolderPlusIcon,
	GridIcon,
	LayoutGridIcon,
	LinkIcon,
	ListIcon,
	MaximizeIcon,
	MinimizeIcon,
	SearchIcon,
	SortAscIcon,
	UploadIcon,
	XIcon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { type IStorageItem, useBackend, useInvoke } from "../..";
import {
	Badge,
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	EmptyState,
	FilePreviewer,
	Input,
	Progress,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	Separator,
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../ui";
import { StorageBreadcrumbs } from "./storage-breadcrumbs";
import { FileOrFolder } from "./storage-file-or-folder";

export function StorageSystem({
	appId,
	prefix,
	updatePrefix,
	fileToUrl,
}: Readonly<{
	appId: string;
	prefix: string;
	updatePrefix: (prefix: string) => void;
	fileToUrl: (prefix: string) => Promise<string>;
}>) {
	// Responsive helper: detect small screens (<= Tailwind 'sm')
	const useIsSmallScreen = () => {
		const [isSmall, setIsSmall] = useState(false);
		useEffect(() => {
			if (typeof window === "undefined") return;
			const mql = window.matchMedia("(max-width: 640px)");
			const onChange = (e: MediaQueryListEvent) => setIsSmall(e.matches);
			const legacyOnChange = () => setIsSmall(mql.matches);
			// Set initial
			setIsSmall(mql.matches);
			// Subscribe (support older Safari)
			try {
				mql.addEventListener("change", onChange);
			} catch {
				// Fallback for Safari < 14
				// @ts-ignore
				mql.addListener(legacyOnChange);
			}
			return () => {
				try {
					mql.removeEventListener("change", onChange);
				} catch {
					// Fallback for Safari < 14
					// @ts-ignore
					mql.removeListener(legacyOnChange);
				}
			};
		}, []);
		return isSmall;
	};
	const isSmallScreen = useIsSmallScreen();
	const fileReference = useRef<HTMLInputElement>(null);
	const folderReference = useRef<HTMLInputElement>(null);
	const backend = useBackend();
	const [preview, setPreview] = useState({
		url: "",
		file: "",
	});
	const [uploadProgress, setUploadProgress] = useState<{
		isUploading: boolean;
		progress: number;
		fileCount: number;
		currentFile: string;
	}>({
		isUploading: false,
		progress: 0,
		fileCount: 0,
		currentFile: "",
	});
	const files = useInvoke(
		backend.storageState.listStorageItems,
		backend.storageState,
		[appId, prefix],
	);

	// ---------- Virtual folders (sessionStorage) ----------
	const [creatingFolder, setCreatingFolder] = useState(false);
	const [newFolderName, setNewFolderName] = useState("");
	const [virtualFoldersHere, setVirtualFoldersHere] = useState<string[]>([]);

	const storeKey = useMemo(() => `vfolders:${appId}`, [appId]);
	const normalizePrefix = useCallback(
		(p: string) => p.replace(/^\/+|\/+$/g, ""),
		[],
	);
	const currentParentKey = useMemo(
		() => normalizePrefix(prefix),
		[prefix, normalizePrefix],
	);

	type VFMap = Record<string, string[]>; // parentPrefix -> [childFolderNames]
	const readVF = useCallback((): VFMap => {
		try {
			const raw = sessionStorage.getItem(storeKey);
			return raw ? (JSON.parse(raw) as VFMap) : {};
		} catch {
			return {};
		}
	}, [storeKey]);
	const writeVF = useCallback(
		(map: VFMap) => {
			try {
				sessionStorage.setItem(storeKey, JSON.stringify(map));
			} catch {
				// ignore
			}
		},
		[storeKey],
	);

	useEffect(() => {
		const map = readVF();
		setVirtualFoldersHere(map[currentParentKey] ?? []);
	}, [currentParentKey, readVF]);

	const addVirtualFolder = useCallback(
		(name: string) => {
			const clean = name.trim();
			if (!clean) {
				toast.error("Folder name cannot be empty");
				return false;
			}
			if (/^[.]{1,2}$/.test(clean)) {
				toast.error("Reserved name");
				return false;
			}
			if (/[\\\/:*?"<>|]/.test(clean)) {
				toast.error("Invalid characters in name");
				return false;
			}

			// check duplicates against visible folders (backend + virtual)
			const existingFolderNames = new Set(
				(files.data ?? [])
					.filter((f) => f.is_dir)
					.map((f) => (f.location.split("/").pop() ?? "").toLowerCase()),
			);
			for (const v of virtualFoldersHere)
				existingFolderNames.add(v.toLowerCase());
			if (existingFolderNames.has(clean.toLowerCase())) {
				toast.error("A folder with that name already exists");
				return false;
			}

			const all = readVF();
			const next = new Set(all[currentParentKey] ?? []);
			next.add(clean);
			all[currentParentKey] = Array.from(next);
			writeVF(all);
			setVirtualFoldersHere(all[currentParentKey]);
			toast.success("Folder created");
			return true;
		},
		[files.data, virtualFoldersHere, currentParentKey, readVF, writeVF],
	);

	// Merge backend items with virtual folders for current prefix
	const filesWithVirtual = useMemo<IStorageItem[]>(() => {
		const base = (files.data ?? []).slice();
		const have = new Set(base.map((f) => f.location));
		const basePrefixNorm = normalizePrefix(prefix);
		const locFor = (name: string) =>
			basePrefixNorm ? `${basePrefixNorm}/${name}` : name;
		const virtualItems: IStorageItem[] = virtualFoldersHere
			.filter((name) => !have.has(locFor(name)))
			.map(
				(name) =>
					({
						location: locFor(name),
						is_dir: true,
						size: 0,
						last_modified: new Date().toISOString(),
					}) as IStorageItem,
			);
		return [...base, ...virtualItems];
	}, [files.data, virtualFoldersHere, prefix, normalizePrefix]);

	const [searchQuery, setSearchQuery] = useState("");
	const [viewMode, setViewMode] = useState<"grid" | "list">("list");
	const [sortBy, setSortBy] = useState<"name" | "date" | "size" | "type">(
		"name",
	);
	const [sortOrder, setSortOrder] = useState<"asc" | "desc">("asc");
	const [isPreviewMaximized, setIsPreviewMaximized] = useState(false);

	const processFiles = useCallback(
		async (inputFiles: File[]) => {
			if (inputFiles.length === 0) return;
			const fileList = Array.from(inputFiles);

			setUploadProgress({
				isUploading: true,
				progress: 0,
				fileCount: fileList.length,
				currentFile: fileList[0]?.name || "",
			});

			try {
				await backend.storageState.uploadStorageItems(
					appId,
					prefix,
					fileList,
					(progress) => {
						setUploadProgress((prev) => ({
							...prev,
							progress: progress,
						}));
					},
				);

				setUploadProgress({
					isUploading: false,
					progress: 100,
					fileCount: 0,
					currentFile: "",
				});

				toast.success("Files uploaded successfully");
				files.refetch();
			} catch (error) {
				console.error(error);
				setUploadProgress({
					isUploading: false,
					progress: 0,
					fileCount: 0,
					currentFile: "",
				});
				toast.error("Failed to upload files");
			}
		},
		[prefix, backend, files.refetch],
	);

	const loadFile = useCallback(
		async (file: string) => {
			if (preview.file === file) {
				setPreview((old) => ({ ...old, file: "", url: "" }));
				return;
			}

			const url = await backend.storageState.downloadStorageItems(appId, [
				file,
			]);

			if (url.length === 0 || !url[0]?.url) {
				toast.error("Failed to load file preview");
				return;
			}

			const fileUrl = url[0].url;

			setPreview({
				url: fileUrl,
				file,
			});
		},
		[appId, preview],
	);

	const downloadFile = useCallback(
		async (file: string) => {
			if (preview.file === file) {
				setPreview((old) => ({ ...old, file: "", url: "" }));
				return;
			}

			const signedUrl = await backend.storageState.downloadStorageItems(appId, [
				file,
			]);

			if (signedUrl.length === 0 || !signedUrl[0]?.url) {
				toast.error("Failed to load file preview");
				return;
			}

			if (backend.storageState.writeStorageItems) {
				await backend.storageState.writeStorageItems(signedUrl);
				return;
			}

			const fileUrl = signedUrl[0].url;
			const fileName =
				fileUrl.split("/").pop()?.split("?")[0] || "downloaded_file";
			const fileContent = await fetch(fileUrl).then((res) => res.blob());
			const blob = new Blob([fileContent], {
				type: "application/octet-stream",
			});
			const url = URL.createObjectURL(blob);
			const a = document.createElement("a");
			a.href = url;
			a.download = fileName;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		},
		[appId, preview],
	);

	const filteredFiles = useMemo(
		() =>
			filesWithVirtual?.filter((file) =>
				file.location
					.split("/")
					.pop()
					?.toLowerCase()
					.includes(searchQuery.toLowerCase()),
			) ?? [],
		[filesWithVirtual, searchQuery],
	);

	const sortedFiles = useMemo(
		() =>
			[...filteredFiles].sort((a, b) => {
				const getName = (file: IStorageItem) =>
					file.location.split("/").pop() ?? "";
				const isFolder = (file: IStorageItem) => file.is_dir;

				// Always sort folders first
				if (isFolder(a) && !isFolder(b)) return -1;
				if (!isFolder(a) && isFolder(b)) return 1;

				let comparison = 0;

				switch (sortBy) {
					case "name":
						comparison = getName(a).localeCompare(getName(b));
						break;
					case "date":
						comparison =
							new Date(a.last_modified ?? 0).getTime() -
							new Date(b.last_modified ?? 0).getTime();
						break;
					case "size":
						comparison = (a.size ?? 0) - (b.size ?? 0);
						break;
					case "type": {
						const extA = getName(a).split(".").pop() ?? "";
						const extB = getName(b).split(".").pop() ?? "";
						comparison = extA.localeCompare(extB);
						break;
					}
				}

				return sortOrder === "asc" ? comparison : -comparison;
			}),
		[filteredFiles, sortBy, sortOrder],
	);

	const fileCount = filesWithVirtual?.filter((f) => !f.is_dir).length ?? 0;
	const folderCount = filesWithVirtual?.filter((f) => f.is_dir).length ?? 0;

	return (
		<div className="flex grow flex-col gap-4 min-h-full h-full max-h-full overflow-hidden w-full">
			<input
				ref={fileReference}
				type="file"
				className="hidden"
				id="file-upload"
				multiple
				onChange={(e) => {
					if (!e.target.files) return;
					const filesArray = Array.from(e.target.files);
					processFiles(filesArray);
					e.target.value = "";
				}}
			/>

			<input
				ref={folderReference}
				type="file"
				className="hidden"
				id="folder-upload"
				// @ts-ignore
				webkitdirectory={"true"}
				directory
				multiple
				onChange={(e) => {
					if (!e.target.files) return;
					const filesArray = Array.from(e.target.files);
					processFiles(filesArray);
					e.target.value = "";
				}}
			/>

			{/* Upload Progress Indicator */}
			{uploadProgress.isUploading && (
				<div className="mx-4 mt-4 p-4 border rounded-lg bg-card">
					<div className="flex items-center justify-between mb-2">
						<div className="flex items-center gap-2">
							<UploadIcon className="h-4 w-4 text-primary animate-pulse" />
							<span className="text-sm font-medium">
								Uploading {uploadProgress.fileCount} file
								{uploadProgress.fileCount !== 1 ? "s" : ""}
							</span>
						</div>
						<span className="text-sm text-muted-foreground">
							{uploadProgress.progress.toFixed(2)}%
						</span>
					</div>
					<Progress value={uploadProgress.progress} className="mb-2" />
					<p className="text-xs text-muted-foreground truncate">
						{uploadProgress.currentFile}
					</p>
				</div>
			)}

			{/* Header Section */}
			<div className="flex flex-col gap-4 px-4 pt-4">
				<div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
					<h2 className="text-2xl font-semibold tracking-tight">Storage</h2>
					<div className="flex items-center gap-2 flex-wrap justify-end">
						<div className="hidden sm:flex items-center gap-2">
							<div className="flex items-center gap-1 text-sm text-muted-foreground">
								<span>Sort by:</span>
								<span className="font-medium text-foreground capitalize">
									{sortBy === "date" ? "Date modified" : sortBy}
								</span>
								<span className="text-xs">
									{sortOrder === "asc" ? "↑" : "↓"}
								</span>
							</div>
							<DropdownMenu>
								<DropdownMenuTrigger asChild>
									<Button variant="outline" size="icon">
										<SortAscIcon className="h-4 w-4" />
									</Button>
								</DropdownMenuTrigger>
								<DropdownMenuContent align="end">
									<DropdownMenuItem
										onClick={() => {
											setSortBy("name");
											setSortOrder(
												sortBy === "name" && sortOrder === "asc"
													? "desc"
													: "asc",
											);
										}}
										className="flex items-center justify-between"
									>
										Name
										{sortBy === "name" && (
											<span className="text-xs text-muted-foreground">
												{sortOrder === "asc" ? " ↑" : " ↓"}
											</span>
										)}
									</DropdownMenuItem>
									<DropdownMenuItem
										onClick={() => {
											setSortBy("date");
											setSortOrder(
												sortBy === "date" && sortOrder === "asc"
													? "desc"
													: "asc",
											);
										}}
										className="flex items-center justify-between"
									>
										Date modified
										{sortBy === "date" && (
											<span className="text-xs text-muted-foreground">
												{sortOrder === "asc" ? " ↑" : " ↓"}
											</span>
										)}
									</DropdownMenuItem>
									<DropdownMenuItem
										onClick={() => {
											setSortBy("size");
											setSortOrder(
												sortBy === "size" && sortOrder === "asc"
													? "desc"
													: "asc",
											);
										}}
										className="flex items-center justify-between"
									>
										Size
										{sortBy === "size" && (
											<span className="text-xs text-muted-foreground">
												{sortOrder === "asc" ? " ↑" : " ↓"}
											</span>
										)}
									</DropdownMenuItem>
									<DropdownMenuItem
										onClick={() => {
											setSortBy("type");
											setSortOrder(
												sortBy === "type" && sortOrder === "asc"
													? "desc"
													: "asc",
											);
										}}
										className="flex items-center justify-between"
									>
										Type
										{sortBy === "type" && (
											<span className="text-xs text-muted-foreground">
												{sortOrder === "asc" ? " ↑" : " ↓"}
											</span>
										)}
									</DropdownMenuItem>
								</DropdownMenuContent>
							</DropdownMenu>
						</div>

						{/* View toggle */}
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="outline"
									size="icon"
									onClick={() =>
										setViewMode(viewMode === "grid" ? "list" : "grid")
									}
								>
									{viewMode === "grid" ? (
										<ListIcon className="h-4 w-4" />
									) : (
										<GridIcon className="h-4 w-4" />
									)}
								</Button>
							</TooltipTrigger>
							<TooltipContent>
								Switch to {viewMode === "grid" ? "list" : "grid"} view
							</TooltipContent>
						</Tooltip>
						<Separator orientation="vertical" className="h-6" />

						{/* New virtual folder */}
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="outline"
									className="gap-2"
									onClick={() => setCreatingFolder((v) => !v)}
								>
									<FolderPlusIcon className="h-4 w-4" />
									<span className="hidden sm:inline">New Folder</span>
								</Button>
							</TooltipTrigger>
							<TooltipContent>
								Create a virtual folder (session only)
							</TooltipContent>
						</Tooltip>

						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="outline"
									className="gap-2"
									onClick={() => fileReference.current?.click()}
								>
									<UploadIcon className="h-4 w-4" />
									<span className="hidden sm:inline">Upload Files</span>
								</Button>
							</TooltipTrigger>
							<TooltipContent>Upload files to current folder</TooltipContent>
						</Tooltip>

						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="outline"
									className="gap-2"
									onClick={() => folderReference.current?.click()}
								>
									<FolderPlusIcon className="h-4 w-4" />
									<span className="hidden sm:inline">Upload Folder</span>
								</Button>
							</TooltipTrigger>
							<TooltipContent>Upload entire folder</TooltipContent>
						</Tooltip>
					</div>
				</div>

				<div className="flex flex-col sm:flex-row sm:items-end gap-2 mt-2 sm:justify-between">
					<div className="overflow-x-auto whitespace-nowrap max-w-full">
						<StorageBreadcrumbs
							appId={appId}
							prefix={prefix}
							updatePrefix={(prefix) => updatePrefix(prefix)}
						/>
					</div>
					{(filesWithVirtual.length ?? 0) > 0 && (
						<div className="relative w-full sm:w-auto">
							<SearchIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
							<Input
								placeholder="Search files and folders..."
								className="pl-10 w-full"
								value={searchQuery}
								onChange={(e) => setSearchQuery(e.target.value)}
							/>
						</div>
					)}
				</div>

				{/* Inline create folder row */}
				{creatingFolder && (
					<div className="flex items-center gap-2 px-4">
						<Input
							placeholder="Folder name"
							value={newFolderName}
							onChange={(e) => setNewFolderName(e.target.value)}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									if (addVirtualFolder(newFolderName)) {
										setNewFolderName("");
										setCreatingFolder(false);
									}
								}
								if (e.key === "Escape") {
									setCreatingFolder(false);
									setNewFolderName("");
								}
							}}
						/>
						<Button
							variant="default"
							onClick={() => {
								if (addVirtualFolder(newFolderName)) {
									setNewFolderName("");
									setCreatingFolder(false);
								}
							}}
						>
							Create
						</Button>
						<Button
							variant="ghost"
							onClick={() => {
								setCreatingFolder(false);
								setNewFolderName("");
							}}
						>
							Cancel
						</Button>
					</div>
				)}
			</div>

			<Separator />

			{/* Content Section */}
			{(filesWithVirtual.length ?? 0) === 0 && (
				<div className="flex flex-col h-full w-full grow relative px-4">
					<EmptyState
						className="w-full h-full max-w-full border-2 border-dashed border-muted-foreground/25 rounded-lg"
						title="No Files Found"
						description="Get started by creating a folder or uploading your first files to this storage space"
						action={[
							{
								label: "New Folder",
								onClick: () => setCreatingFolder(true),
							},
							{
								label: "Upload Files",
								onClick: () => fileReference.current?.click(),
							},
							{
								label: "Upload Folder",
								onClick: () => folderReference.current?.click(),
							},
						]}
						icons={[LayoutGridIcon, FilesIcon, LinkIcon]}
					/>
				</div>
			)}

			{(filesWithVirtual.length ?? 0) > 0 && (
				<div className="flex flex-col gap-4 grow max-h-full h-full overflow-y-hidden px-4 pb-4">
					{preview.url !== "" && (
						<>
							{(isSmallScreen || isPreviewMaximized) && (
								<div className="fixed inset-0 z-50 bg-background">
									<div className="flex flex-col h-full w-full">
										<div className="p-4 border-b bg-background flex items-center justify-between">
											<h3 className="font-medium text-lg">
												Preview - {preview.file.split("/").pop()}
											</h3>
											<Button
												variant="ghost"
												size="sm"
												onClick={() => {
													if (isSmallScreen) {
														setPreview((p) => ({ ...p, url: "", file: "" }));
													} else {
														setIsPreviewMaximized(false);
													}
												}}
												className="h-8 w-8 p-0"
											>
												{isSmallScreen ? (
													<XIcon className="h-4 w-4" />
												) : (
													<MinimizeIcon className="h-4 w-4" />
												)}
											</Button>
										</div>
										<div className="grow overflow-auto">
											<FilePreviewer url={preview.url} page={2} />
										</div>
									</div>
								</div>
							)}
							{!isSmallScreen && !isPreviewMaximized && (
								<ResizablePanelGroup
									direction="horizontal"
									autoSaveId={"file_viewer"}
									className="border rounded-lg"
								>
									<ResizablePanel className="flex flex-col gap-2 grow overflow-y-hidden max-h-full h-full p-4 bg-background">
										<div
											key={sortBy}
											className="flex flex-col grow max-h-full h-full overflow-hidden gap-2"
										>
											<div className="flex items-center gap-2 mb-2">
												<h3 className="font-medium text-sm text-muted-foreground">
													Files & Folders
												</h3>
												<Badge
													variant="secondary"
													className="px-2 py-1 text-xs"
												>
													{fileCount} files
												</Badge>
												<Badge
													variant="secondary"
													className="px-2 py-1 text-xs"
												>
													{folderCount} folders
												</Badge>
											</div>
											<div className="flex flex-col gap-2 grow max-h-full h-full overflow-auto">
												{sortedFiles.map((file) => (
													<FileOrFolder
														highlight={preview.file === file.location}
														key={file.location}
														file={file}
														changePrefix={(new_prefix) =>
															updatePrefix(`${prefix}/${new_prefix}`)
														}
														loadFile={(file) => loadFile(file)}
														deleteFile={async (file) => {
															const filePrefix = `${prefix}/${file}`;
															await backend.storageState.deleteStorageItems(
																appId,
																[filePrefix],
															);
															await files.refetch();
															toast.success("File deleted successfully");
														}}
														shareFile={async (file) => {
															const downloadLinks =
																await backend.storageState.downloadStorageItems(
																	appId,
																	[file],
																);
															if (downloadLinks.length === 0) {
																return;
															}

															const firstItem = downloadLinks[0];
															if (!firstItem?.url) {
																return;
															}

															try {
																await navigator.clipboard.writeText(
																	firstItem.url,
																);
																toast.success(
																	"Copied download link to clipboard",
																);
															} catch (error) {
																console.error(
																	"Failed to copy link to clipboard:",
																	error,
																);
															}
														}}
														downloadFile={async (file) => {
															downloadFile(file);
														}}
													/>
												))}
											</div>
										</div>
									</ResizablePanel>
									<ResizableHandle className="mx-2" />
									<ResizablePanel className="flex flex-col gap-2 grow overflow-y-hidden max-h-full h-full p-4 bg-background">
										<div className="flex flex-col grow overflow-auto max-h-full h-full bg-muted/50 rounded-md border">
											<div className="p-2 border-b bg-background rounded-t-md flex items-center justify-between">
												<h3 className="font-medium text-sm">Preview</h3>
												<Button
													variant="ghost"
													size="sm"
													onClick={() => setIsPreviewMaximized(true)}
													className="h-6 w-6 p-0"
												>
													<MaximizeIcon className="h-3 w-3" />
												</Button>
											</div>
											<div className="grow overflow-auto">
												<FilePreviewer url={preview.url} page={2} />
											</div>
										</div>
									</ResizablePanel>
								</ResizablePanelGroup>
							)}
						</>
					)}
					{preview.url === "" && (
						<div className="flex flex-col grow max-h-full h-full overflow-auto gap-2 border rounded-lg p-3 sm:p-4 bg-background">
							<div className="flex items-center gap-2 mb-2">
								<h3 className="font-medium text-sm text-muted-foreground">
									Files & Folders
								</h3>
								<Badge variant="secondary" className="px-2 py-1 text-xs">
									{fileCount} files
								</Badge>
								<Badge variant="secondary" className="px-2 py-1 text-xs">
									{folderCount} folders
								</Badge>
							</div>
							<div
								className={`grid gap-2 ${viewMode === "grid" ? "grid-cols-2 md:grid-cols-3 lg:grid-cols-4" : "grid-cols-1"}`}
							>
								{sortedFiles.map((file) => (
									<FileOrFolder
										highlight={preview.file === file.location}
										key={file.location}
										file={file}
										changePrefix={(new_prefix) => {
											setPreview({ url: "", file: "" });
											updatePrefix(`${prefix}/${new_prefix}`);
										}}
										loadFile={loadFile}
										deleteFile={async (file) => {
											const filePrefix = `${prefix}/${file}`;
											await backend.storageState.deleteStorageItems(appId, [
												filePrefix,
											]);
											await files.refetch();
											toast.success("File deleted successfully");
										}}
										shareFile={async (file) => {
											const downloadLinks =
												await backend.storageState.downloadStorageItems(appId, [
													file,
												]);
											if (downloadLinks.length === 0) {
												return;
											}
											const firstItem = downloadLinks[0];
											if (!firstItem?.url) {
												return;
											}
											try {
												await navigator.clipboard.writeText(firstItem.url);
												toast.success("Copied download link to clipboard");
											} catch (error) {
												console.error(
													"Failed to copy link to clipboard:",
													error,
												);
											}
										}}
										downloadFile={async (file) => {
											downloadFile(file);
										}}
									/>
								))}
							</div>
						</div>
					)}
				</div>
			)}
		</div>
	);
}
