"use client";
import { Brain, Grid } from "lucide-react";
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
import { ModelCard } from "./model-card";

export function ModelSection({
	models,
	selectedModels,
	skipModels,
	onUpdateModels,
	onSkipModels,
	onShowModal,
}: Readonly<{
	models: string[];
	selectedModels: string[];
	skipModels: boolean;
	onUpdateModels: (models: string[]) => void;
	onSkipModels: (skip: boolean) => void;
	onShowModal: () => void;
}>) {
	return (
		<Card className="border-2 hover:border-primary/20 transition-all duration-300">
			<CardHeader>
				<div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 sm:gap-4">
					<div className="flex items-start gap-3">
						<div className="p-2 bg-muted rounded-lg">
							<Brain className="h-5 w-5 text-primary" />
						</div>
						<div className="min-w-0">
							<CardTitle className="text-base sm:text-lg">
								Embedding Models
							</CardTitle>
							<CardDescription className="text-xs sm:text-sm">
								Select models for semantic search and AI capabilities
							</CardDescription>
						</div>
					</div>
					<div className="flex items-center gap-2 sm:gap-3 flex-wrap w-full sm:w-auto">
						{!skipModels && (
							<Button
								variant="secondary"
								size="sm"
								onClick={onShowModal}
								className="gap-2 w-full sm:w-auto"
							>
								<Grid className="h-4 w-4" />
								Browse All
							</Button>
						)}
						<div className="flex items-center gap-2 ml-auto sm:ml-0">
							<Checkbox
								id="skip-models"
								checked={skipModels}
								onCheckedChange={(checked) => {
									onSkipModels(checked as boolean);
									if (checked) onUpdateModels([]);
								}}
							/>
							<Label
								htmlFor="skip-models"
								className="text-xs sm:text-sm text-muted-foreground cursor-pointer"
							>
								Skip
							</Label>
						</div>
					</div>
				</div>
			</CardHeader>
			<CardContent>
				{!skipModels ? (
					<div className="space-y-4">
						<div className="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
							{models.slice(0, 4).map((bit) => (
								<ModelCard
									key={bit}
									bitId={bit.split(":")[1]}
									hub={bit.split(":")[0]}
									selected={selectedModels.includes(bit.split(":")[1])}
									onToggle={(id) => {
										onUpdateModels(
											selectedModels.includes(id)
												? selectedModels.filter((m) => m !== id)
												: [...selectedModels, id],
										);
									}}
								/>
							))}
						</div>
						{models.length > 4 && (
							<div className="text-center">
								<Button
									variant="ghost"
									onClick={onShowModal}
									className="text-sm text-muted-foreground hover:text-primary"
								>
									+{models.length - 4} more models
								</Button>
							</div>
						)}
						{selectedModels.length > 0 && (
							<div className="text-center text-sm text-muted-foreground">
								{selectedModels.length} model
								{selectedModels.length !== 1 ? "s" : ""} selected
							</div>
						)}
					</div>
				) : (
					<div className="text-center py-8 text-muted-foreground">
						<Brain className="h-12 w-12 mx-auto mb-4 opacity-50" />
						<p>
							Model selection skipped - you can{" "}
							<span className="highlight">NOT</span> add models later
						</p>
					</div>
				)}
			</CardContent>
		</Card>
	);
}
