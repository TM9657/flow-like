import { FileIcon, FolderIcon, Trash2Icon } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Button } from "../../../components/ui/button";
import { Label } from "../../../components/ui/label";
import { Switch } from "../../../components/ui/switch";
import type { IVariable } from "../../../lib/schema/flow/variable";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../lib/uint8";
import { cn } from "../../../lib/utils";
import { useBackend } from "../../../state/backend-state";
import { Separator } from "../../ui";

export function PathbufArrayVariable({
	disabled,
	variable,
	onChange,
}: Readonly<{
	disabled?: boolean;
	variable: IVariable;
	onChange: (variable: IVariable) => void;
}>) {
	const backend = useBackend();

	// parse once from default_value
	const items = useMemo<string[]>(() => {
		const parsed = parseUint8ArrayToJson(variable.default_value);
		return Array.isArray(parsed) ? parsed : [];
	}, [variable.default_value]);

	const [isFolder, setIsFolder] = useState<boolean>(false);

	// add a new path
	const handleAdd = useCallback(async () => {
		if (disabled) return;
		const pathBuf: any = await backend.helperState.openFileOrFolderMenu(
			false,
			isFolder,
			true,
		);
		if (!pathBuf) return;

		let finalPath = pathBuf;

		if (!isFolder) {
			const meta = await backend.helperState.getPathMeta(pathBuf);
			if (!meta || meta.length === 0) return;
			finalPath = meta[0].location;
		}

		const updated = [...items, finalPath];
		onChange({
			...variable,
			default_value: convertJsonToUint8Array(updated),
		});
	}, [disabled, backend, isFolder, items, onChange, variable]);

	const handleRemove = useCallback(
		(idx: number) => {
			if (disabled) return;
			const updated = items.filter((_, i) => i !== idx);
			onChange({
				...variable,
				default_value: convertJsonToUint8Array(updated),
			});
		},
		[disabled, items, onChange, variable],
	);

	return (
		<div className="grid w-full max-w-full grid-cols-6 gap-2">
			<div className="flex items-center space-x-2 col-span-2  sticky top-0 bg-background">
				<Switch
					checked={isFolder}
					onCheckedChange={setIsFolder}
					id="is_folder"
					disabled={disabled}
				/>
				<Label htmlFor="is_folder">Folder</Label>
			</div>

			<Button
				variant="outline"
				className={cn(
					"w-full justify-start text-left font-normal col-span-4  sticky top-0 bg-background",
					items.length === 0 && "text-muted-foreground",
				)}
				disabled={disabled}
				onClick={handleAdd}
			>
				{isFolder ? (
					<FolderIcon className="mr-2 h-4 w-4" />
				) : (
					<FileIcon className="mr-2 h-4 w-4" />
				)}
				<span>{isFolder ? "Add Folder" : "Add File"}</span>
			</Button>

			<Separator className="my-2 w-full col-span-6" />
			<div className="col-span-6 space-y-2">
				{items.map((path, idx) => (
					<div
						key={`${variable.name}-${idx}`}
						className="flex items-center justify-between border p-1"
					>
						{!path.split("/").pop()?.includes(".") ? (
							<FolderIcon className="mx-2 h-4 w-4" />
						) : (
							<FileIcon className="mx-2 h-4 w-4" />
						)}
						<span className="flex-1 truncate">{path.split("/").pop()}</span>
						<Button
							disabled={disabled}
							size="icon"
							variant="destructive"
							onClick={() => handleRemove(idx)}
						>
							<Trash2Icon className="h-4 w-4" />
						</Button>
					</div>
				))}
			</div>
		</div>
	);
}
