"use client";
import { Brain, Filter, Search, X } from "lucide-react";
import { useState } from "react";
import { Badge, Button, Input } from "../../..";
import { ModelCard } from "./model-card";

export function ModelModal({
	open,
	onClose,
	models,
	selectedModels,
	onUpdateModels,
}: Readonly<{
	open: boolean;
	onClose: () => void;
	models: string[];
	selectedModels: string[];
	onUpdateModels: (models: string[]) => void;
}>) {
	const [searchQuery, setSearchQuery] = useState("");
	const [typeFilter, setTypeFilter] = useState<string>("all");
	const [localSelectedModels, setLocalSelectedModels] =
		useState(selectedModels);

	if (!open) return null;

	const handleSave = () => {
		onUpdateModels(localSelectedModels);
		onClose();
	};

	const handleToggle = (id: string) => {
		setLocalSelectedModels((prev) =>
			prev.includes(id) ? prev.filter((m) => m !== id) : [...prev, id],
		);
	};

	return (
		<div className="fixed inset-0 z-50 bg-background/80 backdrop-blur-xs">
			<div className="fixed inset-2 sm:inset-4 bg-background border rounded-lg shadow-2xl flex flex-col max-h-[92dvh] sm:max-h-[90dvh]">
				<div className="flex items-center justify-between p-4 sm:p-6 border-b sticky top-0 bg-background/90 backdrop-blur supports-[backdrop-filter]:bg-background/60 z-10">
					<div className="flex items-center gap-3">
						<div className="p-2 bg-primary/10 rounded-lg">
							<Brain className="h-5 w-5 text-primary" />
						</div>
						<div>
							<h2 className="text-xl sm:text-2xl font-bold">Select Models</h2>
							<p className="text-muted-foreground text-sm sm:text-base">
								Choose embedding models for your app
							</p>
						</div>
					</div>
					<Button variant="ghost" size="sm" onClick={onClose}>
						<X className="h-4 w-4" />
					</Button>
				</div>

				<div className="p-4 sm:p-6 border-b space-y-4">
					<div className="relative">
						<Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
						<Input
							placeholder="Search models..."
							value={searchQuery}
							onChange={(e) => setSearchQuery(e.target.value)}
							className="pl-10"
						/>
					</div>
					<div className="flex items-center gap-2 flex-wrap">
						<Filter className="h-4 w-4 text-muted-foreground" />
						<Badge
							variant={typeFilter === "all" ? "default" : "outline"}
							className="cursor-pointer"
							onClick={() => setTypeFilter("all")}
						>
							All Types
						</Badge>
						<Badge
							variant={typeFilter === "embedding" ? "default" : "outline"}
							className="cursor-pointer"
							onClick={() => setTypeFilter("embedding")}
						>
							Text Embedding
						</Badge>
						<Badge
							variant={typeFilter === "image" ? "default" : "outline"}
							className="cursor-pointer"
							onClick={() => setTypeFilter("image")}
						>
							Image Embedding
						</Badge>
					</div>
				</div>

				<div className="flex-1 overflow-auto p-4 sm:p-6">
					<div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 sm:gap-4">
						{models.map((bit) => (
							<ModelCard
								key={bit}
								bitId={bit.split(":")[1]}
								hub={bit.split(":")[0]}
								selected={localSelectedModels.includes(bit.split(":")[1])}
								onToggle={handleToggle}
								searchQuery={searchQuery}
								typeFilter={typeFilter}
							/>
						))}
					</div>
				</div>

				<div className="p-4 sm:p-6 border-t flex items-center justify-between sticky bottom-0 bg-background/90 backdrop-blur supports-[backdrop-filter]:bg-background/60">
					<div className="text-sm text-muted-foreground">
						{localSelectedModels.length} model
						{localSelectedModels.length !== 1 ? "s" : ""} selected
					</div>
					<div className="flex gap-2">
						<Button variant="outline" onClick={onClose}>
							Cancel
						</Button>
						<Button onClick={handleSave}>Save Selection</Button>
					</div>
				</div>
			</div>
		</div>
	);
}
