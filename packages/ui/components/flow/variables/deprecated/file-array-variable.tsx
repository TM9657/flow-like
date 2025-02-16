// import { Button } from "@/components/ui/button";
// import { HoverCard, HoverCardContent, HoverCardTrigger } from "@/components/ui/hover-card";
// import { Label } from "@/components/ui/label";
// import { cn } from "../../lib/utils";
// import { invoke } from "@tauri-apps/api/core";
// import { open } from '@tauri-apps/plugin-dialog';
// import { FolderIcon, InfoIcon } from "lucide-react";
// import { useEffect, useState } from "react";
// import { FileList } from "./file-list";

// export function FileArrayVariable({ name, variable, onChange }: Readonly<{ name: string, variable: IAdapterConfigEntry, onChange: (value: any) => void }>) {
//     const [files, setFiles] = useState<string[] | undefined>(variable.value)
//     const [fileMetadata, setFileMetadata] = useState<IFile[]>([])

//     async function fetchFileMetadata(){
//         const fileMetadata = await Promise.all((files || []).map((file) => invoke<IFile>("get_file_meta", { filePath: file })));
//         setFileMetadata(fileMetadata);
//     }

//     useEffect(() => {
//         console.dir(files)
//         onChange(files);
//         fetchFileMetadata()
//     }, [files])

//     return <button key={name} className="w-full items-center gap-1.5 relative" onDrop={(e) => {
//         console.dir(e)
//     }}>
//         <div className="flex flex-row items-center gap-2">
//             <Label htmlFor={name}>{name}</Label>
//             <HoverCard>
//                 <HoverCardTrigger>
//                     <InfoIcon className="w-4 h-4" />
//                 </HoverCardTrigger>
//                 <HoverCardContent>
//                     {variable.description}
//                 </HoverCardContent>
//             </HoverCard>
//         </div>
//         <FileList onFileDelete={(file) => {
//                 setFiles(files?.filter(f => f !== file.file_path));
//         }} files={fileMetadata || []} >
//             <div key={name} className="w-full items-center gap-1.5">
//                 <div className="flex flex-row items-center gap-2">
//                     <Button
//                         id={name}
//                         variant={"outline"}
//                         className={cn(
//                             "w-full justify-start text-left font-normal text-muted-foreground"
//                         )}
//                         onClick={async () => {
//                             const files: any = await open({
//                                 multiple: true,
//                                 directory: false,
//                                 recursive: true,
//                             });
//                             if (!files) return;
//                             setFiles(old => Array.from(new Set([...(old || []), ...files.map((f: any) => f.path)])));
//                         }}
//                     >
//                         <FolderIcon className="mr-2 h-4 w-4" />
//                         <span>Pick files</span>
//                     </Button>
//                 </div>
//             </div>
//         </FileList>
//     </button>
// }