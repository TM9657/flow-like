"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	ArrowRight,
	Bot,
	ChevronRight,
	Clock,
	CommandIcon,
	FolderOpen,
	Hash,
	type LucideIcon,
	Sparkles,
	TrendingUp,
	Zap,
} from "lucide-react";
import MiniSearch from "minisearch";
import * as React from "react";
import { cn } from "../../lib/utils";
import {
	type SpotlightItem,
	useSpotlightStore,
} from "../../state/spotlight-state";
import { Avatar, AvatarFallback, AvatarImage } from "../ui/avatar";
import { Badge } from "../ui/badge";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	CommandShortcut,
} from "../ui/command";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";
import { FloatingOrbs } from "../ui/flow-background";
import { SpotlightFlowPilot } from "./spotlight-flowpilot";
import { QuickProjectCreate } from "./spotlight-quick-create";

const typeIcons: Record<string, LucideIcon> = {
	navigation: ArrowRight,
	project: FolderOpen,
	action: Zap,
	dynamic: Sparkles,
	recent: Clock,
};

const typeLabels: Record<string, string> = {
	navigation: "Go to",
	project: "Projects",
	action: "Actions",
	dynamic: "Context",
	recent: "Recent",
};

function highlightMatch(text: string, query: string): React.ReactNode {
	if (!query.trim()) return text;

	const regex = new RegExp(
		`(${query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`,
		"gi",
	);
	const parts = text.split(regex);

	return parts.map((part, i) =>
		regex.test(part) ? (
			<mark key={i} className="bg-primary/20 text-primary rounded-sm px-0.5">
				{part}
			</mark>
		) : (
			part
		),
	);
}

interface SpotlightDialogProps {
	className?: string;
	onFlowPilotMessage?: (message: string) => Promise<string>;
	onQuickCreateProject?: (
		name: string,
		isOffline: boolean,
	) => Promise<{ appId: string; boardId: string } | null>;
}

export function SpotlightDialog({
	className,
	onFlowPilotMessage,
	onQuickCreateProject,
}: SpotlightDialogProps) {
	const {
		isOpen,
		close,
		searchQuery,
		setSearchQuery,
		getAllItems,
		recentItems,
		groups,
		recordItemUsage,
		frecencyData,
		mode,
		setMode,
	} = useSpotlightStore();

	const allItems = React.useMemo(() => getAllItems(), [getAllItems, isOpen]);

	const recentItemsData = React.useMemo(() => {
		return recentItems
			.slice(0, 3)
			.map((id) => allItems.find((item) => item.id === id))
			.filter(Boolean) as SpotlightItem[];
	}, [recentItems, allItems]);

	const suggestedItems = React.useMemo(() => {
		const sorted = [...allItems].sort((a, b) => {
			const countA = frecencyData[a.id]?.count ?? 0;
			const countB = frecencyData[b.id]?.count ?? 0;
			return countB - countA;
		});
		return sorted
			.slice(0, 3)
			.filter((item) => (frecencyData[item.id]?.count ?? 0) > 0);
	}, [allItems, frecencyData]);

	interface SearchableItem {
		id: string;
		label: string;
		description: string;
		keywords: string;
		group: string;
		parentId?: string;
		parentLabel?: string;
		parentIconUrl?: string;
	}

	const { searchIndex, itemMap } = React.useMemo(() => {
		const index = new MiniSearch<SearchableItem>({
			fields: ["label", "keywords", "description", "group"],
			storeFields: ["id", "parentId", "parentLabel", "parentIconUrl"],
			searchOptions: {
				prefix: true,
				fuzzy: 0.2,
				boost: { label: 3, keywords: 2, description: 1, group: 0.5 },
			},
		});

		const map = new Map<string, SpotlightItem>();
		const docs: SearchableItem[] = [];

		for (const item of allItems) {
			map.set(item.id, item);
			docs.push({
				id: item.id,
				label: item.label,
				description: item.description || "",
				keywords: item.keywords?.join(" ") || "",
				group: item.group || item.type,
			});

			if (item.subItems && item.subItems.length > 0) {
				for (const subItem of item.subItems) {
					const subId = `${item.id}__${subItem.id}`;
					map.set(subId, subItem);
					docs.push({
						id: subId,
						label: subItem.label,
						description: subItem.description || "",
						keywords: subItem.keywords?.join(" ") || "",
						group: subItem.group || subItem.type,
						parentId: item.id,
						parentLabel: item.label,
						parentIconUrl: item.iconUrl,
					});
				}
			}
		}

		index.addAll(docs);
		return { searchIndex: index, itemMap: map };
	}, [allItems]);

	const filteredItems = React.useMemo(() => {
		if (!searchQuery.trim()) return allItems;

		const results = searchIndex.search(searchQuery, {
			prefix: true,
			fuzzy: 0.2,
			combineWith: "OR",
		});

		const scoredItems: Array<{ item: SpotlightItem; score: number }> = [];

		for (const result of results) {
			const item = itemMap.get(result.id);
			if (!item) continue;

			const frecencyBoost = frecencyData[item.id]?.score ?? 0;
			const totalScore = result.score + frecencyBoost;

			const parentId = (result as unknown as SearchableItem).parentId;
			const parentLabel = (result as unknown as SearchableItem).parentLabel;
			const parentIconUrl = (result as unknown as SearchableItem).parentIconUrl;

			if (parentId && parentLabel) {
				scoredItems.push({
					item: {
						...item,
						label: `${parentLabel} → ${item.label}`,
						iconUrl: parentIconUrl || item.iconUrl,
					},
					score: totalScore,
				});
			} else {
				scoredItems.push({ item, score: totalScore });
			}
		}

		return scoredItems.sort((a, b) => b.score - a.score).map((r) => r.item);
	}, [searchQuery, searchIndex, itemMap, frecencyData, allItems]);

	const groupedItems = React.useMemo(() => {
		const grouped = new Map<string, SpotlightItem[]>();

		for (const item of filteredItems) {
			const groupKey = item.group || item.type;
			if (!grouped.has(groupKey)) {
				grouped.set(groupKey, []);
			}
			grouped.get(groupKey)!.push(item);
		}

		const sortedGroups = [...grouped.entries()].sort(([a], [b]) => {
			const groupA = groups.find((g) => g.id === a);
			const groupB = groups.find((g) => g.id === b);
			return (groupB?.priority ?? 0) - (groupA?.priority ?? 0);
		});

		return sortedGroups;
	}, [filteredItems, groups]);

	const handleSelect = React.useCallback(
		async (item: SpotlightItem) => {
			if (item.disabled) return;

			recordItemUsage(item.id);

			if (!item.keepOpen) {
				close();
			}

			try {
				await item.action();
			} catch (error) {
				console.error("Spotlight action failed:", error);
			}
		},
		[recordItemUsage, close],
	);

	const getGroupLabel = (groupKey: string) => {
		const customGroup = groups.find((g) => g.id === groupKey);
		if (customGroup) return customGroup.label;
		return typeLabels[groupKey] || groupKey;
	};

	const renderIcon = (item: SpotlightItem) => {
		if (item.iconUrl) {
			return (
				<Avatar className="h-6 w-6 rounded-md">
					<AvatarImage
						src={item.iconUrl}
						alt={item.label}
						className="object-cover"
					/>
					<AvatarFallback className="text-[10px] rounded-md bg-gradient-to-br from-primary/20 to-primary/5">
						{item.label.substring(0, 2).toUpperCase()}
					</AvatarFallback>
				</Avatar>
			);
		}

		if (item.icon) {
			if (React.isValidElement(item.icon)) {
				return item.icon;
			}
			const IconComponent = item.icon as LucideIcon;
			return <IconComponent className="h-4 w-4" />;
		}

		const TypeIcon = typeIcons[item.type] || Hash;
		return <TypeIcon className="h-4 w-4 text-muted-foreground" />;
	};

	const showSuggestions = !searchQuery && suggestedItems.length > 0;
	const showRecent = !searchQuery && recentItemsData.length > 0;
	const [showFlowPilot, setShowFlowPilot] = React.useState(false);

	const renderModeContent = () => {
		// Quick Create mode - inline form
		if (mode === "quick-create" && onQuickCreateProject) {
			return <QuickProjectCreate onCreateProject={onQuickCreateProject} />;
		}

		// FlowPilot mode - full panel replacing search
		if (showFlowPilot && onFlowPilotMessage) {
			return (
				<div className="flex flex-col min-h-[400px] max-h-[60vh] sm:max-h-[500px]">
					<SpotlightFlowPilot
						onSendMessage={onFlowPilotMessage}
						embedded
						onClose={() => setShowFlowPilot(false)}
					/>
				</div>
			);
		}

		return (
			<Command
				className="[&_[cmdk-group-heading]]:text-muted-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group]]:px-2 w-full"
				shouldFilter={false}
			>
				<div className="relative flex items-center border-b border-border/40">
					<CommandInput
						placeholder="Search commands, projects, docs..."
						value={searchQuery}
						onValueChange={setSearchQuery}
						className="flex h-14 sm:h-12 w-full rounded-md bg-transparent px-4 py-3 text-base sm:text-sm outline-none placeholder:text-muted-foreground/60 disabled:cursor-not-allowed disabled:opacity-50 pr-24"
					/>
					<div className="absolute right-4 hidden sm:flex items-center gap-2">
						{onFlowPilotMessage && (
							<button
								type="button"
								onClick={() => setShowFlowPilot(true)}
								className="flex items-center justify-center h-7 w-7 rounded-md transition-colors bg-muted/50 text-muted-foreground hover:bg-violet-500/10 hover:text-violet-500"
								title="Open FlowPilot"
							>
								<Bot className="h-4 w-4" />
							</button>
						)}
						<Badge
							variant="secondary"
							className="text-[10px] px-1.5 py-0.5 h-5 font-mono bg-muted/50"
						>
							<CommandIcon className="h-3 w-3 mr-0.5" />K
						</Badge>
					</div>
				</div>

				<div className="max-h-[60vh] sm:max-h-[400px] overflow-y-auto overscroll-contain">
					<CommandList className="max-h-none py-2">
						<AnimatePresence mode="popLayout">
							<CommandEmpty className="py-16 text-center" asChild>
								<motion.div
									initial={{ opacity: 0, y: 10 }}
									animate={{ opacity: 1, y: 0 }}
									exit={{ opacity: 0, y: -10 }}
									className="flex flex-col items-center gap-3"
								>
									<div className="h-12 w-12 rounded-full bg-muted/50 flex items-center justify-center">
										<Sparkles className="h-6 w-6 text-muted-foreground/50" />
									</div>
									<div>
										<p className="text-sm font-medium text-muted-foreground">
											No results found
										</p>
										<p className="text-xs text-muted-foreground/60 mt-1">
											Try a different search term
										</p>
									</div>
								</motion.div>
							</CommandEmpty>

							{showSuggestions && (
								<motion.div
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									exit={{ opacity: 0 }}
									key="suggestions"
								>
									<CommandGroup
										heading={
											<span className="flex items-center gap-1.5">
												<TrendingUp className="h-3 w-3" />
												Suggested
											</span>
										}
									>
										{suggestedItems.map((item, index) => (
											<SpotlightCommandItem
												key={`suggested-${item.id}`}
												item={item}
												onSelect={handleSelect}
												renderIcon={renderIcon}
												searchQuery=""
												index={index}
											/>
										))}
									</CommandGroup>
								</motion.div>
							)}

							{showRecent && (
								<motion.div
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									exit={{ opacity: 0 }}
									key="recent"
								>
									<CommandGroup
										heading={
											<span className="flex items-center gap-1.5">
												<Clock className="h-3 w-3" />
												Recent
											</span>
										}
									>
										{recentItemsData.map((item, index) => (
											<SpotlightCommandItem
												key={`recent-${item.id}`}
												item={item}
												onSelect={handleSelect}
												renderIcon={renderIcon}
												searchQuery=""
												index={index}
											/>
										))}
									</CommandGroup>
								</motion.div>
							)}

							{groupedItems.map(([groupKey, items]) => (
								<motion.div
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									exit={{ opacity: 0 }}
									key={groupKey}
								>
									<CommandGroup heading={getGroupLabel(groupKey)}>
										{items.map((item, index) => (
											<SpotlightCommandItem
												key={item.id}
												item={item}
												onSelect={handleSelect}
												renderIcon={renderIcon}
												searchQuery={searchQuery}
												index={index}
											/>
										))}
									</CommandGroup>
								</motion.div>
							))}
						</AnimatePresence>
					</CommandList>
				</div>

				<div className="border-t border-border/40 px-4 py-2.5 text-[10px] text-muted-foreground/80 flex items-center justify-between bg-muted/20">
					<div className="hidden sm:flex items-center gap-4">
						<span className="flex items-center gap-1.5">
							<kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded-md border bg-background px-1.5 font-mono text-[10px] font-medium text-muted-foreground shadow-sm">
								↑↓
							</kbd>
							<span>Navigate</span>
						</span>
						<span className="flex items-center gap-1.5">
							<kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded-md border bg-background px-1.5 font-mono text-[10px] font-medium text-muted-foreground shadow-sm">
								↵
							</kbd>
							<span>Select</span>
						</span>
						<span className="flex items-center gap-1.5">
							<kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded-md border bg-background px-1.5 font-mono text-[10px] font-medium text-muted-foreground shadow-sm">
								esc
							</kbd>
							<span>Close</span>
						</span>
					</div>
					<div className="sm:hidden flex items-center gap-2">
						<span>Swipe down to close</span>
					</div>
					<span className="text-muted-foreground/50 flex items-center gap-1">
						<Sparkles className="h-3 w-3" />
						Flow-Like
					</span>
				</div>
			</Command>
		);
	};

	return (
		<Dialog open={isOpen} onOpenChange={(open) => !open && close()}>
			<DialogHeader className="sr-only">
				<DialogTitle>Command Palette</DialogTitle>
				<DialogDescription>
					Search for commands, projects, and actions
				</DialogDescription>
			</DialogHeader>
			<DialogContent
				className={cn(
					"overflow-hidden p-0 shadow-2xl",
					"w-[calc(100vw-2rem)]",
					"max-w-xl sm:max-w-2xl",
					"bg-background/98 backdrop-blur-2xl",
					"border border-border/40",
					"rounded-xl sm:rounded-2xl",
					"transition-all duration-200",
					className,
				)}
				showCloseButton={false}
			>
				<FloatingOrbs count={4} className="opacity-50" />
				<div className="relative z-10">{renderModeContent()}</div>
			</DialogContent>
		</Dialog>
	);
}

interface SpotlightCommandItemProps {
	item: SpotlightItem;
	onSelect: (item: SpotlightItem) => void;
	renderIcon: (item: SpotlightItem) => React.ReactNode;
	searchQuery: string;
	index: number;
}

function SpotlightCommandItem({
	item,
	onSelect,
	renderIcon,
	searchQuery,
	index,
}: SpotlightCommandItemProps) {
	return (
		<CommandItem
			value={item.id}
			onSelect={() => onSelect(item)}
			disabled={item.disabled}
			asChild
		>
			<motion.div
				initial={{ opacity: 0, x: -8 }}
				animate={{ opacity: 1, x: 0 }}
				transition={{ delay: index * 0.02, duration: 0.15 }}
				className={cn(
					"flex items-center gap-3 px-3 py-2.5 mx-1 rounded-lg cursor-pointer",
					"transition-colors duration-100",
					"hover:bg-accent/60 aria-selected:bg-accent aria-selected:shadow-sm",
					item.disabled && "opacity-50 cursor-not-allowed pointer-events-none",
				)}
			>
				<div
					className={cn(
						"flex items-center justify-center h-9 w-9 rounded-lg shrink-0",
						"bg-muted/60 text-foreground",
						"transition-colors duration-100",
						"group-aria-selected:bg-primary/10",
					)}
				>
					{renderIcon(item)}
				</div>
				<div className="flex-1 min-w-0">
					<div className="flex items-center gap-2">
						<span className="font-medium text-sm truncate">
							{highlightMatch(item.label, searchQuery)}
						</span>
						{item.type === "action" && (
							<Badge
								variant="secondary"
								className="text-[9px] px-1.5 py-0 h-4 shrink-0 bg-primary/10 text-primary border-0"
							>
								Action
							</Badge>
						)}
					</div>
					{item.description && (
						<p className="text-xs text-muted-foreground truncate mt-0.5">
							{highlightMatch(item.description, searchQuery)}
						</p>
					)}
				</div>
				{item.shortcut ? (
					<CommandShortcut className="text-[10px] font-mono shrink-0 hidden sm:block">
						{item.shortcut}
					</CommandShortcut>
				) : (
					<ChevronRight className="h-4 w-4 text-muted-foreground/40 shrink-0 opacity-0 group-aria-selected:opacity-100 transition-opacity" />
				)}
			</motion.div>
		</CommandItem>
	);
}
