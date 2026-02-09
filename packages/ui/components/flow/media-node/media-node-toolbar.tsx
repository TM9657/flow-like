"use client";
import {
	LockIcon,
	SquareChevronDownIcon,
	SquareChevronUpIcon,
	UnlockIcon,
} from "lucide-react";
import { memo } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../ui/tooltip";

interface MediaNodeToolbarProps {
	isLocked: boolean;
	onMoveUp: () => void;
	onMoveDown: () => void;
	onToggleLock: () => void;
}

const ToolbarButton = memo(
	({
		onClick,
		icon: Icon,
		tooltip,
	}: {
		onClick: () => void;
		icon: React.ComponentType<{ className?: string }>;
		tooltip: string;
	}) => (
		<Tooltip>
			<TooltipTrigger asChild>
				<button
					type="button"
					className="h-5 w-5 flex items-center justify-center rounded transition-colors hover:bg-white/10"
					onClick={(e) => {
						e.stopPropagation();
						onClick();
					}}
				>
					<Icon className="h-3 w-3" />
				</button>
			</TooltipTrigger>
			<TooltipContent side="top" className="text-[10px] px-1.5 py-0.5">
				{tooltip}
			</TooltipContent>
		</Tooltip>
	),
);

ToolbarButton.displayName = "ToolbarButton";

const Divider = memo(() => <div className="w-px h-3 bg-white/20 mx-0.5" />);

Divider.displayName = "Divider";

export const MediaNodeToolbar = memo(
	({ isLocked, onMoveUp, onMoveDown, onToggleLock }: MediaNodeToolbarProps) => (
		<div className="absolute -top-6 left-1/2 -translate-x-1/2 z-10 flex items-center gap-0.5 bg-black/80 rounded px-1 py-0.5 text-white">
			<ToolbarButton
				icon={SquareChevronUpIcon}
				onClick={onMoveUp}
				tooltip="Move Up"
			/>
			<ToolbarButton
				icon={SquareChevronDownIcon}
				onClick={onMoveDown}
				tooltip="Move Down"
			/>
			<Divider />
			<ToolbarButton
				icon={isLocked ? LockIcon : UnlockIcon}
				onClick={onToggleLock}
				tooltip={isLocked ? "Unlock" : "Lock"}
			/>
		</div>
	),
);

MediaNodeToolbar.displayName = "MediaNodeToolbar";
