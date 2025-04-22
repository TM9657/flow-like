"use client";

import { FilesIcon, LayoutGridIcon, LinkIcon } from "lucide-react";
import { useCallback, useState } from "react";
import {
	Button,
	EmptyState,
	FilePreviewer,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "../ui";
import { StorageBreadcrumbs } from "./storage-breadcrumbs";
import { FileOrFolder, type IStorageItem } from "./storage-file-or-folder";

export function StorageSystem({
	appId,
	prefix,
	files,
	updatePrefix,
	fileToUrl,
	uploadFile,
	deleteFile,
	shareFile,
	moveFile,
	downloadFile,
}: Readonly<{
	appId: string;
	prefix: string;
	files: IStorageItem[];
	updatePrefix: (prefix: string) => void;
	fileToUrl: (prefix: string) => Promise<string>;
	uploadFile: (prefix: string, folder: boolean) => Promise<void>;
	deleteFile: (prefix: string) => Promise<void>;
	shareFile: (prefix: string) => Promise<void>;
	moveFile: (prefix: string, newPrefix: string) => Promise<void>;
	downloadFile?: (prefix: string) => Promise<void>;
}>) {
	const [preview, setPreview] = useState({
		url: "",
		file: "",
	});

	const loadFile = useCallback(
		async (file: string) => {
			if (preview.file === file) {
				setPreview((old) => ({ ...old, file: "", url: "" }));
				return;
			}

			const url = await fileToUrl(file);
			setPreview({
				url,
				file,
			});
		},
		[appId, preview],
	);

	return (
		<div className="flex flex-grow flex-col gap-2 h-full max-h-full overflow-hidden w-full">
			{files.length > 0 && (
				<div className="flex flex-row items-end justify-between">
					<StorageBreadcrumbs
						appId={appId}
						prefix={prefix}
						updatePrefix={(prefix) => updatePrefix(prefix)}
					/>
					<div className="flex flex-row items-center gap-2 ">
						<Button
							variant={"outline"}
							onClick={() => {
								uploadFile(prefix, false);
							}}
						>
							Upload Files
						</Button>
						<Button
							variant={"outline"}
							onClick={() => {
								uploadFile(prefix, true);
							}}
						>
							Upload Folder
						</Button>
					</div>
				</div>
			)}
			{files.length === 0 && (
				<div className="flex flex-col h-full w-full flex-grow relative">
					<EmptyState
						className="w-full h-full max-w-full"
						title="No Files Found"
						description="Upload Files to the Storage Interface"
						action={[
							{
								label: "Upload Files",
								onClick: () => {
									uploadFile(prefix, false);
								},
							},
							{
								label: "Upload Folder",
								onClick: () => {
									uploadFile(prefix, true);
								},
							},
						]}
						icons={[LayoutGridIcon, FilesIcon, LinkIcon]}
					/>
				</div>
			)}
			{files.length > 0 && (
				<div className="flex flex-col gap-2 mt-2 flex-grow max-h-full h-full overflow-y-hidden">
					{preview.url !== "" && (
						<ResizablePanelGroup
							direction="horizontal"
							autoSaveId={"file_viewer"}
						>
							<ResizablePanel className="flex flex-col gap-2 flex-grow overflow-y-auto max-h-full h-full">
								<div className="flex flex-col flex-grow max-h-full h-full overflow-auto gap-2">
								{files.map((file) => (
									<FileOrFolder
										highlight={preview.file === file.location}
										key={file.location}
										file={file}
										changePrefix={(new_prefix) =>
											updatePrefix(`${prefix}/${new_prefix}`)
										}
										loadFile={(file) => loadFile(file)}
										deleteFile={(file) => {
											const filePrefix = `${prefix}/${file}`;
											deleteFile(filePrefix);
										}}
										shareFile={(file) => {
											const filePrefix = `${prefix}/${file}`;
											shareFile(filePrefix);
										}}
										downloadFile={downloadFile}
									/>
								))}
								</div>
							</ResizablePanel>
							<ResizableHandle className="mx-2" />
							<ResizablePanel className="flex flex-col gap-2 flex-grow overflow-y-hidden max-h-full h-full">
								<div className="flex flex-col flex-grow overflow-auto max-h-full h-full bg-background">
									<FilePreviewer url={preview.url} page={2} />
								</div>
							</ResizablePanel>
						</ResizablePanelGroup>
					)}
					{preview.url === "" && <div className="flex flex-col flex-grow max-h-full h-full overflow-auto gap-2">
						{files.map((file) => (
							<FileOrFolder
								highlight={preview.file === file.location}
								key={file.location}
								file={file}
								changePrefix={(new_prefix) => {
									setPreview({
										url: "",
										file: "",
									});
									updatePrefix(`${prefix}/${new_prefix}`);
								}}
								loadFile={loadFile}
								deleteFile={(file) => {
									const filePrefix = `${prefix}/${file}`;
									deleteFile(filePrefix);
								}}
								shareFile={(file) => {
									const filePrefix = `${prefix}/${file}`;
									shareFile(filePrefix);
								}}
								downloadFile={downloadFile}
							/>
						))}
						</div>}
				</div>
			)}
		</div>
	);
}
