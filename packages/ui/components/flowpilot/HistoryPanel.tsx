"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	ClockIcon,
	LayoutGridIcon,
	MessageSquareIcon,
	PlusIcon,
	TrashIcon,
	WorkflowIcon,
} from "lucide-react";
import { memo, useCallback, useEffect, useState } from "react";

import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../ui/tooltip";

import {
	type IFlowPilotConversation,
	deleteConversation,
	getRecentConversations,
} from "../../lib/flowpilot-db";
import type { AgentMode } from "./types";

interface HistoryPanelProps {
	mode: AgentMode;
	currentConversationId?: string;
	onSelectConversation: (conversation: IFlowPilotConversation) => void;
	onNewConversation: () => void;
	isOpen: boolean;
	onClose: () => void;
}

const ModeIcon = memo(function ModeIcon({
	mode,
}: { mode: "board" | "ui" | "both" }) {
	switch (mode) {
		case "board":
			return <WorkflowIcon className="h-3 w-3" />;
		case "ui":
			return <LayoutGridIcon className="h-3 w-3" />;
		default:
			return <MessageSquareIcon className="h-3 w-3" />;
	}
});

const formatRelativeTime = (dateString: string): string => {
	const date = new Date(dateString);
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
	const diffMins = Math.floor(diffMs / 60000);
	const diffHours = Math.floor(diffMs / 3600000);
	const diffDays = Math.floor(diffMs / 86400000);

	if (diffMins < 1) return "Just now";
	if (diffMins < 60) return `${diffMins}m ago`;
	if (diffHours < 24) return `${diffHours}h ago`;
	if (diffDays < 7) return `${diffDays}d ago`;
	return date.toLocaleDateString();
};

export const HistoryPanel = memo(function HistoryPanel({
	mode,
	currentConversationId,
	onSelectConversation,
	onNewConversation,
	isOpen,
	onClose,
}: HistoryPanelProps) {
	const [conversations, setConversations] = useState<IFlowPilotConversation[]>(
		[],
	);
	const [loading, setLoading] = useState(true);

	const loadConversations = useCallback(async () => {
		setLoading(true);
		try {
			const recent = await getRecentConversations(50, mode);
			setConversations(recent);
		} catch (error) {
			console.error("Failed to load conversations:", error);
		} finally {
			setLoading(false);
		}
	}, [mode]);

	useEffect(() => {
		if (isOpen) {
			loadConversations();
		}
	}, [isOpen, loadConversations]);

	const handleDelete = useCallback(
		async (e: React.MouseEvent, id: string) => {
			e.stopPropagation();
			try {
				await deleteConversation(id);
				setConversations((prev) => prev.filter((c) => c.id !== id));
			} catch (error) {
				console.error("Failed to delete conversation:", error);
			}
		},
		[],
	);

	if (!isOpen) return null;

	return (
		<AnimatePresence>
			<motion.div
				initial={{ opacity: 0, x: -10 }}
				animate={{ opacity: 1, x: 0 }}
				exit={{ opacity: 0, x: -10 }}
				className="absolute inset-0 z-10 bg-background/95 backdrop-blur-sm flex flex-col"
			>
				{/* Header */}
				<div className="flex items-center justify-between px-3 py-2 border-b shrink-0">
					<div className="flex items-center gap-2">
						<ClockIcon className="h-4 w-4 text-muted-foreground" />
						<span className="text-sm font-medium">History</span>
					</div>
					<div className="flex items-center gap-1">
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									size="sm"
									variant="ghost"
									className="h-7 w-7 p-0"
									onClick={() => {
										onNewConversation();
										onClose();
									}}
								>
									<PlusIcon className="h-4 w-4" />
								</Button>
							</TooltipTrigger>
							<TooltipContent>New conversation</TooltipContent>
						</Tooltip>
						<Button
							size="sm"
							variant="ghost"
							className="h-7 px-2 text-xs"
							onClick={onClose}
						>
							Close
						</Button>
					</div>
				</div>

				{/* Conversations list */}
				<ScrollArea className="flex-1">
					<div className="p-2 space-y-1">
						{loading ? (
							<div className="flex items-center justify-center py-8">
								<div className="animate-pulse text-xs text-muted-foreground">
									Loading...
								</div>
							</div>
						) : conversations.length === 0 ? (
							<div className="flex flex-col items-center justify-center py-8 text-center">
								<MessageSquareIcon className="h-8 w-8 text-muted-foreground/50 mb-2" />
								<p className="text-xs text-muted-foreground">
									No conversations yet
								</p>
								<p className="text-[10px] text-muted-foreground/70 mt-1">
									Start a new chat to see it here
								</p>
							</div>
						) : (
							conversations.map((conversation) => (
								<motion.button
									key={conversation.id}
									initial={{ opacity: 0, y: 5 }}
									animate={{ opacity: 1, y: 0 }}
									onClick={() => {
										onSelectConversation(conversation);
										onClose();
									}}
									className={`w-full text-left p-2 rounded-lg transition-colors group ${
										conversation.id === currentConversationId
											? "bg-primary/10 border border-primary/20"
											: "hover:bg-muted/50 border border-transparent"
									}`}
								>
									<div className="flex items-start gap-2">
										<div
											className={`p-1 rounded shrink-0 ${
												conversation.id === currentConversationId
													? "bg-primary/20 text-primary"
													: "bg-muted text-muted-foreground"
											}`}
										>
											<ModeIcon mode={conversation.mode} />
										</div>
										<div className="flex-1 min-w-0">
											<div className="flex items-center justify-between gap-2">
												<span className="text-xs font-medium truncate">
													{conversation.title}
												</span>
												<Button
													size="sm"
													variant="ghost"
													className="h-5 w-5 p-0 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
													onClick={(e) => handleDelete(e, conversation.id)}
												>
													<TrashIcon className="h-3 w-3 text-muted-foreground hover:text-destructive" />
												</Button>
											</div>
											<div className="flex items-center gap-2 mt-0.5">
												<span className="text-[10px] text-muted-foreground">
													{conversation.messageCount} message
													{conversation.messageCount !== 1 ? "s" : ""}
												</span>
												<span className="text-[10px] text-muted-foreground/70">
													•
												</span>
												<span className="text-[10px] text-muted-foreground/70">
													{formatRelativeTime(conversation.updatedAt)}
												</span>
											</div>
										</div>
									</div>
								</motion.button>
							))
						)}
					</div>
				</ScrollArea>
			</motion.div>
		</AnimatePresence>
	);
});
