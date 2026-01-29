import {
	CheckIcon,
	Download,
	ExternalLink,
	FileText,
	FilterIcon,
	GlobeIcon,
	GridIcon,
	ImageIcon,
	ListIcon,
	MaximizeIcon,
	MinimizeIcon,
	Music,
	SearchIcon,
	SortAscIcon,
	VideoIcon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { humanFileSize } from "../../../lib";
import {
	Badge,
	Button,
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
	Input,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	Separator,
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../../ui";
import { FilePreviewer, canPreview, isCode, isText } from "../../ui/file-previewer";
import type { ProcessedAttachment } from "./attachment";

const getDisplayFileName = (name: string) => {
	try {
		const decoded = decodeURIComponent(name);
		const parts = decoded.split(/[/\\]/);
		return parts[parts.length - 1];
	} catch {
		return name;
	}
};

interface FileDialogProps {
	files: ProcessedAttachment[];
	handleFileClick: (file: ProcessedAttachment) => void;
	open?: boolean;
	onOpenChange?: (open: boolean) => void;
	initialSelectedFile?: ProcessedAttachment | null;
	trigger?: React.ReactNode;
}

export const canPreviewFile = (file: ProcessedAttachment) => {
	return canPreview(file.url);
};

export async function downloadFile(file: ProcessedAttachment): Promise<void> {
	const isTauriEnv = typeof window !== "undefined" &&
		!!((window as any).__TAURI__ || (window as any).__TAURI_IPC__ || (window as any).__TAURI_INTERNALS__);

	if (isTauriEnv) {
		try {
			const { save } = await import("@tauri-apps/plugin-dialog");
			const { writeFile } = await import("@tauri-apps/plugin-fs");

			const response = await fetch(file.url);
			const blob = await response.blob();
			const arrayBuffer = await blob.arrayBuffer();

			const filePath = await save({
				defaultPath: file.name,
				filters: [{ name: "All Files", extensions: ["*"] }],
			});

			if (filePath) {
				await writeFile(filePath, new Uint8Array(arrayBuffer));
			}
			return;
		} catch (e) {
			console.warn("Tauri save failed, falling back to browser download", e);
		}
	}

	// Browser fallback - fetch and trigger download
	try {
		const response = await fetch(file.url);
		const blob = await response.blob();
		const blobUrl = URL.createObjectURL(blob);

		const link = document.createElement("a");
		link.href = blobUrl;
		link.download = file.name || "file";
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
		URL.revokeObjectURL(blobUrl);
	} catch (e) {
		// Ultimate fallback - open in new tab
		window.open(file.url, "_blank", "noopener,noreferrer");
	}
}

export const getFileIcon = (type: ProcessedAttachment["type"]) => {
	switch (type) {
		case "image":
			return <ImageIcon className="w-4 h-4" />;
		case "video":
			return <VideoIcon className="w-4 h-4" />;
		case "audio":
			return <Music className="w-4 h-4" />;
		case "pdf":
			return <FileText className="w-4 h-4" />;
		case "document":
			return <FileText className="w-4 h-4" />;
		case "website":
			return <GlobeIcon className="w-4 h-4" />;
		default:
			return <Download className="w-4 h-4" />;
	}
};

export function FileDialog({
	files,
	handleFileClick,
	open: controlledOpen,
	onOpenChange,
	initialSelectedFile,
	trigger,
}: Readonly<FileDialogProps>) {
	const [internalOpen, setInternalOpen] = useState(false);
	const [searchQuery, setSearchQuery] = useState("");
	const [viewMode, setViewMode] = useState<"grid" | "list">("list");
	const [sortBy, setSortBy] = useState<"name" | "type" | "size">("name");
	const [sortOrder, setSortOrder] = useState<"asc" | "desc">("asc");
	const [filterType, setFilterType] = useState<
		ProcessedAttachment["type"] | "all"
	>("all");
	const [selectedFile, setSelectedFile] = useState<ProcessedAttachment | null>(
		initialSelectedFile ?? null,
	);
	const [isPreviewMaximized, setIsPreviewMaximized] = useState(false);

	const isControlled = controlledOpen !== undefined;
	const isOpen = isControlled ? controlledOpen : internalOpen;

	const handleOpenChange = useCallback(
		(open: boolean) => {
			if (!isControlled) {
				setInternalOpen(open);
			}
			onOpenChange?.(open);
			if (open && initialSelectedFile) {
				setSelectedFile(initialSelectedFile);
			}
		},
		[isControlled, onOpenChange, initialSelectedFile],
	);

	// Update selected file when initialSelectedFile changes
	useEffect(() => {
		if (initialSelectedFile) {
			setSelectedFile(initialSelectedFile);
		}
	}, [initialSelectedFile]);

	const filteredFiles = useMemo(() => {
		return files.filter((file) => {
			const matchesSearch =
				file.name?.toLowerCase().includes(searchQuery.toLowerCase()) ?? true;
			const matchesType = filterType === "all" || file.type === filterType;
			return matchesSearch && matchesType;
		});
	}, [files, searchQuery, filterType]);

	const sortedFiles = useMemo(() => {
		return [...filteredFiles].sort((a, b) => {
			let comparison = 0;

			switch (sortBy) {
				case "name":
					comparison = (a.name ?? "").localeCompare(b.name ?? "");
					break;
				case "type":
					comparison = a.type.localeCompare(b.type);
					break;
				case "size":
					comparison = (a.size ?? 0) - (b.size ?? 0);
					break;
			}

			return sortOrder === "asc" ? comparison : -comparison;
		});
	}, [filteredFiles, sortBy, sortOrder]);

	const canPreview = useCallback((file: ProcessedAttachment) => {
		return canPreviewFile(file);
	}, []);

	const fileTypeCount = useMemo(() => {
		const counts: Record<string, number> = {};
		filteredFiles.forEach((file) => {
			counts[file.type] = (counts[file.type] || 0) + 1;
		});
		return counts;
	}, [filteredFiles]);

	const handleFileSelect = useCallback(
		(file: ProcessedAttachment) => {
			if (canPreview(file)) {
				setSelectedFile(selectedFile?.url === file.url ? null : file);
			} else {
				handleFileClick(file);
			}
		},
		[selectedFile, canPreview, handleFileClick],
	);

	const defaultTrigger = (
		<button className="text-muted-foreground hover:text-foreground transition-colors">
			<Badge
				variant="secondary"
				className="cursor-pointer hover:bg-secondary/80 transition-colors gap-1 text-xs h-6 rounded-full"
			>
				<FileText className="w-3 h-3" />
				{files.length}
			</Badge>
		</button>
	);

	return (
		<TooltipProvider>
			<div>
				<Dialog open={isOpen} onOpenChange={handleOpenChange}>
					{trigger !== null && (
						<DialogTrigger asChild>{trigger ?? defaultTrigger}</DialogTrigger>
					)}
					<DialogContent className="min-w-[calc(100dvw-5rem)] min-h-[calc(100dvh-5rem)] max-w-[calc(100dvw-5rem)] max-h-[calc(100dvh-5rem)] overflow-hidden flex flex-col">
						<DialogHeader>
							<DialogTitle className="flex items-center gap-2">
								<FileText className="w-4 h-4" />
								References ({files.length})
							</DialogTitle>
						</DialogHeader>

						{/* Header Controls */}
						<div className="flex flex-col gap-4">
							<div className="flex flex-row items-center justify-between">
								<div className="flex flex-col gap-1">
									<div className="flex items-center gap-2 text-sm text-muted-foreground">
										<Badge variant="secondary" className="px-2 py-1">
											{filteredFiles.length} files
										</Badge>
										{filterType !== "all" && (
											<Badge variant="default" className="px-2 py-1 capitalize">
												Filter: {filterType}
											</Badge>
										)}
										{Object.entries(fileTypeCount).map(([type, count]) => (
											<Badge
												key={type}
												variant="outline"
												className="px-2 py-1 capitalize"
											>
												{count} {type}
											</Badge>
										))}
									</div>
								</div>

								<div className="flex items-center gap-2">
									{/* Sort Controls */}
									<div className="flex items-center gap-2">
										<div className="flex items-center gap-1 text-sm text-muted-foreground">
											<span>Sort by:</span>
											<span className="font-medium text-foreground capitalize">
												{sortBy}
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
											</DropdownMenuContent>
										</DropdownMenu>
									</div>

									{/* View Mode Toggle */}
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

									{/* Filter by Type */}
									<DropdownMenu>
										<DropdownMenuTrigger asChild>
											<Button variant="outline" size="icon">
												<FilterIcon className="h-4 w-4" />
											</Button>
										</DropdownMenuTrigger>
										<DropdownMenuContent align="end">
											<DropdownMenuLabel>Filter by Type</DropdownMenuLabel>
											<DropdownMenuSeparator />
											<DropdownMenuItem onClick={() => setFilterType("all")}>
												{filterType === "all" && (
													<CheckIcon className="w-4 h-4 mr-2" />
												)}
												All Files
											</DropdownMenuItem>
											<DropdownMenuItem onClick={() => setFilterType("image")}>
												{filterType === "image" && (
													<CheckIcon className="w-4 h-4 mr-2" />
												)}
												Images
											</DropdownMenuItem>
											<DropdownMenuItem onClick={() => setFilterType("video")}>
												{filterType === "video" && (
													<CheckIcon className="w-4 h-4 mr-2" />
												)}
												Videos
											</DropdownMenuItem>
											<DropdownMenuItem onClick={() => setFilterType("audio")}>
												{filterType === "audio" && (
													<CheckIcon className="w-4 h-4 mr-2" />
												)}
												Audio
											</DropdownMenuItem>
											<DropdownMenuItem onClick={() => setFilterType("pdf")}>
												{filterType === "pdf" && (
													<CheckIcon className="w-4 h-4 mr-2" />
												)}
												PDFs
											</DropdownMenuItem>
											<DropdownMenuItem
												onClick={() => setFilterType("document")}
											>
												{filterType === "document" && (
													<CheckIcon className="w-4 h-4 mr-2" />
												)}
												Documents
											</DropdownMenuItem>
											<DropdownMenuItem
												onClick={() => setFilterType("website")}
											>
												{filterType === "website" && (
													<CheckIcon className="w-4 h-4 mr-2" />
												)}
												Websites
											</DropdownMenuItem>
										</DropdownMenuContent>
									</DropdownMenu>
								</div>
							</div>

							{/* Search */}
							<div className="relative">
								<SearchIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
								<Input
									placeholder="Search files..."
									className="pl-10"
									value={searchQuery}
									onChange={(e) => setSearchQuery(e.target.value)}
								/>
							</div>
						</div>

						<Separator />

						{/* Content Section */}
						<div className="flex flex-col gap-4 flex-grow max-h-full h-full overflow-hidden">
							{isPreviewMaximized && selectedFile && (
								<div className="fixed inset-0 z-50 bg-background flex flex-grow flex-col h-full">
									<div className="p-4 border-b bg-background flex items-center justify-between">
										<h3 className="font-medium text-lg">
											Preview - {getDisplayFileName(selectedFile.name)}
										</h3>
										<Button
											variant="ghost"
											size="sm"
											onClick={() => setIsPreviewMaximized(false)}
											className="h-8 w-8 p-0"
										>
											<MinimizeIcon className="h-4 w-4" />
										</Button>
									</div>
									<div className="flex flex-col flex-grow overflow-auto h-full min-h-full">
										<FileDialogPreview file={selectedFile} maximized={true} />
									</div>
								</div>
							)}

							{!isPreviewMaximized && selectedFile && (
								<ResizablePanelGroup
									direction="horizontal"
									autoSaveId="attachment_viewer"
									className="border rounded-lg flex-1 min-h-0"
								>
									<ResizablePanel
										defaultSize={60}
										className="flex flex-col gap-2 overflow-hidden p-4 bg-background"
									>
										<div className="flex flex-col flex-1 min-h-0 gap-2">
											<h3 className="font-medium text-sm text-muted-foreground mb-2 flex-shrink-0">
												Files & References
											</h3>
											<div className="flex flex-col gap-2 flex-1 min-h-0 overflow-auto">
												<FileList
													files={sortedFiles}
													viewMode={viewMode}
													selectedFile={selectedFile}
													onFileSelect={handleFileSelect}
													handleFileClick={handleFileClick}
													canPreview={canPreview}
												/>
											</div>
										</div>
									</ResizablePanel>
									<ResizableHandle className="mx-2" />
									<ResizablePanel
										defaultSize={40}
										className="flex flex-col gap-2 p-4 bg-background min-h-0"
									>
										<div className="flex flex-col flex-1 min-h-0 bg-muted/50 rounded-md border">
											<div className="p-2 border-b bg-background rounded-t-md flex items-center justify-between flex-shrink-0">
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
											<div className="flex-1 min-h-0 overflow-hidden">
												<FileDialogPreview
													key={selectedFile.url}
													file={selectedFile}
												/>
											</div>
										</div>
									</ResizablePanel>
								</ResizablePanelGroup>
							)}

							{!selectedFile && (
								<div className="flex flex-col flex-grow overflow-auto gap-2 border rounded-lg p-4 bg-background">
									<h3 className="font-medium text-sm text-muted-foreground mb-2">
										Files & References
									</h3>
									<FileList
										files={sortedFiles}
										viewMode={viewMode}
										selectedFile={null}
										onFileSelect={handleFileSelect}
										handleFileClick={handleFileClick}
										canPreview={canPreview}
									/>
								</div>
							)}
						</div>
					</DialogContent>
				</Dialog>
			</div>
		</TooltipProvider>
	);
}

interface FileListProps {
	files: ProcessedAttachment[];
	viewMode: "grid" | "list";
	selectedFile: ProcessedAttachment | null;
	onFileSelect: (file: ProcessedAttachment) => void;
	handleFileClick: (file: ProcessedAttachment) => void;
	canPreview: (file: ProcessedAttachment) => boolean;
}

function FileList({
	files,
	viewMode,
	selectedFile,
	onFileSelect,
	handleFileClick,
	canPreview,
}: FileListProps) {
	return (
		<div
			className={`grid gap-2 ${viewMode === "grid" ? "grid-cols-2 md:grid-cols-3 lg:grid-cols-4" : "grid-cols-1"}`}
		>
			{files.map((file, index) => (
				<FileItem
					grid={viewMode === "grid"}
					key={index}
					file={file}
					isSelected={selectedFile?.url === file.url}
					onSelect={onFileSelect}
					handleFileClick={handleFileClick}
					canPreview={canPreview}
				/>
			))}
		</div>
	);
}

interface FileItemProps {
	grid: boolean;
	file: ProcessedAttachment;
	isSelected: boolean;
	onSelect: (file: ProcessedAttachment) => void;
	handleFileClick: (file: ProcessedAttachment) => void;
	canPreview: (file: ProcessedAttachment) => boolean;
}

function FileItem({
	grid,
	file,
	isSelected,
	onSelect,
	handleFileClick,
	canPreview,
}: Readonly<FileItemProps>) {
	if (grid) {
		return (
			<div
				className={`group relative rounded-lg border border-border/50 p-2 w-full transition-all duration-200 bg-gradient-to-r from-background to-muted/10 ${
					isSelected ? "border-primary bg-primary/5 shadow-sm" : ""
				} ${
					canPreview(file)
						? "hover:border-primary/50 hover:shadow-md cursor-pointer"
						: "cursor-not-allowed opacity-75"
				}`}
			>
				<button
					className="w-full flex flex-col items-center gap-2"
					onClick={() => onSelect(file)}
				>
					{file.type === "image" ? (
						<div className="relative w-10 h-10 rounded-md flex items-center justify-center overflow-hidden">
							<img
								src={file.url}
								alt={file.name}
								className="w-full h-full object-cover rounded-sm"
								onError={(e) => {
									// Fallback to icon if image fails to load
									e.currentTarget.style.display = "none";
									const iconElement = e.currentTarget
										.nextElementSibling as HTMLElement;
									if (iconElement) iconElement.style.display = "block";
								}}
							/>
							<div className="hidden">{getFileIcon(file.type)}</div>
						</div>
					) : (
						<div
							className={`relative p-3 rounded-md transition-colors overflow-hidden ${
								canPreview(file)
									? "bg-primary/10 group-hover:bg-primary/20"
									: "bg-muted/50"
							}`}
						>
							{getFileIcon(file.type)}
						</div>
					)}
					<div className="flex flex-col items-center gap-1 w-full overflow-hidden">
						<p className="max-w-full w-full text-center font-medium text-foreground text-xs leading-tight line-clamp-1">
							{getDisplayFileName(file.name)}
						</p>
						{file.previewText && (
							<p className="text-xs text-muted-foreground line-clamp-1 w-full text-center">
								{file.previewText}
							</p>
						)}
						<div className="flex flex-col items-center gap-1">
							<Badge
								variant="outline"
								className="text-xs px-1 py-0 h-4 capitalize"
							>
								{file.type}
							</Badge>
							{file.pageNumber !== undefined && (
								<Badge variant="secondary" className="text-xs px-1 py-0 h-4">
									Page {file.pageNumber}
								</Badge>
							)}
							{file.size && (
								<Badge variant="secondary" className="text-xs px-1 py-0 h-4">
									{humanFileSize(file.size)}
								</Badge>
							)}
						</div>
					</div>
				</button>

				{/* Download/Open button as overlay */}
				<Button
					variant="outline"
					size="sm"
					onClick={(e) => {
						e.stopPropagation();
						handleFileClick(file);
					}}
					className="absolute top-2 right-2 gap-1 opacity-0 group-hover:opacity-100 transition-opacity text-xs h-7 w-7 p-0"
				>
					{file.isDataUrl ? (
						<Download className="w-3 h-3" />
					) : (
						<ExternalLink className="w-3 h-3" />
					)}
				</Button>
			</div>
		);
	}

	// List mode (existing layout)
	return (
		<div
			className={`group relative rounded-lg border border-border/50 p-3 w-full transition-all duration-200 bg-gradient-to-r from-background to-muted/10 ${
				isSelected ? "border-primary bg-primary/5 shadow-sm" : ""
			} ${
				canPreview(file)
					? "hover:border-primary/50 hover:shadow-md cursor-pointer"
					: "cursor-not-allowed opacity-75"
			}`}
		>
			<button
				className="w-full flex flex-row justify-between items-center"
				onClick={() => onSelect(file)}
			>
				<div className="flex flex-row items-center gap-3 flex-1 min-w-0">
					{file.type === "image" ? (
						<div className="relative w-8 h-8 flex items-center justify-center rounded-md overflow-hidden">
							<img
								src={file.url}
								alt={file.name}
								className="w-full h-full object-cover rounded-sm"
								onError={(e) => {
									// Fallback to icon if image fails to load
									e.currentTarget.style.display = "none";
									const iconElement = e.currentTarget
										.nextElementSibling as HTMLElement;
									if (iconElement) iconElement.style.display = "block";
								}}
							/>
							<div className="hidden">{getFileIcon(file.type)}</div>
						</div>
					) : (
						<div
							className={`relative p-2 rounded-md transition-colors overflow-hidden ${
								canPreview(file)
									? "bg-primary/10 group-hover:bg-primary/20"
									: "bg-muted/50"
							}`}
						>
							{getFileIcon(file.type)}
						</div>
					)}
					<div className="flex flex-col items-start flex-1 min-w-0">
						<p className="line-clamp-1 text-start font-medium text-foreground truncate w-full text-sm">
							{getDisplayFileName(file.name)}
						</p>
						{file.previewText && (
							<p className="text-xs text-muted-foreground line-clamp-1 w-full mt-0.5">
								{file.previewText}
							</p>
						)}
						<div className="flex items-center gap-1 mt-1">
							<Badge
								variant="outline"
								className="text-xs px-1 py-0 h-4 capitalize"
							>
								{file.type}
							</Badge>
							{file.pageNumber !== undefined && (
								<Badge variant="secondary" className="text-xs px-1 py-0 h-4">
									Page {file.pageNumber}
								</Badge>
							)}
							{file.size && (
								<Badge variant="secondary" className="text-xs px-1 py-0 h-4">
									{humanFileSize(file.size)}
								</Badge>
							)}
						</div>
					</div>
				</div>
				<Button
					variant="outline"
					size="sm"
					onClick={(e) => {
						e.stopPropagation();
						handleFileClick(file);
					}}
					className="gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
				>
					{file.isDataUrl ? (
						<Download className="w-3 h-3" />
					) : (
						<ExternalLink className="w-3 h-3" />
					)}
					{file.isDataUrl ? "Download" : "Open"}
				</Button>
			</button>
		</div>
	);
}

interface FileDialogPreviewProps {
	file: ProcessedAttachment;
	maximized?: boolean;
}

export function FileDialogPreview({ file }: Readonly<FileDialogPreviewProps>) {
	const handleFileClick = (file: ProcessedAttachment) => {
		if (file.isDataUrl) {
			if (file.type === "image") {
				const newWindow = window.open();
				if (newWindow) {
					newWindow.document.write(
						`<img src="${file.url}" style="max-width: 100%; height: auto;" />`,
					);
				}
			} else {
				const link = document.createElement("a");
				link.href = file.url;
				link.download = file.name ?? "file";
				document.body.appendChild(link);
				link.click();
				document.body.removeChild(link);
			}
		} else {
			window.open(file.url, "_blank", "noopener,noreferrer");
		}
	};

	const imageClasses = "max-w-full h-full object-contain rounded-md";
	const videoClasses = "max-w-full h-full rounded-md";
	const showTextPreview = isText(file.url) || isCode(file.url);

	switch (file.type) {
		case "image":
			return (
				<div className={"flex justify-center items-center h-full p-4"}>
					<img src={file.url} alt={file.name} className={imageClasses} />
				</div>
			);
		case "video":
			return (
				<div className={"flex justify-center items-center h-full p-4"}>
					<video controls className={videoClasses} poster={file.thumbnailUrl}>
						<source src={file.url} />
						Your browser does not support the video tag.
					</video>
				</div>
			);
		case "audio":
			return (
				<div
					className={
						"flex flex-col items-center justify-center gap-4 h-full p-8"
					}
				>
					<Music className="w-16 h-16 text-muted-foreground" />
					<p className="text-lg font-medium text-center">
						{getDisplayFileName(file.name)}
					</p>
					<audio controls className="w-full max-w-md">
						<source src={file.url} />
						Your browser does not support the audio tag.
					</audio>
				</div>
			);
		case "pdf": {
			const pdfUrl =
				file.pageNumber !== undefined
					? `${file.url}#page=${file.pageNumber}`
					: file.url;
			return (
				<div className="w-full h-full">
					<iframe
						src={pdfUrl}
						className="w-full h-full border-0"
						title={file.name}
					/>
				</div>
			);
		}
		case "document":
		case "other":
			if (showTextPreview) {
				return (
					<div className="w-full h-full">
						<FilePreviewer url={file.url} />
					</div>
				);
			}
			return (
				<div
					className={
						"flex flex-col items-center justify-center gap-4 h-full p-8"
					}
				>
					{getFileIcon(file.type)}
					<p className="text-sm text-muted-foreground">
						Preview not available for this file type
					</p>
					<Button
						variant="outline"
						onClick={() => handleFileClick(file)}
						className="gap-2"
					>
						{file.isDataUrl ? (
							<Download className="w-4 h-4" />
						) : (
							<ExternalLink className="w-4 h-4" />
						)}
						{file.isDataUrl ? "Download" : "Open"}
					</Button>
				</div>
			);
		default:
			return (
				<div
					className={
						"flex flex-col items-center justify-center gap-4 h-full p-8"
					}
				>
					{getFileIcon(file.type)}
					<p className="text-sm text-muted-foreground">
						Preview not available for this file type
					</p>
					<Button
						variant="outline"
						onClick={() => handleFileClick(file)}
						className="gap-2"
					>
						{file.isDataUrl ? (
							<Download className="w-4 h-4" />
						) : (
							<ExternalLink className="w-4 h-4" />
						)}
						{file.isDataUrl ? "Download" : "Open"}
					</Button>
				</div>
			);
	}
}
