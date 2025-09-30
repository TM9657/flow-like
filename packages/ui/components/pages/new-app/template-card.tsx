"use client";
import { Calendar, Check, Copy, Workflow } from "lucide-react";
import {
	Badge,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	type IDate,
	formatRelativeTime,
} from "../../..";
import type { IMetadata } from "../../../lib";

export function TemplateCard({
	appId,
	templateId,
	metadata,
	selected,
	onSelect,
	compact = false,
}: Readonly<{
	appId: string;
	templateId: string;
	metadata: IMetadata;
	selected: boolean;
	onSelect: () => void;
	compact?: boolean;
}>) {
	return (
		<Card
			role="button"
			aria-pressed={selected}
			aria-label={metadata?.name ?? "Template"}
			tabIndex={0}
			className={`group cursor-pointer transition-all duration-300 h-full flex flex-col overflow-hidden focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background ${
				selected
					? "ring-2 ring-primary shadow-xl shadow-primary/20"
					: "hover:shadow-xl"
			}`}
			onClick={onSelect}
			onKeyDown={(e) => {
				if (e.key === "Enter" || e.key === " ") {
					e.preventDefault();
					onSelect();
				}
			}}
		>
			<CardHeader className="space-y-2 p-3 sm:p-5">
				<div className="flex items-start justify-between">
					<div className="flex items-center gap-3">
						<div
							className={`p-2 rounded-lg transition-colors flex-none ${
								selected
									? "bg-primary/30"
									: "bg-primary/10 group-hover:bg-primary/30"
							}`}
						>
							<Copy className="w-5 h-5 text-primary shrink-0" />
						</div>
						<div className="flex-1 min-w-0">
							<CardTitle
								className={`text-lg font-semibold transition-colors truncate break-words ${
									selected
										? "text-primary"
										: "text-foreground group-hover:text-primary"
								}`}
								title={metadata?.name}
							>
								{metadata?.name}
							</CardTitle>
						</div>
					</div>
					<div className="flex items-center gap-2">
						{selected && (
							<div className="p-1.5 bg-primary rounded-full">
								<Check className="h-4 w-4 text-primary-foreground" />
							</div>
						)}
					</div>
				</div>
			</CardHeader>
			<CardContent className="space-y-2 sm:space-y-4 flex-1 flex flex-col p-3 pt-0 sm:p-5 sm:pt-0">
				<p className="text-muted-foreground text-sm leading-relaxed line-clamp-2 text-start break-words whitespace-normal max-w-full hyphens-auto">
					{metadata?.description}
				</p>

				<div className="flex flex-wrap gap-1 min-w-0">
					<Badge
						variant={selected ? "default" : "secondary"}
						className="text-xs max-w-full overflow-hidden whitespace-nowrap text-ellipsis"
					>
						<Workflow className="h-3 w-3 mr-1 shrink-0" />
						Template
					</Badge>
					{metadata?.tags?.map((tag) => (
						<Badge
							key={tag}
							variant="outline"
							className="text-xs max-w-full overflow-hidden whitespace-nowrap text-ellipsis min-w-0"
							title={tag}
						>
							{tag}
						</Badge>
					))}
				</div>

				<div className="pt-2 sm:pt-4 border-t mt-auto">
					<div className="flex items-center justify-between text-xs text-muted-foreground">
						<div className="flex items-center gap-1">
							<Calendar className="w-3 h-3" />
							{metadata?.created_at && (
								<span>{formatRelativeTime(metadata?.created_at as IDate)}</span>
							)}
						</div>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}
