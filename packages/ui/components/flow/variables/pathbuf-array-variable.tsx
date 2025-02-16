import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { HoverCard, HoverCardContent, HoverCardTrigger } from "../../../components/ui/hover-card";
import { Label } from "../../../components/ui/label";
import { cn } from "../../../lib/utils";
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import { FolderIcon, InfoIcon, X } from "lucide-react";
import { useEffect, useState } from "react";
import { FileList } from "./pathbuf-list";
import { type IFileMetadata } from "../../../lib/schema/files/file-metadata";

    // @ts-ignore
export function FolderArrayVariable({ name, variable, onChange }: Readonly<{ name: string, variable: IAdapterConfigEntry, onChange: (value: any) => void }>) {
    const [folders, setFolders] = useState<string[] | undefined>(variable.value)
    const [files, setFiles] = useState<IFileMetadata[]>([])

    async function loadFiles() {
        if (!folders) return;
        const files = (await Promise.all(folders?.map(folder => invoke<IFileMetadata[]>("get_folder_meta", { folderPath: folder })))).flat();
        setFiles(files);
    }

    useEffect(() => {
        onChange(folders);
        loadFiles()
    }, [folders])

    return <div key={name} className="w-full items-center gap-1.5">
        <div className="flex flex-row items-center gap-2">
            <Label htmlFor={name}>{name}</Label>
            <HoverCard>
                <HoverCardTrigger>
                    <InfoIcon className="w-4 h-4" />
                </HoverCardTrigger>
                <HoverCardContent>
                    {variable.description}
                </HoverCardContent>
            </HoverCard>
        </div>
        <div className="flex flex-row flex-wrap items-center justify-start gap-2 mt-2">
            {folders && folders.length > 0 && folders.map(folder => <Badge onClick={() => {
                setFolders(folders?.filter(f => f !== folder));
            }} className="gap-2" key={folder}>{folder} <X className="w-4 h-4"/></Badge>)}
        </div>
        <FileList files={files} >
            <div key={name} className="w-full items-center gap-1.5">
                <div className="flex flex-row items-center gap-2">
                    <Button
                        id={name}
                        variant={"outline"}
                        className={cn(
                            "w-full justify-start text-left font-normal text-muted-foreground"
                        )}
                        onClick={async () => {
                            const folders: any = await open({
                                multiple: true,
                                directory: true,
                                recursive: true,
                            });
                            if (!folders) return;
                            setFolders(old => Array.from(new Set([...(old || []), ...(folders || [])])));
                        }}
                    >
                        <FolderIcon className="mr-2 h-4 w-4" />
                        <span>Pick folders</span>
                    </Button>
                    {folders &&
                        <Button size={"sm"} variant={"ghost"} onClick={() => {
                            setFolders([]);
                            setFiles([]);
                        }}><X className="text-muted-foreground w-4 h-4" /></Button>
                    }
                </div>
            </div>
        </FileList>
    </div>
}