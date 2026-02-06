"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	Brain,
	ChevronDown,
	Code2,
	Cpu,
	DollarSign,
	FileSearchIcon,
	Filter,
	Globe,
	Grid3X3,
	ImageIcon,
	LayoutList,
	Lightbulb,
	Loader2,
	type LucideIcon,
	MessageSquare,
	PackageCheck,
	Search,
	Shield,
	Sparkles,
	Type,
	Wand2,
	X,
	Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useMiniSearch } from "react-minisearch";
import { useInvoke } from "../../../hooks/index";
import { Bit } from "../../../lib/bit/bit";
import type { IBit } from "../../../lib/schema/bit/bit";
import { IBitTypes } from "../../../lib/schema/bit/bit";
import type { ILlmParameters } from "../../../lib/schema/bit/bit/llm-parameters";
import { useBackend } from "../../../state/backend-state";
import {
	Button,
	Input,
	ModelCard,
	ModelDetailSheet,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Slider,
	formatContextLength,
} from "../../ui";
import { Badge } from "../../ui/badge";
import { Checkbox } from "../../ui/checkbox";
import {
	Sheet,
	SheetContent,
	SheetHeader,
	SheetTitle,
	SheetDescription,
} from "../../ui/sheet";

type SortOption =
	| "name"
	| "updated"
	| "context"
	| "speed"
	| "cost"
	| "reasoning"
	| "coding";
type ViewMode = "grid" | "list";
type InputModality = "text" | "image";
type OutputModality = "text" | "embedding";

function getBitModality(type: IBitTypes): {
	input: InputModality;
	output: OutputModality;
} {
	switch (type) {
		case IBitTypes.Llm:
			return { input: "text", output: "text" };
		case IBitTypes.Vlm:
			return { input: "image", output: "text" };
		case IBitTypes.Embedding:
			return { input: "text", output: "embedding" };
		case IBitTypes.ImageEmbedding:
			return { input: "image", output: "embedding" };
		default:
			return { input: "text", output: "text" };
	}
}

interface CapabilityInfo {
	icon: LucideIcon;
	label: string;
	color: string;
}

const capabilityIcons: Record<string, CapabilityInfo> = {
	coding: { icon: Code2, label: "Coding", color: "text-blue-500" },
	cost: { icon: DollarSign, label: "Cost Efficiency", color: "text-green-500" },
	creativity: {
		icon: Lightbulb,
		label: "Creativity",
		color: "text-yellow-500",
	},
	factuality: { icon: Shield, label: "Factuality", color: "text-emerald-500" },
	function_calling: {
		icon: Wand2,
		label: "Function Calling",
		color: "text-purple-500",
	},
	multilinguality: {
		icon: Globe,
		label: "Multilingual",
		color: "text-cyan-500",
	},
	reasoning: { icon: Brain, label: "Reasoning", color: "text-orange-500" },
	speed: { icon: Zap, label: "Speed", color: "text-amber-500" },
};

const LLM_LIKE_TYPES = new Set([IBitTypes.Llm, IBitTypes.Vlm]);

function isHostedModel(bit: IBit): boolean {
	return (bit.size ?? 0) === 0 || !bit.download_link;
}

interface AIModelPageProps {
	webMode?: boolean;
}

export function AIModelPage({ webMode = false }: AIModelPageProps) {
	const backend = useBackend();
	const profile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
	);
	const [searchTerm, setSearchTerm] = useState("");
	const [blacklist, setBlacklist] = useState(new Set<string>());
	const [viewMode, setViewMode] = useState<ViewMode>("grid");
	const [sortBy, setSortBy] = useState<SortOption>("updated");
	const [showFilters, setShowFilters] = useState(true);
	const [mobileFiltersOpen, setMobileFiltersOpen] = useState(false);
	const [providerFilter, setProviderFilter] = useState("all");
	const [contextLengthFilter, setContextLengthFilter] = useState<
		[number, number]
	>([0, 2000000]);
	const [showInProfileOnly, setShowInProfileOnly] = useState(false);
	const [showDownloadedOnly, setShowDownloadedOnly] = useState(false);
	const [selectedModel, setSelectedModel] = useState<IBit | null>(null);
	const [inputModalities, setInputModalities] = useState<Set<InputModality>>(
		new Set(["text", "image"]),
	);
	const [outputModalities, setOutputModalities] = useState<Set<OutputModality>>(
		new Set(["text", "embedding"]),
	);
	const [capabilityFilters, setCapabilityFilters] = useState<
		Record<string, number>
	>({
		reasoning: 0,
		coding: 0,
		speed: 0,
		cost: 0,
		creativity: 0,
		factuality: 0,
	});

	const checkInstalled = useCallback(
		async (bit: IBit) => {
			try {
				const result = await backend.bitState.isBitInstalled(bit);
				return result;
			} catch {
				return false;
			}
		},
		[backend.bitState],
	);

	const foundBits = useInvoke(
		backend.bitState.searchBits,
		backend.bitState,
		[
			{
				bit_types: [
					IBitTypes.Llm,
					IBitTypes.Vlm,
					IBitTypes.Embedding,
					IBitTypes.ImageEmbedding,
				],
			},
		],
		typeof profile.data !== "undefined",
		[profile.data?.id ?? ""],
	);

	const imageBlacklist = useCallback(async () => {
		if (!foundBits.data) return;
		const dependencies = await Promise.all(
			foundBits.data
				.filter((bit) => bit.type === IBitTypes.ImageEmbedding)
				.map((bit) =>
					Bit.fromObject(bit).setBackend(backend).fetchDependencies(),
				),
		);
		const bl = new Set<string>(
			dependencies.flatMap((dep) =>
				dep.bits
					.filter((bit) => bit.type !== IBitTypes.ImageEmbedding)
					.map((bit) => bit.id),
			),
		);
		setBlacklist(bl);
	}, [backend, foundBits.data]);

	const [installedBits, setInstalledBits] = useState<Set<string>>(new Set());

	const { search, searchResults, addAllAsync, removeAll } = useMiniSearch<IBit>(
		[],
		{
			fields: [
				"authors",
				"file_name",
				"hub",
				"id",
				"name",
				"long_description",
				"description",
				"type",
			],
			storeFields: ["id"],
			searchOptions: {
				fuzzy: true,
				boost: { name: 2, type: 1.5, description: 1, long_description: 0.5 },
			},
		},
	);

	useEffect(() => {
		if (!foundBits.data) return;
		imageBlacklist();
	}, [foundBits.data, imageBlacklist]);

	useEffect(() => {
		if (!foundBits.data || !profile.data || !checkInstalled) return;
		const checkInstalledAll = async () => {
			const installedSet = new Set<string>();
			for (const bit of foundBits.data) {
				const isInstalled = await checkInstalled(bit);
				if (isInstalled) installedSet.add(bit.id);
			}
			setInstalledBits(installedSet);
		};
		checkInstalledAll();
	}, [foundBits.data, profile.data, checkInstalled]);

	useEffect(() => {
		if (!foundBits.data) return;
		removeAll();
		addAllAsync(
			foundBits.data.map((item) => ({
				...item,
				name: item.meta?.en?.name,
				long_description: item.meta?.en?.long_description,
				description: item.meta?.en?.description,
			})),
		);
	}, [foundBits.data, addAllAsync, removeAll]);

	const providers = useMemo(() => {
		if (!foundBits.data) return [];
		const providerSet = new Set<string>();
		for (const model of foundBits.data) {
			const params = model.parameters as ILlmParameters | undefined;
			if (params?.provider?.provider_name) {
				providerSet.add(params.provider.provider_name);
			}
		}
		return Array.from(providerSet).sort();
	}, [foundBits.data]);

	const maxContextLength = useMemo(() => {
		if (!foundBits.data) return 2000000;
		return Math.max(
			...foundBits.data.map(
				(m) =>
					(m.parameters as ILlmParameters | undefined)?.context_length ?? 0,
			),
			128000,
		);
	}, [foundBits.data]);

	const profileBitIds = useMemo(() => {
		return new Set(profile.data?.bits?.map((id) => id.split(":").pop()) ?? []);
	}, [profile.data]);

	const filteredModels = useMemo(() => {
		let models = searchTerm.trim()
			? ((searchResults as IBit[]) ?? [])
			: (foundBits.data ?? []);
		models = models.filter((bit) => !blacklist.has(bit.id));
		models = models.filter((bit) => bit.meta?.en !== undefined);

		// In web mode, filter LLM/VLM to only show hosted models
		if (webMode) {
			models = models.filter((bit) => {
				if (LLM_LIKE_TYPES.has(bit.type)) {
					return isHostedModel(bit);
				}
				return true;
			});
		}

		if (inputModalities.size < 2 || outputModalities.size < 2) {
			models = models.filter((m) => {
				const bitModality = getBitModality(m.type);
				const inputMatch =
					inputModalities.size === 0 || inputModalities.has(bitModality.input);
				const outputMatch =
					outputModalities.size === 0 ||
					outputModalities.has(bitModality.output);
				return inputMatch && outputMatch;
			});
		}

		if (showInProfileOnly)
			models = models.filter((m) => profileBitIds.has(m.id));
		if (showDownloadedOnly)
			models = models.filter((m) => installedBits.has(m.id));

		if (providerFilter !== "all") {
			models = models.filter((m) => {
				const params = m.parameters as ILlmParameters | undefined;
				return params?.provider?.provider_name === providerFilter;
			});
		}

		models = models.filter((m) => {
			const params = m.parameters as ILlmParameters | undefined;
			const contextLength = params?.context_length ?? 0;
			return (
				contextLength >= contextLengthFilter[0] &&
				contextLength <= contextLengthFilter[1]
			);
		});

		models = models.filter((m) => {
			const params = m.parameters as ILlmParameters | undefined;
			const classification = params?.model_classification;
			if (!classification) return true;
			for (const [key, minValue] of Object.entries(capabilityFilters)) {
				if (minValue > 0) {
					const modelValue =
						classification[key as keyof typeof classification] ?? 0;
					if (modelValue < minValue) return false;
				}
			}
			return true;
		});

		models.sort((a, b) => {
			const aParams = a.parameters as ILlmParameters | undefined;
			const bParams = b.parameters as ILlmParameters | undefined;
			switch (sortBy) {
				case "name":
					return (a.meta?.en?.name || a.id).localeCompare(
						b.meta?.en?.name || b.id,
					);
				case "updated":
					return Date.parse(b.updated) - Date.parse(a.updated);
				case "context":
					return (
						(bParams?.context_length ?? 0) - (aParams?.context_length ?? 0)
					);
				case "speed":
					return (
						(bParams?.model_classification?.speed ?? 0) -
						(aParams?.model_classification?.speed ?? 0)
					);
				case "cost":
					return (
						(bParams?.model_classification?.cost ?? 0) -
						(aParams?.model_classification?.cost ?? 0)
					);
				case "reasoning":
					return (
						(bParams?.model_classification?.reasoning ?? 0) -
						(aParams?.model_classification?.reasoning ?? 0)
					);
				case "coding":
					return (
						(bParams?.model_classification?.coding ?? 0) -
						(aParams?.model_classification?.coding ?? 0)
					);
				default:
					return 0;
			}
		});

		return models;
	}, [
		foundBits.data,
		searchResults,
		searchTerm,
		inputModalities,
		outputModalities,
		providerFilter,
		contextLengthFilter,
		showInProfileOnly,
		showDownloadedOnly,
		profileBitIds,
		installedBits,
		blacklist,
		sortBy,
		capabilityFilters,
		webMode,
	]);

	const modalityCounts = useMemo(() => {
		const counts = { text: 0, image: 0, embedding: 0, total: 0 };
		if (!foundBits.data) return counts;
		const validBits = foundBits.data.filter((bit) => !blacklist.has(bit.id));
		counts.total = validBits.length;
		for (const bit of validBits) {
			const modality = getBitModality(bit.type);
			if (modality.input === "text") counts.text++;
			if (modality.input === "image") counts.image++;
			if (modality.output === "embedding") counts.embedding++;
		}
		return counts;
	}, [foundBits.data, blacklist]);

	const activeFilterCount = useMemo(() => {
		let count = 0;
		if (providerFilter !== "all") count++;
		if (showInProfileOnly) count++;
		if (showDownloadedOnly) count++;
		if (contextLengthFilter[0] > 0 || contextLengthFilter[1] < maxContextLength)
			count++;
		if (inputModalities.size < 2) count++;
		if (outputModalities.size < 2) count++;
		if (Object.values(capabilityFilters).some((v) => v > 0)) count++;
		return count;
	}, [
		providerFilter,
		showInProfileOnly,
		showDownloadedOnly,
		contextLengthFilter,
		maxContextLength,
		inputModalities,
		outputModalities,
		capabilityFilters,
	]);

	const toggleInputModality = useCallback((modality: InputModality) => {
		setInputModalities((prev) => {
			const next = new Set(prev);
			if (next.has(modality)) next.delete(modality);
			else next.add(modality);
			return next;
		});
	}, []);

	const toggleOutputModality = useCallback((modality: OutputModality) => {
		setOutputModalities((prev) => {
			const next = new Set(prev);
			if (next.has(modality)) next.delete(modality);
			else next.add(modality);
			return next;
		});
	}, []);

	const resetFilters = useCallback(() => {
		setProviderFilter("all");
		setShowInProfileOnly(false);
		setShowDownloadedOnly(false);
		setContextLengthFilter([0, maxContextLength]);
		setInputModalities(new Set(["text", "image"]));
		setOutputModalities(new Set(["text", "embedding"]));
		setCapabilityFilters({
			reasoning: 0,
			coding: 0,
			speed: 0,
			cost: 0,
			creativity: 0,
			factuality: 0,
		});
	}, [maxContextLength]);

	const sidebarContent = (
		<div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-5">
						<FilterSection title="Input" icon={MessageSquare}>
							<FilterCheckbox
								checked={inputModalities.has("text")}
								onCheckedChange={() => toggleInputModality("text")}
								icon={Type}
								iconColor="text-blue-500"
								label="Text"
								count={modalityCounts.text}
							/>
							<FilterCheckbox
								checked={inputModalities.has("image")}
								onCheckedChange={() => toggleInputModality("image")}
								icon={ImageIcon}
								iconColor="text-purple-500"
								label="Image"
								count={modalityCounts.image}
							/>
						</FilterSection>

						<FilterSection
							title="Output"
							icon={MessageSquare}
							className="-scale-x-100"
						>
							<FilterCheckbox
								checked={outputModalities.has("text")}
								onCheckedChange={() => toggleOutputModality("text")}
								icon={Type}
								iconColor="text-green-500"
								label="Text"
							/>
							<FilterCheckbox
								checked={outputModalities.has("embedding")}
								onCheckedChange={() => toggleOutputModality("embedding")}
								icon={FileSearchIcon}
								iconColor="text-orange-500"
								label="Embedding"
								count={modalityCounts.embedding}
							/>
						</FilterSection>

						<FilterSection title="Status" icon={Filter}>
							<FilterCheckbox
								checked={showInProfileOnly}
								onCheckedChange={(c) => setShowInProfileOnly(!!c)}
								icon={Sparkles}
								iconColor="text-primary"
								label="In Profile"
							/>
							{!webMode && (
								<FilterCheckbox
									checked={showDownloadedOnly}
									onCheckedChange={(c) => setShowDownloadedOnly(!!c)}
									icon={PackageCheck}
									iconColor="text-emerald-500"
									label="Downloaded"
								/>
							)}
						</FilterSection>

						{providers.length > 0 && (
							<FilterSection title="Provider" icon={Globe}>
								<Select
									value={providerFilter}
									onValueChange={setProviderFilter}
								>
									<SelectTrigger className="h-8 text-xs">
										<SelectValue placeholder="All providers" />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value="all">All providers</SelectItem>
										{providers.map((provider) => (
											<SelectItem key={provider} value={provider}>
												{provider}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</FilterSection>
						)}

						<div className="space-y-2">
							<div className="flex items-center justify-between px-2">
								<p className="text-xs font-medium text-muted-foreground uppercase tracking-wider flex items-center gap-2">
									<Cpu className="h-3 w-3" />
									Context
								</p>
								<span className="text-[10px] text-muted-foreground">
									{formatContextLength(contextLengthFilter[0])} -{" "}
									{formatContextLength(contextLengthFilter[1])}
								</span>
							</div>
							<div className="px-2 pb-2">
								<Slider
									value={contextLengthFilter}
									onValueChange={(v) =>
										setContextLengthFilter(v as [number, number])
									}
									min={0}
									max={maxContextLength}
									step={1000}
								/>
							</div>
						</div>

						<div className="space-y-3">
							<button
								type="button"
								onClick={() => setShowFilters(!showFilters)}
								className="flex items-center gap-2 text-xs font-medium text-muted-foreground uppercase tracking-wider px-2 w-full hover:text-foreground transition-colors"
							>
								<Brain className="h-3 w-3" />
								<span>Capabilities</span>
								{Object.values(capabilityFilters).some((v) => v > 0) && (
									<Badge variant="default" className="h-4 px-1 text-[9px] ml-1">
										Active
									</Badge>
								)}
								<ChevronDown
									className={`h-3 w-3 ml-auto transition-transform ${showFilters ? "rotate-180" : ""}`}
								/>
							</button>

							<AnimatePresence>
								{showFilters && (
									<motion.div
										initial={{ height: 0, opacity: 0 }}
										animate={{ height: "auto", opacity: 1 }}
										exit={{ height: 0, opacity: 0 }}
										className="space-y-5 overflow-hidden px-2 pb-2"
									>
										{Object.entries(capabilityIcons)
											.slice(0, 6)
											.map(([key, info]) => {
												const Icon = info.icon;
												const value = capabilityFilters[key] ?? 0;
												return (
													<div key={key} className="space-y-2">
														<div className="flex items-center justify-between text-xs">
															<div
																className={`flex items-center gap-1.5 ${info.color}`}
															>
																<Icon className="h-3 w-3" />
																<span>{info.label}</span>
															</div>
															<span className="text-muted-foreground">
																{value > 0
																	? `≥${Math.round(value * 100)}%`
																	: "Any"}
															</span>
														</div>
														<Slider
															value={[value]}
															onValueChange={([v]) =>
																setCapabilityFilters((prev) => ({
																	...prev,
																	[key]: v,
																}))
															}
															min={0}
															max={1}
															step={0.1}
															className="h-1"
														/>
													</div>
												);
											})}
									</motion.div>
								)}
							</AnimatePresence>
						</div>

						{activeFilterCount > 0 && (
							<div className="px-2 pt-2">
								<Button
									variant="outline"
									size="sm"
									className="w-full h-8 text-xs"
									onClick={resetFilters}
								>
									<X className="h-3 w-3 mr-1.5" />
									Clear {activeFilterCount} Filter
									{activeFilterCount !== 1 ? "s" : ""}
								</Button>
					</div>
				)}
		</div>
	);

	return (
		<main className="flex grow h-full min-h-0 overflow-hidden flex-col w-full -m-4 sm:m-0 sm:rounded-lg sm:border sm:border-border/40">
			<div className="flex flex-1 min-h-0 overflow-hidden">
				{/* Desktop Sidebar */}
				<div className="hidden lg:flex w-64 border-r border-border/40 flex-col bg-muted/10 min-h-0">
					<div className="p-4 border-b border-border/40 shrink-0">
						<div className="flex items-center gap-2 mb-1">
							<Sparkles className="h-5 w-5 text-primary" />
							<h1 className="text-lg font-bold">Model Catalog</h1>
						</div>
						<p className="text-xs text-muted-foreground">
							{modalityCounts.total} models available
						</p>
					</div>
					{sidebarContent}
				</div>

				{/* Mobile Filter Sheet */}
				<Sheet open={mobileFiltersOpen} onOpenChange={setMobileFiltersOpen}>
					<SheetContent side="left" className="w-72 p-0 lg:hidden">
						<SheetHeader className="p-4 border-b border-border/40">
							<SheetTitle className="flex items-center gap-2">
								<Filter className="h-4 w-4" />
								Filters
							</SheetTitle>
							<SheetDescription className="text-xs">
								{modalityCounts.total} models available
							</SheetDescription>
						</SheetHeader>
						{sidebarContent}
					</SheetContent>
				</Sheet>

				{/* Main Content */}
				<div className="flex-1 flex flex-col min-h-0 min-w-0 overflow-hidden">
					<div className="p-3 sm:p-4 border-b border-border/40 space-y-3 bg-background/80 backdrop-blur-sm shrink-0">
						{/* Mobile header */}
						<div className="flex items-center gap-2 lg:hidden">
							<Sparkles className="h-5 w-5 text-primary" />
							<h1 className="text-lg font-bold">Model Catalog</h1>
						</div>

						<div className="flex flex-wrap items-center gap-2 sm:gap-3">
							{/* Mobile filter toggle */}
							<Button
								variant="outline"
								size="icon"
								className="relative h-10 w-10 lg:hidden shrink-0"
								onClick={() => setMobileFiltersOpen(true)}
							>
								<Filter className="h-4 w-4" />
								{activeFilterCount > 0 && (
									<span className="absolute -top-1 -right-1 h-4 w-4 rounded-full bg-primary text-[10px] text-primary-foreground flex items-center justify-center">
										{activeFilterCount}
									</span>
								)}
							</Button>

							<div className="relative flex-1 min-w-37.5 max-w-lg">
								<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
								<Input
									placeholder="Search models..."
									value={searchTerm}
									onChange={(e) => {
										setSearchTerm(e.target.value);
										search(e.target.value);
									}}
									className="pl-10 h-10"
								/>
							</div>

							<Select
								value={sortBy}
								onValueChange={(v) => setSortBy(v as SortOption)}
							>
								<SelectTrigger className="w-32 sm:w-44 h-10 shrink-0">
									<SelectValue placeholder="Sort by" />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="updated">Recently Updated</SelectItem>
									<SelectItem value="name">Name</SelectItem>
									<SelectItem value="context">Context Length</SelectItem>
									<SelectItem value="speed">Speed</SelectItem>
									<SelectItem value="cost">Cost Efficiency</SelectItem>
									<SelectItem value="reasoning">Reasoning</SelectItem>
									<SelectItem value="coding">Coding</SelectItem>
								</SelectContent>
							</Select>

							<div className="flex items-center border rounded-lg p-0.5 bg-muted/30">
								<Button
									variant={viewMode === "grid" ? "secondary" : "ghost"}
									size="icon"
									className="h-8 w-8"
									onClick={() => setViewMode("grid")}
								>
									<Grid3X3 className="h-4 w-4" />
								</Button>
								<Button
									variant={viewMode === "list" ? "secondary" : "ghost"}
									size="icon"
									className="h-8 w-8"
									onClick={() => setViewMode("list")}
								>
									<LayoutList className="h-4 w-4" />
								</Button>
							</div>
						</div>

						<div className="flex items-center justify-between text-sm">
							<span className="text-muted-foreground">
								{searchTerm
									? `${filteredModels.length} results for "${searchTerm}"`
									: `${filteredModels.length} models`}
							</span>
							{activeFilterCount > 0 && (
								<Badge variant="outline" className="text-xs">
									{activeFilterCount} filter{activeFilterCount !== 1 ? "s" : ""}{" "}
									applied
								</Badge>
							)}
						</div>
					</div>

					<div className="flex-1 min-h-0 overflow-y-auto">
						<div className="p-2 sm:p-4">
							{foundBits.isLoading ? (
								<div className="flex flex-col items-center justify-center py-16">
									<Loader2 className="h-8 w-8 animate-spin text-primary mb-4" />
									<p className="text-sm text-muted-foreground">
										Loading models...
									</p>
								</div>
							) : filteredModels.length === 0 ? (
								<div className="flex flex-col items-center justify-center py-16">
									<div className="h-16 w-16 rounded-full bg-muted/50 flex items-center justify-center mb-4">
										<Search className="h-8 w-8 text-muted-foreground/50" />
									</div>
									<p className="text-sm font-medium text-muted-foreground">
										No models found
									</p>
									<p className="text-xs text-muted-foreground/60 mt-1">
										Try adjusting your filters
									</p>
									{(searchTerm || activeFilterCount > 0) && (
										<Button
											variant="outline"
											size="sm"
											className="mt-4"
											onClick={() => {
												setSearchTerm("");
												search("");
												resetFilters();
											}}
										>
											Clear all filters
										</Button>
									)}
								</div>
							) : (
								<div
									className={
										viewMode === "grid"
										? "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 gap-3"
											: "space-y-2"
									}
								>
									<AnimatePresence mode="popLayout">
										{filteredModels.map((bit, index) => (
											<motion.div
												key={bit.id}
												initial={{ opacity: 0, y: 8 }}
												animate={{ opacity: 1, y: 0 }}
												exit={{ opacity: 0, y: -8 }}
												transition={{ delay: index * 0.01, duration: 0.15 }}
											>
												<ModelCard
													bit={bit}
													variant={viewMode}
													onClick={() => setSelectedModel(bit)}
												/>
											</motion.div>
										))}
									</AnimatePresence>
								</div>
							)}
						</div>
					</div>
				</div>
			</div>

			<ModelDetailSheet
				bit={selectedModel}
				open={selectedModel !== null}
				onOpenChange={(open) => !open && setSelectedModel(null)}
				webMode={webMode}
			/>
		</main>
	);

	return null;
}

function FilterSection({
	title,
	icon: Icon,
	children,
	className,
}: {
	title: string;
	icon: LucideIcon;
	children: React.ReactNode;
	className?: string;
}) {
	return (
		<div className="space-y-2">
			<p className="text-xs font-medium text-muted-foreground uppercase tracking-wider px-2 flex items-center gap-2">
				<Icon className={"h-3 w-3 " + (className ?? "")} />
				{title}
			</p>
			<div className="space-y-1.5 px-2">{children}</div>
		</div>
	);
}

interface FilterCheckboxProps {
	checked: boolean;
	onCheckedChange: (checked: boolean | "indeterminate") => void;
	icon: LucideIcon;
	iconColor: string;
	label: string;
	count?: number;
}

function FilterCheckbox({
	checked,
	onCheckedChange,
	icon: Icon,
	iconColor,
	label,
	count,
}: FilterCheckboxProps) {
	return (
		<label className="flex items-center gap-2.5 text-sm cursor-pointer hover:text-foreground transition-colors">
			<Checkbox checked={checked} onCheckedChange={onCheckedChange} />
			<Icon className={`h-3.5 w-3.5 ${iconColor}`} />
			<span>{label}</span>
			{count !== undefined && (
				<Badge variant="secondary" className="ml-auto h-5 px-1.5 text-[10px]">
					{count}
				</Badge>
			)}
		</label>
	);
}
