"use client";

import { AnimatePresence, motion } from "framer-motion";
import { ChevronDown, LayoutGridIcon, PlayIcon, XIcon } from "lucide-react";
import { memo, useMemo, useState } from "react";

import { Button } from "../ui/button";

import type { SurfaceComponent } from "../a2ui/types";
import { getComponentCounts } from "./utils";

interface PendingComponentsViewProps {
	components: SurfaceComponent[];
	onApply: () => void;
	onDismiss: () => void;
}

export const PendingComponentsView = memo(function PendingComponentsView({
	components,
	onApply,
	onDismiss,
}: PendingComponentsViewProps) {
	const [isOpen, setIsOpen] = useState(true);

	const componentCounts = useMemo(
		() => getComponentCounts(components),
		[components],
	);

	if (components.length === 0) return null;

	return (
		<motion.div
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			className="border-t border-border/30 bg-primary/5"
		>
			<div className="px-3 py-2.5">
				<div className="flex items-center justify-between gap-2 mb-2">
					<div className="flex items-center gap-2">
						<div className="p-1 bg-primary/20 rounded-md">
							<LayoutGridIcon className="h-3 w-3 text-primary" />
						</div>
						<div>
							<div className="text-[10px] font-semibold">Ready to Apply</div>
							<div className="text-[9px] text-muted-foreground">
								{components.length} component
								{components.length !== 1 ? "s" : ""}
							</div>
						</div>
					</div>
					<div className="flex items-center gap-1">
						<Button
							size="sm"
							variant="ghost"
							className="h-6 w-6 p-0"
							onClick={() => setIsOpen(!isOpen)}
						>
							<ChevronDown
								className={`h-3 w-3 transition-transform ${isOpen ? "rotate-180" : ""}`}
							/>
						</Button>
						<Button
							size="sm"
							className="h-6 px-2 text-[10px] gap-1"
							onClick={onApply}
						>
							<PlayIcon className="h-3 w-3" />
							Apply
						</Button>
						<Button
							size="sm"
							variant="ghost"
							className="h-6 w-6 p-0 text-muted-foreground hover:text-destructive"
							onClick={onDismiss}
						>
							<XIcon className="h-3 w-3" />
						</Button>
					</div>
				</div>

				{/* Component badges */}
				<div className="flex flex-wrap gap-1">
					{Object.entries(componentCounts).map(([type, count]) => (
						<span
							key={type}
							className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded-full text-[9px] font-medium bg-primary/10 text-primary border border-primary/20"
						>
							{count} {type}
						</span>
					))}
				</div>

				{/* Expanded list */}
				<AnimatePresence>
					{isOpen && (
						<motion.div
							initial={{ height: 0, opacity: 0 }}
							animate={{ height: "auto", opacity: 1 }}
							exit={{ height: 0, opacity: 0 }}
							transition={{ duration: 0.2 }}
							className="overflow-hidden"
						>
							<div className="pt-2 space-y-1 max-h-24 overflow-y-auto">
								{components.map((comp, i) => (
									<div
										key={comp.id || i}
										className="flex items-center gap-2 p-1.5 rounded-md bg-background/50 text-[9px]"
									>
										<span className="font-medium">
											{comp.component?.type ?? "Unknown"}
										</span>
										<span className="text-muted-foreground truncate">
											{comp.id}
										</span>
									</div>
								))}
							</div>
						</motion.div>
					)}
				</AnimatePresence>
			</div>
		</motion.div>
	);
});
