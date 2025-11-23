"use client";

import { BrainCircuit, ChevronDown, ChevronUp } from "lucide-react";
import { Suspense, lazy, useMemo, useState } from "react";
import { Button } from "../../ui/button";

const TextEditor = lazy(() =>
	import("../../ui/text-editor").then((m) => ({ default: m.TextEditor })),
);

interface ReasoningViewerProps {
	reasoning: string;
	defaultExpanded?: boolean;
	compact?: boolean;
}

export function ReasoningViewer({
	reasoning,
	defaultExpanded = false,
	compact = false,
}: ReasoningViewerProps) {
	const [isExpanded, setIsExpanded] = useState(defaultExpanded);
	const [shouldRender, setShouldRender] = useState(defaultExpanded);

	// Memoize the reasoning content to prevent re-renders
	const memoizedReasoning = useMemo(() => reasoning, [reasoning]);

	// Lazy render on first expansion
	const handleExpand = () => {
		if (!isExpanded && !shouldRender) {
			setShouldRender(true);
		}
		setIsExpanded(!isExpanded);
	};

	if (!reasoning || reasoning.trim() === "") {
		return null;
	}

	return (
		<div
			className={
				compact
					? "rounded-lg overflow-hidden bg-muted/20"
					: "mt-2 border rounded-lg overflow-hidden bg-muted/30"
			}
		>
			<button
				onClick={handleExpand}
				className={
					compact
						? "w-full flex items-center justify-between p-2 hover:bg-muted/30 transition-colors"
						: "w-full flex items-center justify-between p-3 hover:bg-muted/50 transition-colors"
				}
			>
				<div className="flex items-center gap-2">
					<BrainCircuit
						className={
							compact ? "w-3 h-3 text-primary" : "w-4 h-4 text-primary"
						}
					/>
					<span
						className={compact ? "text-xs font-medium" : "text-sm font-medium"}
					>
						Reasoning
					</span>
				</div>
				<Button
					variant="ghost"
					size="sm"
					className="h-6 w-6 p-0"
					onClick={(e) => {
						e.stopPropagation();
						handleExpand();
					}}
				>
					{isExpanded ? (
						<ChevronUp className="w-4 h-4" />
					) : (
						<ChevronDown className="w-4 h-4" />
					)}
				</Button>
			</button>

			{isExpanded && shouldRender && (
				<div className={compact ? "" : "border-t"}>
					<div
						className={
							compact
								? "max-h-[200px] overflow-y-auto scroll-smooth"
								: "max-h-[300px] overflow-y-auto scroll-smooth"
						}
						style={{
							containIntrinsicSize: compact ? "200px" : "300px",
							contentVisibility: "auto",
						}}
					>
						<div className={compact ? "p-2 text-xs" : "p-3 text-sm"}>
							<Suspense
								fallback={
									<div className="flex items-center justify-center py-4 text-muted-foreground">
										<div className="animate-pulse">Loading...</div>
									</div>
								}
							>
								<TextEditor
									initialContent={memoizedReasoning}
									isMarkdown={true}
									editable={false}
									minimal={true}
								/>
							</Suspense>
						</div>
					</div>
				</div>
			)}
		</div>
	);
}
