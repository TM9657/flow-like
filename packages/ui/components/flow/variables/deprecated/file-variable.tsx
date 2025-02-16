// import { Button } from "@/components/ui/button";
// import { HoverCard, HoverCardContent, HoverCardTrigger } from "@/components/ui/hover-card";
// import { Label } from "@/components/ui/label";
// import { cn } from "../../lib/utils";
// import { invoke } from "@tauri-apps/api/core";
// import { open } from '@tauri-apps/plugin-dialog';
// import { FileIcon, InfoIcon, X } from "lucide-react";
// import { useEffect, useState } from "react";

// export function FileVariable({ name, variable, onChange }: Readonly<{ name: string, variable: IAdapterConfigEntry, onChange: (value: any) => void }>) {
//     const [file, setFile] = useState<string | undefined>(variable.value)
//     const [fileMetadata, setFileMetadata] = useState<IFile | undefined>(undefined)

//     async function fetchFileMetadata(){
//         const fileMetadata = await invoke<IFile>("get_file_meta", { filePath: file });
//         setFileMetadata(fileMetadata);
//     }

//     useEffect(() => {
//         onChange(file);
//         fetchFileMetadata()
//     }, [file])

//     return <div key={name} className="grid w-full max-w-sm items-center gap-1.5">
//         <div key={name} className="grid w-full max-w-sm items-center gap-1.5">
//             <div className="flex flex-row items-center gap-2">
//                 <Label htmlFor={name}>{name}</Label>
//                 <HoverCard>
//                     <HoverCardTrigger>
//                         <InfoIcon className="w-4 h-4" />
//                     </HoverCardTrigger>
//                     <HoverCardContent>
//                         {variable.description}
//                     </HoverCardContent>
//                 </HoverCard>
//             </div>
//             <div className="flex flex-row items-center gap-2">
//             <Button
//                 id={name}
//                 variant={"outline"}
//                 className={cn(
//                     "w-[280px] justify-start text-left font-normal",
//                     !file && "text-muted-foreground"
//                 )}
//                 onClick={async () => {
//                     const file: any = await open({
//                         multiple: false,
//                         directory: false,
//                     });
//                     if (!file) return;
//                     setFile(file.path);
//                 }}
//             >
//                 <FileIcon className="mr-2 h-4 w-4" />
//                 {file ? <span>{fileMetadata?.file_name}</span> : <span>Pick a file</span>}
//             </Button>
//             {file &&
//                 <Button size={"sm"} variant={"ghost"} onClick={() => {
//                     setFile(undefined);
//                 }}><X className="text-muted-foreground w-4 h-4"/></Button>
//             }
//             </div>
            
//         </div>
//     </div>
// }