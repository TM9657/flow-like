"use client";

import {
	ActivityIcon,
	BookOpenIcon,
	CornerRightUpIcon,
	ExternalLinkIcon,
	InfoIcon,
	LockIcon,
	ScaleIcon,
	ShieldIcon,
} from "lucide-react";
import {
	type RefObject,
	forwardRef,
	memo,
	useCallback,
	useImperativeHandle,
	useMemo,
	useState,
} from "react";
import { type IBoard, cn } from "../../../lib";
import type { INode } from "../../../lib/schema/flow/node";
import { type IPin, IPinType } from "../../../lib/schema/flow/pin";
import { Badge } from "../../ui/badge";
import { Button } from "../../ui/button";
import { DynamicImage } from "../../ui/dynamic-image";
import { Separator } from "../../ui/separator";
import { Sheet, SheetContent, SheetHeader } from "../../ui/sheet";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../ui/tooltip";
import { typeToColor } from "../utils";

export interface FlowNodeInfoOverlayHandle {
	openNodeInfo: (node: INode) => void;
	close: () => void;
}

type FlowNodeInfoOverlayProps = {
	className?: string;
	refs:
		| {
				[key: string]: string;
		  }
		| undefined;
	boardRef: RefObject<IBoard | undefined>;
	onFocusNode: (nodeId: string) => void;
};

function buildDocsUrl(node: INode): string {
	if (node.docs) {
		return node.docs;
	}
	const categoryPath = node.category.toLowerCase().split("/").join("/");
	const nodeName = node.name
		.split("_")
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
		.join(" ");
	return `https://docs.flow-like.com/nodes/${categoryPath}/${nodeName}`;
}

const HeroSection = memo(
	({
		node,
		unrefValue,
	}: { node: INode; unrefValue: (key: string) => string }) => {
		const category = node.category
			.split("/")
			.map(
				(segment) =>
					segment.charAt(0).toUpperCase() + segment.slice(1).toLowerCase(),
			)
			.join(" / ");

		return (
			<div className="rounded-2xl bg-linear-to-br from-primary/15 via-primary/5 to-transparent p-5 border border-primary/20">
				<div className="flex items-start gap-4">
					{node.icon ? (
						<div className="flex items-center justify-center w-12 h-12 rounded-2xl bg-background/50 border border-primary/20">
							<DynamicImage className="w-7 h-7 bg-foreground" url={node.icon} />
						</div>
					) : (
						<div className="flex items-center justify-center w-12 h-12 rounded-2xl bg-primary/10 border border-primary/20 text-primary">
							<InfoIcon className="w-6 h-6" />
						</div>
					)}
					<div className="flex-1 min-w-0 space-y-2">
						<div className="flex items-center gap-2 flex-wrap">
							<Badge
								variant="secondary"
								className="text-xs uppercase tracking-wide"
							>
								{category}
							</Badge>
							{node.layer && (
								<Badge variant="outline" className="text-xs">
									Layer: {node.layer}
								</Badge>
							)}
						</div>
						<div>
							<h2 className="text-2xl font-semibold leading-tight">
								{node.friendly_name}
							</h2>
							<p className="text-xs text-muted-foreground font-mono">
								{node.name}
							</p>
						</div>
						<p className="text-sm text-muted-foreground leading-relaxed">
							{unrefValue(node.description)}
						</p>
					</div>
				</div>
			</div>
		);
	},
);
HeroSection.displayName = "HeroSection";

const KeyFactCard = memo(
	({ label, value }: { label: string; value: string | number | boolean }) => {
		return (
			<div className="rounded-xl border bg-card/80 p-3 shadow-sm">
				<p className="text-[11px] uppercase tracking-wide text-muted-foreground">
					{label}
				</p>
				<p className="mt-1 text-base font-medium wrap-break-word">
					{value ?? "—"}
				</p>
			</div>
		);
	},
);
KeyFactCard.displayName = "KeyFactCard";

const OverviewSection = memo(({ node }: { node: INode }) => {
	const facts = useMemo(
		() => [
			{ label: "Category", value: node.category },
			{ label: "Layer", value: node.layer ?? "Global" },
			{ label: "Entry point", value: node.start ? "Starts flows" : "Utility" },
			{
				label: "Long running",
				value: node.long_running ? "Yes" : "No",
			},
			{
				label: "Event callback",
				value: node.event_callback ? "Enabled" : "Disabled",
			},
		],
		[
			node.category,
			node.layer,
			node.start,
			node.long_running,
			node.event_callback,
		],
	);

	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<h3 className="text-sm font-semibold tracking-wide text-muted-foreground uppercase">
					Overview
				</h3>
			</div>
			<div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
				{facts.map((fact) => (
					<KeyFactCard key={fact.label} label={fact.label} value={fact.value} />
				))}
			</div>
		</div>
	);
});
OverviewSection.displayName = "OverviewSection";

const DocsPreview = memo(({ url }: { url: string }) => {
	const [showPreview, setShowPreview] = useState(true);
	const togglePreview = useCallback(() => setShowPreview((prev) => !prev), []);

	return (
		<div className="flex flex-col h-full gap-3">
			<div className="flex items-center justify-between gap-3 flex-wrap shrink-0">
				<div>
					<h3 className="text-sm font-semibold">Documentation</h3>
					<p className="text-xs text-muted-foreground">
						Preview docs inline without leaving the board.
					</p>
				</div>
				<Button variant="outline" size="sm" onClick={togglePreview}>
					{showPreview ? "Hide preview" : "Show preview"}
				</Button>
			</div>
			{showPreview ? (
				<div className="rounded-2xl border bg-card overflow-hidden flex-1 min-h-[300px] md:min-h-0">
					<iframe
						title="Node docs preview"
						src={url}
						className="w-full h-full"
						loading="lazy"
						sandbox="allow-same-origin allow-scripts"
					/>
				</div>
			) : (
				<div className="rounded-2xl border border-dashed p-4 text-xs text-muted-foreground flex items-center gap-2">
					<BookOpenIcon className="w-4 h-4" />
					Inline docs are loaded on demand to keep the board fast.
				</div>
			)}
		</div>
	);
});
DocsPreview.displayName = "DocsPreview";

const PinInfo = memo(
	({ pin, unrefValue }: { pin: IPin; unrefValue: (key: string) => string }) => {
		const color = typeToColor(pin.data_type);

		return (
			<div className="group relative p-3 rounded-lg border bg-card hover:bg-accent/50 hover:border-accent transition-all">
				<div className="flex items-start gap-3">
					<div className="shrink-0 mt-0.5">
						<div
							className="w-3 h-3 rounded-full border-2 border-background shadow-sm"
							style={{ backgroundColor: color }}
						/>
					</div>
					<div className="flex-1 min-w-0 space-y-2">
						<div className="flex items-start justify-between gap-2">
							<h4 className="text-sm font-medium leading-tight">
								{pin.friendly_name}
							</h4>
							<div className="flex items-center gap-1 shrink-0">
								<Badge
									variant="outline"
									className="text-[10px] px-1.5 py-0 h-5"
									style={{ borderColor: color, color }}
								>
									{pin.data_type}
								</Badge>
							</div>
						</div>
						<p className="text-xs text-muted-foreground leading-relaxed">
							{unrefValue(pin.description)}
						</p>
						<div className="flex items-center gap-3 text-[10px] text-muted-foreground">
							<span className="flex items-center gap-1">
								<span className="font-medium">Type:</span>
								<code className="px-1 py-0.5 rounded bg-muted">
									{pin.value_type}
								</code>
							</span>
						</div>
					</div>
				</div>
			</div>
		);
	},
);
PinInfo.displayName = "PinInfo";

const PinsSection = memo(
	({
		inputPins,
		outputPins,
		unrefValue,
	}: {
		inputPins: IPin[];
		outputPins: IPin[];
		unrefValue: (key: string) => string;
	}) => {
		return (
			<div className="space-y-4">
				<h3 className="text-sm font-semibold flex items-center gap-2">
					<div className="w-1 h-4 bg-primary rounded-full" />
					Pins
				</h3>

				{inputPins.length > 0 && (
					<div className="space-y-3">
						<div className="flex items-center gap-2">
							<Badge variant="outline" className="text-xs">
								Input
							</Badge>
							<span className="text-xs text-muted-foreground">
								{inputPins.length} {inputPins.length === 1 ? "pin" : "pins"}
							</span>
						</div>
						<div className="space-y-2">
							{inputPins.map((pin) => (
								<PinInfo key={pin.id} pin={pin} unrefValue={unrefValue} />
							))}
						</div>
					</div>
				)}

				{outputPins.length > 0 && (
					<div className="space-y-3">
						<div className="flex items-center gap-2">
							<Badge variant="outline" className="text-xs">
								Output
							</Badge>
							<span className="text-xs text-muted-foreground">
								{outputPins.length} {outputPins.length === 1 ? "pin" : "pins"}
							</span>
						</div>
						<div className="space-y-2">
							{outputPins.map((pin) => (
								<PinInfo key={pin.id} pin={pin} unrefValue={unrefValue} />
							))}
						</div>
					</div>
				)}
			</div>
		);
	},
);
PinsSection.displayName = "PinsSection";

const ScoreCard = memo(
	({
		label,
		score,
		description,
		icon: Icon,
	}: {
		label: string;
		score: number;
		description: string;
		icon: React.ComponentType<{ className?: string }>;
	}) => {
		const { color, bgColor, textColor } = useMemo(() => {
			if (score <= 3)
				return {
					color: "text-emerald-500",
					bgColor: "bg-emerald-500/10",
					textColor: "text-emerald-700 dark:text-emerald-400",
				};
			if (score <= 6)
				return {
					color: "text-amber-500",
					bgColor: "bg-amber-500/10",
					textColor: "text-amber-700 dark:text-amber-400",
				};
			return {
				color: "text-rose-500",
				bgColor: "bg-rose-500/10",
				textColor: "text-rose-700 dark:text-rose-400",
			};
		}, [score]);

		return (
			<Tooltip>
				<TooltipTrigger asChild>
					<div
						className={cn(
							"relative p-4 rounded-lg border transition-all hover:shadow-md cursor-help",
							bgColor,
						)}
					>
						<div className="flex items-center justify-between mb-2">
							<Icon className={cn("w-4 h-4", color)} />
							<span className={cn("text-2xl font-bold tabular-nums", color)}>
								{score}
							</span>
						</div>
						<div className="space-y-0.5">
							<p className="text-xs font-semibold text-foreground">{label}</p>
							<p className={cn("text-[10px]", textColor)}>{description}</p>
						</div>
					</div>
				</TooltipTrigger>
				<TooltipContent>
					<p className="text-xs">
						{score <= 3 ? "Good" : score <= 6 ? "Moderate" : "Needs Attention"}
					</p>
				</TooltipContent>
			</Tooltip>
		);
	},
);
ScoreCard.displayName = "ScoreCard";

const ScoresSection = memo(
	({ scores }: { scores: NonNullable<INode["scores"]> }) => {
		const scoreItems = useMemo(
			() => [
				{
					label: "Privacy",
					score: scores.privacy,
					description: "Data protection level",
					icon: LockIcon,
				},
				{
					label: "Security",
					score: scores.security,
					description: "Attack resistance",
					icon: ShieldIcon,
				},
				{
					label: "Performance",
					score: scores.performance,
					description: "Computational efficiency",
					icon: ActivityIcon,
				},
				{
					label: "Governance",
					score: scores.governance,
					description: "Policy compliance",
					icon: ScaleIcon,
				},
			],
			[scores.privacy, scores.security, scores.performance, scores.governance],
		);

		return (
			<div className="space-y-4">
				<h3 className="text-sm font-semibold flex items-center gap-2">
					<div className="w-1 h-4 bg-primary rounded-full" />
					Quality Metrics
				</h3>
				<p className="text-xs text-muted-foreground">
					Scores range from 0-10, where higher values indicate areas requiring
					attention.
				</p>
				<div className="grid grid-cols-2 gap-3">
					{scoreItems.map((item) => (
						<ScoreCard key={item.label} {...item} />
					))}
				</div>
			</div>
		);
	},
);
ScoresSection.displayName = "ScoresSection";

const FnRefsSection = memo(
	({
		fnRefs,
		boardRef,
		onFocusNode,
	}: {
		fnRefs: NonNullable<INode["fn_refs"]>;
		boardRef: RefObject<IBoard | undefined>;
		onFocusNode: (nodeId: string) => void;
	}) => {
		return (
			<div className="space-y-4">
				<h3 className="text-sm font-semibold flex items-center gap-2">
					<div className="w-1 h-4 bg-primary rounded-full" />
					Function References
				</h3>
				<div className="space-y-3">
					{fnRefs.can_reference_fns && (
						<div className="flex items-start gap-3 p-3 rounded-lg bg-muted/50 border">
							<div className="flex items-center justify-center w-6 h-6 rounded-full bg-primary/10 text-primary shrink-0 mt-0.5">
								<span className="text-xs font-semibold">→</span>
							</div>
							<div className="flex-1 min-w-0">
								<p className="text-sm font-medium">Can reference functions</p>
								<p className="text-xs text-muted-foreground mt-0.5">
									This node can call other functions in the flow
								</p>
							</div>
						</div>
					)}
					{fnRefs.can_be_referenced_by_fns && (
						<div className="flex items-start gap-3 p-3 rounded-lg bg-muted/50 border">
							<div className="flex items-center justify-center w-6 h-6 rounded-full bg-primary/10 text-primary shrink-0 mt-0.5">
								<span className="text-xs font-semibold">←</span>
							</div>
							<div className="flex-1 min-w-0">
								<p className="text-sm font-medium">Can be referenced</p>
								<p className="text-xs text-muted-foreground mt-0.5">
									Other functions can call this node
								</p>
							</div>
						</div>
					)}
					{fnRefs.fn_refs.length > 0 && (
						<div className="space-y-2">
							<p className="text-xs font-medium text-muted-foreground">
								Active References
							</p>
							<div className="space-y-2">
								{fnRefs.fn_refs.map((ref) => (
									<div
										key={ref}
										className="flex items-center gap-3 rounded-xl border bg-card/70 px-3 py-2"
									>
										<code className="text-xs font-mono wrap-break-word flex-1">
											{boardRef.current?.nodes[ref]?.friendly_name || ref}
										</code>
										<Button
											variant="outline"
											size="icon"
											className="h-6 w-6 p-1"
											onClick={() => onFocusNode(ref)}
											title="Go to node"
										>
											<CornerRightUpIcon className="w-3.5 h-3.5" />
										</Button>
									</div>
								))}
							</div>
						</div>
					)}
				</div>
			</div>
		);
	},
);
FnRefsSection.displayName = "FnRefsSection";

export const FlowNodeInfoOverlay = forwardRef<
	FlowNodeInfoOverlayHandle,
	FlowNodeInfoOverlayProps
>(({ className, refs, boardRef, onFocusNode }, ref) => {
	const [isOpen, setIsOpen] = useState(false);
	const [selectedNode, setSelectedNode] = useState<INode | null>(null);

	useImperativeHandle(ref, () => ({
		openNodeInfo: (node: INode) => {
			setSelectedNode(node);
			setIsOpen(true);
		},
		close: () => {
			setIsOpen(false);
		},
	}));

	const unrefValue = useCallback(
		(key: string): string => {
			return refs?.[key] ?? key;
		},
		[refs],
	);

	const inputPins = useMemo(
		() =>
			selectedNode
				? Object.values(selectedNode.pins)
						.filter((pin) => pin.pin_type === IPinType.Input)
						.sort((a, b) => a.index - b.index)
				: [],
		[selectedNode],
	);

	const outputPins = useMemo(
		() =>
			selectedNode
				? Object.values(selectedNode.pins)
						.filter((pin) => pin.pin_type === IPinType.Output)
						.sort((a, b) => a.index - b.index)
				: [],
		[selectedNode],
	);

	const docsUrl = useMemo(
		() => (selectedNode ? buildDocsUrl(selectedNode) : ""),
		[selectedNode],
	);

	const hasScores =
		selectedNode?.scores &&
		(selectedNode.scores.privacy > 0 ||
			selectedNode.scores.security > 0 ||
			selectedNode.scores.performance > 0 ||
			selectedNode.scores.governance > 0);

	const hasFnRefs =
		selectedNode?.fn_refs &&
		(selectedNode.fn_refs.can_reference_fns ||
			selectedNode.fn_refs.can_be_referenced_by_fns);

	if (!selectedNode) return null;

	return (
		<Sheet open={isOpen} onOpenChange={setIsOpen}>
			<SheetContent
				side="bottom"
				className={cn(
					"max-h-[90vh] overflow-hidden flex flex-col md:inset-y-0 md:right-0 md:left-auto md:h-full md:max-h-full md:w-[90vw] lg:w-[80vw] xl:w-[75vw] md:border-l md.data-[state=closed]:slide-out-to-right md.data-[state=open]:slide-in-from-right",
					className,
				)}
			>
				<SheetHeader className="shrink-0 border-b pb-4">
					<HeroSection node={selectedNode} unrefValue={unrefValue} />
				</SheetHeader>

				<div className="flex-1 overflow-hidden flex flex-col md:flex-row gap-4">
					{/* Left side - Node info */}
					<div className="flex-1 overflow-y-auto space-y-8 py-6 px-3 min-w-0">
						<OverviewSection node={selectedNode} />

						<Separator />
						<PinsSection
							inputPins={inputPins}
							outputPins={outputPins}
							unrefValue={unrefValue}
						/>

						{hasScores && selectedNode.scores && (
							<>
								<Separator />
								<ScoresSection scores={selectedNode.scores} />
							</>
						)}

						{hasFnRefs && selectedNode.fn_refs && (
							<>
								<Separator />
								<FnRefsSection
									fnRefs={selectedNode.fn_refs}
									boardRef={boardRef}
									onFocusNode={onFocusNode}
								/>
							</>
						)}
					</div>

					{/* Right side - Documentation (desktop only, horizontal) */}
					{docsUrl && (
						<>
							<Separator className="md:hidden" />
							<div className="md:border-l md:flex-1 md:min-w-0 md:overflow-hidden md:flex md:flex-col py-6 px-3">
								<DocsPreview url={docsUrl} />
							</div>
						</>
					)}
				</div>

				<div className="shrink-0 border-t pt-4 pb-2">
					<Button variant="outline" size="sm" className="w-full" asChild>
						<a
							href={docsUrl}
							target="_blank"
							rel="noopener noreferrer"
							className="inline-flex items-center justify-center gap-2"
						>
							<ExternalLinkIcon className="w-4 h-4" />
							Open full documentation
						</a>
					</Button>
				</div>
			</SheetContent>
		</Sheet>
	);
});

FlowNodeInfoOverlay.displayName = "FlowNodeInfoOverlay";
