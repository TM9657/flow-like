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
import { humanFileSize, type INode } from "../../lib";
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
import { useCallback } from "react";
import { parseUint8ArrayToJson, convertJsonToUint8Array } from "../../lib/uint8";
import { toast } from "sonner";

const TEMPLATE = JSON.parse(`{
  "nodes": [
    {
      "id": "e1l3erwbnvnxnnv97524oiuq",
      "name": "child",
      "friendly_name": "Child",
      "comment": "",
      "coordinates": [
        1502.9323,
        -1082.7615,
        0
      ],
      "pins": {
        "mvgidpho004uf2ntonjyt1ji": {
          "id": "mvgidpho004uf2ntonjyt1ji",
          "name": "path",
          "friendly_name": "Path",
          "pin_type": "Output",
          "data_type": "Struct",
          "value_type": "Normal",
          "depends_on": [],
          "connected_to": [],
          "index": 1
        },
        "w8k4qi9sq7265ium4c3l6qg8": {
          "id": "w8k4qi9sq7265ium4c3l6qg8",
          "name": "parent_path",
          "friendly_name": "Path",
          "pin_type": "Input",
          "data_type": "Struct",
          "value_type": "Normal",
          "depends_on": [
            "mgn6k0wfu3owigtms65ngqts"
          ],
          "connected_to": [],
          "index": 1
        },
        "x56ex8kn2uoq37rd8xitawbh": {
          "id": "x56ex8kn2uoq37rd8xitawbh",
          "name": "child_name",
          "friendly_name": "Child",
          "pin_type": "Input",
          "data_type": "String",
          "value_type": "Normal",
          "depends_on": [],
          "connected_to": [],
          "default_value": [
            34,
            116,
            101,
            115,
            116,
            47,
            97,
            98,
            99,
            34
          ],
          "index": 2
        }
      }
    },
    {
      "id": "bt5orj8f06apjvbbz4isbapr",
      "name": "path_from_upload_dir",
      "friendly_name": "Upload Dir",
      "comment": "",
      "coordinates": [
        1315.6329,
        -1082.7726,
        0
      ],
      "pins": {
        "mgn6k0wfu3owigtms65ngqts": {
          "id": "mgn6k0wfu3owigtms65ngqts",
          "name": "path",
          "friendly_name": "Path",
          "pin_type": "Output",
          "data_type": "Struct",
          "value_type": "Normal",
          "depends_on": [],
          "connected_to": [
            "w8k4qi9sq7265ium4c3l6qg8"
          ],
          "index": 1
        }
      }
    }
  ],
  "comments": []
}`)

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

	const copyPath = useCallback((isFolder: boolean) => {
		const childNode = (TEMPLATE.nodes as INode[]).findIndex((node) => node.name === "child");
		if(childNode === -1) return;

		if (isFolder) {
			const location = file.location.split("/").pop()?.slice(1, -7) ?? "";
			const parentPath = file.location.split("/").slice(3, -1).join("/");
			TEMPLATE.nodes[childNode].pins["w8k4qi9sq7265ium4c3l6qg8"].default_value = convertJsonToUint8Array(`${parentPath}/${location}`);
			navigator.clipboard.writeText(JSON.stringify(TEMPLATE))
			toast.success("Path copied to clipboard");
			return;
		}

		TEMPLATE.nodes[childNode].pins["x56ex8kn2uoq37rd8xitawbh"].default_value = convertJsonToUint8Array(file.location.split("/").slice(3).join("/"));
		navigator.clipboard.writeText(JSON.stringify(TEMPLATE))
		toast.success("Path copied to clipboard");
	}, [
		file.location
	]);

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
						<DropdownMenuItem
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									copyPath(true);
								}}
							>
								Copy Path
							</DropdownMenuItem>
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
						<DropdownMenuItem
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									copyPath(false);
								}}
							>
								Copy Path
							</DropdownMenuItem>
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
