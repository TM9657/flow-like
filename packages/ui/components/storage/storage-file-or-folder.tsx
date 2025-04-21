import { IconBinary, IconPdf } from "@tabler/icons-react";
import {
    BracesIcon,
    EllipsisVerticalIcon,
    FileIcon,
    FolderIcon,
    HeadphonesIcon,
    ImageIcon,
    LetterTextIcon,
    VideoIcon,
} from "lucide-react";
import { humanFileSize } from "../../lib";
import {
    Button,
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuTrigger,
    canPreview,
    isAudio,
    isCode,
    isImage,
    isText,
    isVideo,
} from "../ui";

export interface IStorageItem {
	location: string;
	last_modified: string;
	size: number;
	e_tag?: string;
	version?: string;
}

export function FileOrFolder({
	file,
	highlight,
	changePrefix,
	loadFile,
	shareFile,
	deleteFile,
	downloadFile,
}: Readonly<{
	file: IStorageItem;
	highlight: boolean;
	changePrefix?: (prefix: string) => void;
	loadFile?: (file: string) => void;
	shareFile?: (file: string) => void;
	deleteFile?: (file: string) => void;
	downloadFile?: (file: string) => void;
}>) {
	if (file.location.endsWith("._path")) {
		return (
			<button
				className="border p-2 w-full flex flex-row justify-between items-center hover:bg-muted bg-background"
				onClick={() => {
					changePrefix?.(file.location.split("/").pop()?.slice(1, -7) ?? "");
				}}
			>
				<div className="flex flex-row items-center gap-2">
					<FolderIcon className="w-4 h-4 text-primary" />
					<p className="line-clamp-1 text-start">
						{file.location.split("/").pop()?.slice(1, -7)}
					</p>
				</div>
				<DropdownMenu>
					<DropdownMenuTrigger>
						<Button
							className="max-h-4 max-w-4"
							variant={"ghost"}
							size={"icon"}
							onClick={(e) => {
								e.stopPropagation();
								e.preventDefault();
							}}
						>
							<EllipsisVerticalIcon className="max-h-4" />
						</Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent>
						<DropdownMenuLabel>Folder Actions</DropdownMenuLabel>
						{typeof deleteFile !== "undefined" && (
							<DropdownMenuItem
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									deleteFile?.(
										file.location.split("/").pop()?.slice(1, -7) ?? "",
									);
									deleteFile?.(file.location.split("/").pop() ?? "");
								}}
							>
								Delete
							</DropdownMenuItem>
						)}
					</DropdownMenuContent>
				</DropdownMenu>
			</button>
		);
	}

	return (
		<button
			className={`border p-2 w-full flex flex-row justify-between items-center bg-background ${highlight ? "bg-muted" : ""} ${canPreview(file.location) ? "hover:border-primary" : "cursor-not-allowed"}`}
			onClick={() => {
				if (canPreview(file.location)) loadFile?.(file.location);
			}}
		>
			<div className="flex flex-row items-center gap-2">
				<IconForFile file={file} />
				<p className="line-clamp-1 text-start">
					{file.location.split("/").pop()}
				</p>
			</div>
			<div className="flex flex-row items-center gap-2 ml-2">
				<small className="text-xs text-muted-foreground whitespace-nowrap">
					{humanFileSize(file.size, true)}
				</small>
				<DropdownMenu>
					<DropdownMenuTrigger>
						<Button
							className="max-h-4 max-w-4"
							variant={"ghost"}
							size={"icon"}
							onClick={(e) => {
								e.stopPropagation();
								e.preventDefault();
							}}
						>
							<EllipsisVerticalIcon className="max-h-4" />
						</Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent>
						<DropdownMenuLabel>File Actions</DropdownMenuLabel>
						{typeof downloadFile !== "undefined" && (
							<DropdownMenuItem
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									downloadFile?.(file.location.split("/").pop() ?? "");
								}}
							>
								Download
							</DropdownMenuItem>
						)}
						{typeof shareFile !== "undefined" && (
							<DropdownMenuItem
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									shareFile?.(file.location.split("/").pop() ?? "");
								}}
							>
								Share
							</DropdownMenuItem>
						)}
						{typeof deleteFile !== "undefined" && (
							<DropdownMenuItem
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									deleteFile?.(file.location.split("/").pop() ?? "");
								}}
							>
								Delete
							</DropdownMenuItem>
						)}
					</DropdownMenuContent>
				</DropdownMenu>
			</div>
		</button>
	);
}

function IconForFile({ file }: Readonly<{ file: IStorageItem }>) {
	const className = `w-4 h-4 ${canPreview(file.location) ? "text-primary" : "text-foreground"}`;
	if (file.location.endsWith(".pdf")) return <IconPdf className={className} />;
	if (isImage(file.location)) return <ImageIcon className={className} />;
	if (isVideo(file.location)) return <VideoIcon className={className} />;
	if (isAudio(file.location)) return <HeadphonesIcon className={className} />;
	if (isCode(file.location)) return <BracesIcon className={className} />;
	if (isText(file.location)) return <LetterTextIcon className={className} />;
	if (!file.location.split("/").pop()?.includes("."))
		return <IconBinary className={className} />;
	return <FileIcon className={className} />;
}
