"use client";
import { Filter, Search, Workflow } from "lucide-react";
import { useState } from "react";
import {
	Badge,
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
	Input,
} from "../../..";
import type { IMetadata } from "../../../lib";
import { AppTemplateFolder } from "./template-app-folder";
import { useTemplateFolders } from "./use-template-folder";

export function TemplateModal({
	open,
	onClose,
	templates,
	selectedTemplate,
	onSelectTemplate,
}: Readonly<{
	open: boolean;
	onClose: () => void;
	templates: [string, string, IMetadata][];
	selectedTemplate: [string, string];
	onSelectTemplate: (appId: string, templateId: string) => void;
}>) {
	const [searchQuery, setSearchQuery] = useState("");
	const [selectedTags, setSelectedTags] = useState<string[]>([]);
	const templatesByApp = useTemplateFolders(templates);

	const filteredTemplates = templates.filter(
		([appId, templateId, metadata]) => {
			const matchesSearch =
				metadata.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
				metadata?.description.toLowerCase().includes(searchQuery.toLowerCase());
			const matchesTags =
				selectedTags.length === 0 ||
				selectedTags.some((tag) => metadata.tags?.includes(tag));
			return matchesSearch && matchesTags;
		},
	);

	const allTags = [
		...new Set(
			templates.flatMap(
				([appId, templateId, metadata]) => metadata?.tags || [],
			),
		),
	];

	return (
		<Dialog open={open} onOpenChange={(open) => !open && onClose()}>
			<DialogContent className="h-[85dvh] sm:h-[90dvh] w-[95vw] sm:w-auto sm:max-w-5xl flex flex-col p-2 sm:p-3 overflow-hidden">
				<DialogHeader className="sticky top-0 z-10 bg-background/90 backdrop-blur supports-[backdrop-filter]:bg-background/60 px-1 sm:px-0">
					<div className="flex items-center gap-3">
						<div className="p-2 bg-primary/10 rounded-lg">
							<Workflow className="h-5 w-5 text-primary" />
						</div>
						<div>
							<DialogTitle className="text-xl sm:text-2xl font-bold">
								Choose Template
							</DialogTitle>
							<p className="text-muted-foreground text-sm sm:text-base">
								Select a template to start building your app
							</p>
						</div>
					</div>
				</DialogHeader>

				<div className="space-y-4 border-b pb-4">
					<div className="relative">
						<Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
						<Input
							placeholder="Search templates..."
							value={searchQuery}
							onChange={(e) => setSearchQuery(e.target.value)}
							className="pl-10"
						/>
					</div>
					{allTags.length > 0 && (
						<div className="flex items-center gap-2 flex-wrap">
							<Filter className="h-4 w-4 text-muted-foreground" />
							{allTags.map((tag) => (
								<Badge
									key={tag}
									variant={selectedTags.includes(tag) ? "default" : "outline"}
									className="cursor-pointer"
									onClick={() => {
										setSelectedTags((prev) =>
											prev.includes(tag)
												? prev.filter((t) => t !== tag)
												: [...prev, tag],
										);
									}}
								>
									{tag}
								</Badge>
							))}
						</div>
					)}
				</div>

				<div className="flex-1 overflow-auto">
					<div className="space-y-4 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3 sm:gap-4 p-1 sm:p-2">
						{templatesByApp.map(([appId, templates]) => (
							<AppTemplateFolder
								key={appId}
								appId={appId}
								templates={templates.filter(([templateId, metadata]) =>
									filteredTemplates.some(([, tid]) => tid === templateId),
								)}
								selectedTemplate={selectedTemplate}
								onSelectTemplate={(appId, templateId) => {
									onSelectTemplate(appId, templateId);
									onClose();
								}}
							/>
						))}
					</div>
					{filteredTemplates.length === 0 && (
						<div className="text-center py-12 text-muted-foreground">
							<Workflow className="h-12 w-12 mx-auto mb-4 opacity-50" />
							<p>No templates found matching your criteria</p>
						</div>
					)}
				</div>
			</DialogContent>
		</Dialog>
	);
}
