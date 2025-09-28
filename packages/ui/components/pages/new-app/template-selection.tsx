"use client";
import { Copy, CopyIcon, Grid } from "lucide-react";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Checkbox,
	Label,
} from "../../..";
import type { IMetadata } from "../../../lib";
import { AppTemplateFolder } from "./template-app-folder";
import { useTemplateFolders } from "./use-template-folder";

export function TemplateSection({
	templates,
	selectedTemplate,
	skipTemplate,
	onSelectTemplate,
	onSkipTemplate,
	onShowModal,
}: Readonly<{
	templates: [string, string, IMetadata | undefined][];
	selectedTemplate: [string, string];
	skipTemplate: boolean;
	onSelectTemplate: (appId: string, templateId: string) => void;
	onSkipTemplate: (skip: boolean) => void;
	onShowModal: () => void;
}>) {
	const templatesByApp = useTemplateFolders(templates);

	return (
		<Card className="border-2 hover:border-primary/20 transition-all duration-300">
			<CardHeader>
				<div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 sm:gap-4">
					<div className="flex items-start gap-3">
						<div className="p-2 bg-muted rounded-lg">
							<CopyIcon className="h-5 w-5 text-primary" />
						</div>
						<div className="flex-1 min-w-0">
							<CardTitle className="text-base sm:text-lg">Start With A Template?</CardTitle>
							<CardDescription className="text-xs sm:text-sm">
								Start with a pre-built template or create from scratch
							</CardDescription>
						</div>
					</div>
					<div className="flex items-center gap-2 sm:gap-3 flex-wrap w-full sm:w-auto">
						<Button
							variant="secondary"
							size="sm"
							onClick={onShowModal}
							className="gap-2 w-full sm:w-auto"
						>
							<Grid className="h-4 w-4" />
							Browse All
						</Button>
						<div className="flex items-center gap-2 ml-auto sm:ml-0">
							<Checkbox
								id="skip-template"
								checked={skipTemplate}
								onCheckedChange={(checked) => {
									onSkipTemplate(checked as boolean);
									if (checked) onSelectTemplate("", "");
								}}
							/>
							<Label htmlFor="skip-template" className="text-xs sm:text-sm text-muted-foreground cursor-pointer">
								Skip
							</Label>
						</div>
					</div>
				</div>
			</CardHeader>
			<CardContent className="space-y-4">
				{!skipTemplate && (
					<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3 sm:gap-4">
						{templatesByApp.slice(0, 4).map(([appId, templates]) => (
							<AppTemplateFolder
								key={appId}
								appId={appId}
								templates={templates}
								selectedTemplate={selectedTemplate}
								onSelectTemplate={onSelectTemplate}
							/>
						))}

						{templatesByApp.length > 3 && (
							<div className="text-center">
								<Button
									variant="ghost"
									onClick={onShowModal}
									className="text-sm text-muted-foreground hover:text-primary"
								>
									+{templatesByApp.length - 3} more template folders
								</Button>
							</div>
						)}
					</div>
				)}

				{skipTemplate && (
					<div className="text-center py-8 text-muted-foreground">
						<Copy className="h-12 w-12 mx-auto mb-4 opacity-50" />
						<p>Starting from a blank canvas</p>
					</div>
				)}
			</CardContent>
		</Card>
	);
}
