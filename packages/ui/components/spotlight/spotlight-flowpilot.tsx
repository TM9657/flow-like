"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	ArrowLeft,
	Bot,
	Loader2,
	SendHorizonal,
	Sparkles,
	User,
} from "lucide-react";
import type * as React from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "../../lib/utils";
import { useSpotlightStore } from "../../state/spotlight-state";
import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";

interface Message {
	id: string;
	role: "user" | "assistant";
	content: string;
	timestamp: Date;
}

interface SpotlightFlowPilotProps {
	onSendMessage?: (message: string) => Promise<string>;
	placeholder?: string;
	embedded?: boolean;
	onClose?: () => void;
}

export function SpotlightFlowPilot({
	onSendMessage,
	placeholder = "Ask FlowPilot anything...",
	embedded = false,
	onClose,
}: SpotlightFlowPilotProps) {
	const { setMode, close } = useSpotlightStore();
	const [messages, setMessages] = useState<Message[]>([]);
	const [input, setInput] = useState("");
	const [isLoading, setIsLoading] = useState(false);
	const inputRef = useRef<HTMLInputElement>(null);
	const scrollRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		inputRef.current?.focus();
	}, []);

	useEffect(() => {
		if (scrollRef.current) {
			scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
		}
	}, [messages]);

	const handleSend = useCallback(async () => {
		if (!input.trim() || isLoading) return;

		const userMessage: Message = {
			id: crypto.randomUUID(),
			role: "user",
			content: input.trim(),
			timestamp: new Date(),
		};

		setMessages((prev) => [...prev, userMessage]);
		setInput("");
		setIsLoading(true);

		try {
			const response = onSendMessage
				? await onSendMessage(userMessage.content)
				: "FlowPilot is not configured. Please set up the chat handler.";

			const assistantMessage: Message = {
				id: crypto.randomUUID(),
				role: "assistant",
				content: response,
				timestamp: new Date(),
			};

			setMessages((prev) => [...prev, assistantMessage]);
		} catch (error) {
			const errorMessage: Message = {
				id: crypto.randomUUID(),
				role: "assistant",
				content: "Sorry, I encountered an error. Please try again.",
				timestamp: new Date(),
			};
			setMessages((prev) => [...prev, errorMessage]);
		} finally {
			setIsLoading(false);
		}
	}, [input, isLoading, onSendMessage]);

	const handleBack = () => {
		if (embedded && onClose) {
			onClose();
		} else {
			setMode("search");
		}
	};

	const handleKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			handleSend();
		}
		if (e.key === "Escape") {
			handleBack();
		}
	};

	return (
		<div className={cn("flex flex-col h-full", embedded && "min-h-[350px]")}>
			<div className="flex items-center gap-2 px-3 py-2 border-b border-border/40 shrink-0">
				<Button
					variant="ghost"
					size="icon"
					className="h-7 w-7"
					onClick={handleBack}
				>
					<ArrowLeft className="h-3.5 w-3.5" />
				</Button>
				<div className="flex items-center gap-2">
					<div
						className={cn(
							"rounded-lg bg-linear-to-br from-violet-500/20 to-purple-500/20 flex items-center justify-center",
							embedded ? "h-6 w-6" : "h-8 w-8",
						)}
					>
						<Bot
							className={cn(
								"text-violet-500",
								embedded ? "h-3.5 w-3.5" : "h-4 w-4",
							)}
						/>
					</div>
					<div>
						<h3
							className={cn("font-semibold", embedded ? "text-xs" : "text-sm")}
						>
							FlowPilot
						</h3>
						{!embedded && (
							<p className="text-[10px] text-muted-foreground">AI Assistant</p>
						)}
					</div>
				</div>
			</div>

			<ScrollArea
				className={cn("flex-1 min-h-0", embedded ? "px-3" : "px-4")}
				ref={scrollRef}
			>
				<div className={cn("space-y-4", embedded ? "py-3" : "py-4")}>
					{messages.length === 0 && (
						<motion.div
							initial={{ opacity: 0, y: 10 }}
							animate={{ opacity: 1, y: 0 }}
							className={cn(
								"flex flex-col items-center justify-center text-center",
								embedded ? "py-4" : "py-8",
							)}
						>
							<div
								className={cn(
									"rounded-2xl bg-linear-to-br from-violet-500/10 to-purple-500/10 flex items-center justify-center mb-3",
									embedded ? "h-10 w-10" : "h-16 w-16 mb-4",
								)}
							>
								<Sparkles
									className={cn(
										"text-violet-500/60",
										embedded ? "h-5 w-5" : "h-8 w-8",
									)}
								/>
							</div>
							<h4
								className={cn(
									"font-medium mb-1",
									embedded ? "text-xs" : "text-sm",
								)}
							>
								{embedded ? "Ask anything" : "How can I help?"}
							</h4>
							<p
								className={cn(
									"text-muted-foreground",
									embedded
										? "text-[10px] max-w-[180px]"
										: "text-xs max-w-[200px]",
								)}
							>
								{embedded
									? "I'm here to help with Flow-Like"
									: "Ask me about Flow-Like, workflows, or anything else!"}
							</p>
							<div
								className={cn(
									"flex flex-wrap gap-1.5 justify-center",
									embedded ? "mt-3" : "mt-4 gap-2",
								)}
							>
								{(embedded
									? ["Create flow", "What are nodes?", "Help"]
									: [
											"How do I create a flow?",
											"What are nodes?",
											"Help with storage",
										]
								).map((suggestion) => (
									<button
										key={suggestion}
										type="button"
										onClick={() => setInput(suggestion)}
										className={cn(
											"rounded-full bg-muted/50 hover:bg-muted text-muted-foreground hover:text-foreground transition-colors",
											embedded
												? "text-[10px] px-2 py-1"
												: "text-xs px-3 py-1.5",
										)}
									>
										{suggestion}
									</button>
								))}
							</div>
						</motion.div>
					)}

					<AnimatePresence mode="popLayout">
						{messages.map((message) => (
							<motion.div
								key={message.id}
								initial={{ opacity: 0, y: 10 }}
								animate={{ opacity: 1, y: 0 }}
								exit={{ opacity: 0, y: -10 }}
								className={cn(
									"flex gap-3",
									message.role === "user" ? "justify-end" : "justify-start",
								)}
							>
								{message.role === "assistant" && (
									<div className="h-7 w-7 rounded-lg bg-linear-to-br from-violet-500/20 to-purple-500/20 flex items-center justify-center shrink-0">
										<Bot className="h-3.5 w-3.5 text-violet-500" />
									</div>
								)}
								<div
									className={cn(
										"max-w-[80%] rounded-xl px-3 py-2 text-sm",
										message.role === "user"
											? "bg-primary text-primary-foreground"
											: "bg-muted/60",
									)}
								>
									{message.content}
								</div>
								{message.role === "user" && (
									<div className="h-7 w-7 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
										<User className="h-3.5 w-3.5 text-primary" />
									</div>
								)}
							</motion.div>
						))}
					</AnimatePresence>

					{isLoading && (
						<motion.div
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
							className="flex gap-3"
						>
							<div className="h-7 w-7 rounded-lg bg-linear-to-br from-violet-500/20 to-purple-500/20 flex items-center justify-center shrink-0">
								<Bot className="h-3.5 w-3.5 text-violet-500" />
							</div>
							<div className="bg-muted/60 rounded-xl px-3 py-2 flex items-center gap-2">
								<Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
								<span className="text-sm text-muted-foreground">
									Thinking...
								</span>
							</div>
						</motion.div>
					)}
				</div>
			</ScrollArea>

			<div
				className={cn(
					"border-t border-border/40 shrink-0",
					embedded ? "p-2" : "p-4",
				)}
			>
				<div className="flex items-center gap-2">
					<input
						ref={inputRef}
						type="text"
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyDown={handleKeyDown}
						placeholder={embedded ? "Ask FlowPilot..." : placeholder}
						disabled={isLoading}
						className={cn(
							"flex-1 rounded-lg bg-muted/50 outline-none placeholder:text-muted-foreground/60 focus:ring-2 focus:ring-primary/20 transition-all disabled:opacity-50",
							embedded ? "h-8 px-3 text-xs" : "h-10 px-4 text-sm",
						)}
					/>
					<Button
						size="icon"
						className={cn("rounded-lg", embedded ? "h-8 w-8" : "h-10 w-10")}
						onClick={handleSend}
						disabled={!input.trim() || isLoading}
					>
						<SendHorizonal
							className={cn(embedded ? "h-3.5 w-3.5" : "h-4 w-4")}
						/>
					</Button>
				</div>
				{!embedded && (
					<p className="text-[10px] text-muted-foreground/60 mt-2 text-center">
						Press Enter to send • Esc to go back
					</p>
				)}
			</div>
		</div>
	);
}
