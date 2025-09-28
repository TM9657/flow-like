"use client";
import {
	Check,
	FileSearch,
	Package2,
	PackageCheck,
	Plus,
	ScanEye,
} from "lucide-react";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Badge,
	Card,
	CardContent,
	IBitTypes,
	humanFileSize,
	useBackend,
	useInvoke,
} from "../../..";

export function ModelCard({
	bitId,
	hub,
	selected,
	onToggle,
	searchQuery = "",
	typeFilter = "all",
}: Readonly<{
	bitId: string;
	hub: string;
	selected: boolean;
	onToggle: (id: string) => void;
	searchQuery?: string;
	typeFilter?: string;
}>) {
	const backend = useBackend();
	const bitData = useInvoke(backend.bitState.getBit, backend.bitState, [
		bitId,
		hub,
	]);
	const isInstalled = useInvoke(
		backend.bitState.isBitInstalled,
		backend.bitState,
		[bitData.data!],
		!!bitData.data,
	);
	const bitSize = useInvoke(
		backend.bitState.getBitSize,
		backend.bitState,
		[bitData.data!],
		!!bitData.data,
	);

	if (!bitData.data) return null;
	if (
		bitData.data.type !== IBitTypes.Embedding &&
		bitData.data.type !== IBitTypes.ImageEmbedding
	)
		return null;

	if (
		searchQuery &&
		!bitData.data.meta?.en?.name
			.toLowerCase()
			.includes(searchQuery.toLowerCase())
	) {
		return null;
	}

	if (typeFilter !== "all") {
		if (typeFilter === "embedding" && bitData.data.type !== IBitTypes.Embedding)
			return null;
		if (
			typeFilter === "image" &&
			bitData.data.type !== IBitTypes.ImageEmbedding
		)
			return null;
	}

	const getTypeIcon = (type: IBitTypes) => {
		switch (type) {
			case IBitTypes.Embedding:
				return FileSearch;
			case IBitTypes.ImageEmbedding:
				return ScanEye;
			default:
				return Package2;
		}
	};

	const TypeIcon = getTypeIcon(bitData.data.type);

	return (
		<Card
			className={`group cursor-pointer transition-all duration-300 hover:shadow-md ${
				selected
					? "ring-2 ring-primary shadow-md shadow-primary/10 bg-linear-to-br from-primary/5 to-transparent"
					: "hover:border-primary/20"
			}`}
			onClick={() => onToggle(bitId)}
		>
			<CardContent className="p-3 sm:p-4">
				<div className="flex items-center gap-3 mb-3">
					<Avatar className="h-10 w-10 border border-border">
						<AvatarImage src={bitData.data.icon} />
						<AvatarFallback className="bg-linear-to-br from-primary/10 to-secondary/10">
							<TypeIcon className="h-5 w-5" />
						</AvatarFallback>
					</Avatar>
					<div className="flex-1 min-w-0">
						<h4 className="font-medium truncate text-sm">
							{bitData.data.meta?.en?.name}
						</h4>
						<div className="flex items-center gap-2 mt-1">
							<Badge variant="outline" className="text-xs">
								<TypeIcon className="h-3 w-3 mr-1" />
								{bitData.data.type}
							</Badge>
							{isInstalled.data && bitSize.data !== 0 && (
								<Badge
									variant="secondary"
									className="text-xs bg-emerald-100 text-emerald-700"
								>
									<PackageCheck className="h-3 w-3 mr-1" />
									{humanFileSize(bitSize.data ?? 0)}
								</Badge>
							)}
							{bitSize.data === 0 && (
								<Badge
									variant="secondary"
									className="text-xs bg-emerald-100 text-emerald-700"
								>
									<PackageCheck className="h-3 w-3 mr-1" />
									Bundled
								</Badge>
							)}
						</div>
					</div>
					<div className="flex items-center gap-2">
						{selected ? (
							<div className="p-1.5 rounded-full bg-primary text-primary-foreground border border-primary">
								<Check className="h-3 w-3" />
							</div>
						) : (
							<div className="p-1.5 rounded-full bg-muted text-muted-foreground border border-border group-hover:bg-muted/80">
								<Plus className="h-3 w-3" />
							</div>
						)}
					</div>
				</div>
				<p className="text-xs text-muted-foreground line-clamp-2">
					{bitData.data.meta?.en?.description}
				</p>
			</CardContent>
		</Card>
	);
}
