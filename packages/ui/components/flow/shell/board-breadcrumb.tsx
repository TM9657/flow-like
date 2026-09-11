"use client";

import { ChevronRightIcon } from "lucide-react";
import { memo, useMemo } from "react";
import { cn } from "../../../lib/utils";

/** Ancestors within the open file. The tab strip already names the file at its root. */
export const BoardBreadcrumb = memo(function BoardBreadcrumb({
	fileLabel,
	fileRootPath,
	layerPath,
	layerNames,
	onJumpToLayer,
}: Readonly<{
	fileLabel: string;
	/** Full layer path to the module root. Undefined or `root` means main. */
	fileRootPath?: string;
	/** Full layer path from the board root, deepest last. */
	layerPath?: string;
	layerNames: Map<string, string>;
	onJumpToLayer: (path: string) => void;
}>) {
	const { rootPath, segments } = useMemo(() => {
		const fileSegments =
			fileRootPath && fileRootPath !== "root"
				? fileRootPath.split("/").filter(Boolean)
				: [];
		const pathSegments =
			layerPath && layerPath !== "root"
				? layerPath.split("/").filter(Boolean)
				: [];
		const rootPath = fileSegments.join("/") || "root";
		if (!fileSegments.every((id, index) => pathSegments[index] === id)) {
			return { rootPath, segments: [] };
		}
		return {
			rootPath,
			segments: pathSegments.slice(fileSegments.length).map((id, index) => ({
				id,
				path: pathSegments.slice(0, fileSegments.length + index + 1).join("/"),
			})),
		};
	}, [fileRootPath, layerPath]);

	if (segments.length === 0) return null;

	return (
		<nav
			aria-label={fileLabel}
			className="flex h-5 shrink-0 items-center gap-0.5 overflow-x-auto border-b bg-muted/10 px-2 no-scrollbar"
		>
			<button
				type="button"
				onClick={() => onJumpToLayer(rootPath)}
				className="shrink-0 rounded-sm px-1 font-mono text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground"
			>
				{fileLabel}
			</button>
			{segments.map(({ id, path }, index) => {
				const last = index === segments.length - 1;
				return (
					<span key={path} className="flex shrink-0 items-center gap-0.5">
						<ChevronRightIcon className="size-3 text-muted-foreground/50" />
						<button
							type="button"
							disabled={last}
							aria-current={last ? "page" : undefined}
							onClick={() => onJumpToLayer(path)}
							className={cn(
								"rounded-sm px-1 text-[11px]",
								last
									? "text-foreground"
									: "text-muted-foreground hover:bg-accent hover:text-foreground",
							)}
						>
							{layerNames.get(id) ?? id}
						</button>
					</span>
				);
			})}
		</nav>
	);
});
