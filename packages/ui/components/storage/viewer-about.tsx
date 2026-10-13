"use client";

import { InfoIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "../ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "../ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

export function ViewerAbout({
	label,
	licensesUrl,
	children,
}: {
	label: string;
	licensesUrl: string;
	children?: ReactNode;
}) {
	return (
		<Dialog>
			<Tooltip>
				<TooltipTrigger asChild>
					<DialogTrigger asChild>
						<Button
							variant="ghost"
							size="icon"
							className="size-8 shrink-0 text-muted-foreground"
							aria-label={`About ${label}`}
						>
							<InfoIcon />
						</Button>
					</DialogTrigger>
				</TooltipTrigger>
				<TooltipContent>Third-party licenses and source</TooltipContent>
			</Tooltip>
			<DialogContent className="h-[80dvh] overflow-hidden sm:max-w-4xl">
				<DialogHeader className="pr-6">
					<DialogTitle>About {label}</DialogTitle>
					<DialogDescription>
						{children ?? "Third-party licenses, notices, and source code."}
					</DialogDescription>
				</DialogHeader>
				<iframe
					src={licensesUrl}
					title={`${label} third-party licenses and source`}
					className="min-h-0 w-full flex-1 rounded-md border bg-white"
				/>
			</DialogContent>
		</Dialog>
	);
}
