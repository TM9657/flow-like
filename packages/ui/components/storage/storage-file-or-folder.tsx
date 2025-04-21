import { EllipsisVerticalIcon, FileIcon, FolderIcon } from "lucide-react";
import { humanFileSize } from "../../lib";
import { Button } from "../ui";

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
}: Readonly<{
	file: IStorageItem;
	highlight: boolean;
	changePrefix?: (prefix: string) => void;
	loadFile?: (file: string) => void;
}>) {
	if (file.location.endsWith("._path")) {
		return (
			<button
				className="border p-2 w-full flex flex-row justify-start items-center hover:bg-muted bg-background"
				onClick={() => {
					changePrefix?.(file.location.split("/").pop()?.slice(1, -7) ?? "");
				}}
			>
				<FolderIcon className="w-4 h-4 mr-2" />
				<p className="line-clamp-1 text-start">
					{file.location.split("/").pop()?.slice(1, -7)}
				</p>
			</button>
		);
	}

	return (
		<button
			className={`border p-2 w-full flex flex-row justify-between items-center bg-background ${highlight ? "bg-muted" : ""}`}
			onClick={() => {
				loadFile?.(file.location);
			}}
		>
			<div className="flex flex-row items-center gap-2">
				<FileIcon className="w-4 h-4" />
				<p className="line-clamp-1 text-start">
					{file.location.split("/").pop()}
				</p>
			</div>
			<div className="flex flex-row items-center gap-2 ml-2">
				<small className="text-xs text-muted-foreground whitespace-nowrap">
					{humanFileSize(file.size, true)}
				</small>
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
			</div>
		</button>
	);
}
