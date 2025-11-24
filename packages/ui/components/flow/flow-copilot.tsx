"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	BrainCircuitIcon,
	ChevronDown,
	MessageSquareIcon,
	SendIcon,
	SparklesIcon,
	SquarePenIcon,
	Wand2Icon,
	XIcon,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button, Input, ScrollArea, TextEditor } from "../..";
import { useInvoke } from "../../hooks";
import { IBitTypes, type IBoard } from "../../lib";
import { useBackend } from "../../state/backend-state";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../ui/collapsible";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";

interface Suggestion {
	node_type: string;
	reason: string;
	connection_description: string;
}

interface FlowCopilotProps {
	board: IBoard | undefined;
	selectedNodeIds: string[];
	onAcceptSuggestion: (suggestion: Suggestion) => void;
}

type Mode = "chat" | "autocomplete";

function MessageContent({ content }: { content: string }) {
	const thinkingMatch = content.match(/<think>([\s\S]*?)<\/think>/);

	if (thinkingMatch) {
		const thinkingContent = thinkingMatch[1];
		const restContent = content.replace(/<think>[\s\S]*?<\/think>/, "").trim();

		return (
			<div className="space-y-2 w-full">
				<Collapsible className="w-full border rounded-lg bg-background/50 overflow-hidden">
					<CollapsibleTrigger className="flex items-center gap-2 p-2 w-full hover:bg-muted/50 transition-colors text-xs font-medium text-muted-foreground group">
						<BrainCircuitIcon className="w-3 h-3" />
						<span>Reasoning Process</span>
						<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180" />
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="p-3 pt-0 text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/30">
							{thinkingContent.trim()}
						</div>
					</CollapsibleContent>
				</Collapsible>
				<div className="text-sm leading-relaxed whitespace-break-spaces text-wrap max-w-full w-full">
					<TextEditor
						initialContent={restContent}
						isMarkdown={true}
						editable={false}
					/>
				</div>
			</div>
		);
	}

	const openThinkingMatch = content.match(/<think>([\s\S]*?)$/);
	if (openThinkingMatch) {
		const thinkingContent = openThinkingMatch[1];
		const beforeContent = content.substring(0, openThinkingMatch.index).trim();

		return (
			<div className="space-y-2 w-full">
				{beforeContent && (
					<div className="text-sm leading-relaxed whitespace-break-spaces text-wrap max-w-full w-full">
						<TextEditor
							initialContent={beforeContent}
							isMarkdown={true}
							editable={false}
						/>
					</div>
				)}
				<Collapsible
					className="w-full border rounded-lg bg-background/50 overflow-hidden"
					defaultOpen={true}
				>
					<CollapsibleTrigger className="flex items-center gap-2 p-2 w-full hover:bg-muted/50 transition-colors text-xs font-medium text-muted-foreground group">
						<BrainCircuitIcon className="w-3 h-3 animate-pulse" />
						<span>Reasoning Process...</span>
						<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180" />
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="p-3 pt-0 text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/30">
							{thinkingContent.trim()}
							<span className="inline-block w-1.5 h-3 ml-1 bg-primary/50 animate-pulse" />
						</div>
					</CollapsibleContent>
				</Collapsible>
			</div>
		);
	}

	return (
		<div className="text-sm leading-relaxed whitespace-break-spaces text-wrap max-w-full w-full">
			<TextEditor initialContent={content} isMarkdown={true} editable={false} />
		</div>
	);
}

export function FlowCopilot({
	board,
	selectedNodeIds,
	onAcceptSuggestion,
}: FlowCopilotProps) {
	const [isOpen, setIsOpen] = useState(false);
	const [mode, setMode] = useState<Mode>("chat");
	const [input, setInput] = useState("");
	const [messages, setMessages] = useState<
		{ role: "user" | "assistant"; content: string }[]
	>([]);
	const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
	const [loading, setLoading] = useState(false);
	const [selectedModelId, setSelectedModelId] = useState<string | undefined>(
		undefined,
	);
	const backend = useBackend();
	const messagesEndRef = useRef<HTMLDivElement>(null);

	const profile = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
		isOpen,
	);

	const foundBits = useInvoke(
		backend.bitState.searchBits,
		backend.bitState,
		[
			{
				bit_types: [IBitTypes.Llm, IBitTypes.Vlm],
			},
		],
		isOpen && !!profile.data,
		[profile.data?.hub_profile.id],
	);

	const models = foundBits.data || [];

	useEffect(() => {
		if (!selectedModelId && models.length > 0) {
			// Default to Hosted models, then fall back to GPT-4o, then first available
			const hostedModel = models.find(
				(m) => m.parameters?.provider?.provider_name === "Hosted",
			);
			const gpt4o = models.find((m) => m.id.includes("gpt-4o"));
			const defaultModel = hostedModel || gpt4o || models[0];
			setSelectedModelId(defaultModel.id);
		}
	}, [models, selectedModelId]);

	const scrollToBottom = () => {
		// Use requestAnimationFrame to ensure DOM update is complete
		requestAnimationFrame(() => {
			messagesEndRef.current?.scrollIntoView({
				behavior: "smooth",
				block: "nearest",
			});
		});
	};

	useEffect(() => {
		scrollToBottom();
		// Double check scroll after a short delay to handle any layout shifts (like Collapsible animations)
		const timeout = setTimeout(scrollToBottom, 150);
		return () => clearTimeout(timeout);
	}, [messages, suggestions, loading]);

	const handleNewChat = () => {
		setMessages([]);
		setSuggestions([]);
		setInput("");
	};

	const handleSubmit = async () => {
		if (!input.trim()) return;

		const userMsg = input;
		setMessages((prev) => [...prev, { role: "user", content: userMsg }]);
		setInput("");
		setLoading(true);
		setSuggestions([]);

		try {
			if (!board) return;

			let currentMessageContent = "";
			// Add an empty assistant message to stream into
			setMessages((prev) => [...prev, { role: "assistant", content: "" }]);

			const onToken = (token: string) => {
				currentMessageContent += token;
				setMessages((prev) => {
					const newMessages = [...prev];
					const lastMessage = newMessages[newMessages.length - 1];
					if (lastMessage && lastMessage.role === "assistant") {
						lastMessage.content = currentMessageContent;
					}
					return newMessages;
				});
			};

			if (mode === "autocomplete") {
				const result = await backend.boardState.autocomplete(
					board,
					selectedNodeIds,
					userMsg,
					onToken,
					selectedModelId,
				);
				setSuggestions(result);
				// Optional: Replace the JSON/raw text with a friendly message after parsing
				setMessages((prev) => {
					const newMessages = [...prev];
					const lastMessage = newMessages[newMessages.length - 1];
					if (lastMessage && lastMessage.role === "assistant") {
						lastMessage.content =
							"Here are some suggestions based on your request.";
					}
					return newMessages;
				});
			} else {
				// Chat mode
				const result = await backend.boardState.autocomplete(
					board,
					selectedNodeIds,
					userMsg,
					onToken,
					selectedModelId,
				);
				setSuggestions(result);
				// Only overwrite if we actually got suggestions and the message was empty (which shouldn't happen with streaming)
				// or if we want to append a system note.
				// For now, let's NOT overwrite the streamed content.
				if (result.length > 0) {
					setMessages((prev) => {
						const newMessages = [...prev];
						// We don't overwrite the content, the streaming did that.
						// We just ensure the state is consistent.
						return newMessages;
					});
				}
			}
		} catch (e) {
			console.error(e);
			setMessages((prev) => [
				...prev,
				{
					role: "assistant",
					content: "Sorry, I encountered an error processing your request.",
				},
			]);
		} finally {
			setLoading(false);
		}
	};

	if (!isOpen) {
		return (
			<motion.div
				initial={{ scale: 0, opacity: 0 }}
				animate={{ scale: 1, opacity: 1 }}
				className="absolute bottom-6 right-6 z-50"
			>
				<Button
					className="rounded-full w-14 h-14 p-0 shadow-xl bg-linear-to-r from-primary to-purple-600 hover:scale-105 transition-transform"
					onClick={() => setIsOpen(true)}
				>
					<SparklesIcon className="w-6 h-6 text-white" />
				</Button>
			</motion.div>
		);
	}

	return (
		<AnimatePresence>
			{isOpen && (
				<motion.div
					initial={{ opacity: 0, y: 20, scale: 0.95 }}
					animate={{ opacity: 1, y: 0, scale: 1 }}
					exit={{ opacity: 0, y: 20, scale: 0.95 }}
					className="absolute bottom-6 right-6 z-50 w-[420px] h-[600px] bg-linear-to-br from-background via-background to-muted/30 backdrop-blur-xl border border-border/50 rounded-3xl shadow-2xl flex flex-col overflow-hidden ring-1 ring-primary/10"
				>
					{/* Header */}
					<div className="flex flex-col border-b border-border/50 bg-linear-to-r from-primary/5 via-purple-500/5 to-primary/5">
						<div className="p-5 pb-3 flex justify-between items-center">
							<div className="flex items-center gap-3">
								<div className="p-2.5 bg-linear-to-br from-primary to-purple-600 rounded-xl shadow-lg shadow-primary/20">
									<SparklesIcon className="w-5 h-5 text-white" />
								</div>
								<div>
									<h3 className="font-semibold text-base">FlowPilot</h3>
									<p className="text-xs text-muted-foreground flex items-center gap-1">
										<span className="w-1.5 h-1.5 bg-green-500 rounded-full animate-pulse" />
										AI Assistant
									</p>
								</div>
							</div>
							<div className="flex items-center gap-1.5">
								<Button
									variant="ghost"
									size="icon"
									className="h-9 w-9 rounded-xl hover:bg-accent/50 transition-all duration-200"
									onClick={handleNewChat}
									title="New Chat"
								>
									<SquarePenIcon className="w-4 h-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									className="h-9 w-9 rounded-xl hover:bg-destructive/10 hover:text-destructive transition-all duration-200"
									onClick={() => setIsOpen(false)}
								>
									<XIcon className="w-4 h-4" />
								</Button>
							</div>
						</div>

						<div className="px-5 pb-4 flex items-center gap-2">
							<div className="flex-1">
								<Select
									value={selectedModelId}
									onValueChange={setSelectedModelId}
								>
									<SelectTrigger className="h-9 text-xs bg-background/80 backdrop-blur-sm border-border/50 hover:border-primary/30 transition-all duration-200 rounded-xl focus:ring-2 focus:ring-primary/20">
										<SelectValue placeholder="Select Model" />
									</SelectTrigger>
									<SelectContent className="rounded-xl">
										{models.map((model) => (
											<SelectItem
												key={model.id}
												value={model.id}
												className="text-xs rounded-lg"
											>
												{model.meta?.en?.name || model.id}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
							<div className="flex bg-muted/50 backdrop-blur-sm rounded-xl p-1 shrink-0 border border-border/30">
								<Button
									variant={mode === "chat" ? "secondary" : "ghost"}
									size="sm"
									className={`h-7 px-3 text-xs rounded-lg transition-all duration-200 ${mode === "chat" ? "shadow-sm" : ""}`}
									onClick={() => setMode("chat")}
								>
									<MessageSquareIcon className="w-3 h-3 mr-1.5" /> Chat
								</Button>
								<Button
									variant={mode === "autocomplete" ? "secondary" : "ghost"}
									size="sm"
									className={`h-7 px-3 text-xs rounded-lg transition-all duration-200 ${mode === "autocomplete" ? "shadow-sm" : ""}`}
									onClick={() => setMode("autocomplete")}
								>
									<Wand2Icon className="w-3 h-3 mr-1.5" /> Auto
								</Button>
							</div>
						</div>
					</div>

					{/* Chat Area */}
					<ScrollArea className="flex-1 p-5 flex flex-col max-h-full overflow-auto">
						<div className="space-y-3">
							{messages.length === 0 && (
								<div className="text-center text-muted-foreground py-16 space-y-3">
									<div className="relative inline-block">
										<div className="absolute inset-0 bg-primary/20 blur-2xl rounded-full" />
										<SparklesIcon className="w-12 h-12 mx-auto relative text-primary/40" />
									</div>
									<p className="text-sm font-medium">
										How can I help you build your flow today?
									</p>
									<p className="text-xs text-muted-foreground/60">
										Ask me anything about your workflow
									</p>
								</div>
							)}

							{messages.map((m, i) => (
								<motion.div
									initial={{ opacity: 0, y: 10 }}
									animate={{ opacity: 1, y: 0 }}
									key={i}
									className={`flex ${m.role === "user" ? "justify-end" : "justify-start"}`}
								>
									<div
										className={`p-3.5 rounded-2xl text-sm wrap-break-word overflow-hidden transition-all duration-200 ${
											m.role === "user"
												? "bg-linear-to-br from-primary to-primary/90 text-primary-foreground rounded-br-sm max-w-[85%] shadow-md shadow-primary/10"
												: "bg-muted/60 backdrop-blur-sm rounded-bl-sm w-full border border-border/30"
										}`}
									>
										{m.content || loading ? (
											<MessageContent content={m.content} />
										) : null}
									</div>
								</motion.div>
							))}

							{suggestions.length > 0 && (
								<motion.div
									initial={{ opacity: 0, y: 10 }}
									animate={{ opacity: 1, y: 0 }}
									className="space-y-2.5 pl-3 border-l-2 border-primary/30 ml-2"
								>
									<p className="text-xs font-semibold text-foreground/70 mb-2 flex items-center gap-1.5">
										<Wand2Icon className="w-3.5 h-3.5 text-primary" />
										Suggestions
									</p>
									{suggestions.map((s, i) => (
										<div
											key={i}
											className="group border border-border/50 bg-card/80 backdrop-blur-sm hover:bg-accent/30 hover:border-primary/40 p-3.5 rounded-xl cursor-pointer transition-all duration-200 hover:shadow-md hover:shadow-primary/5"
											onClick={() => onAcceptSuggestion(s)}
										>
											<div className="flex items-center justify-between mb-1.5">
												<span className="font-semibold text-sm text-primary">
													{s.node_type}
												</span>
												<Wand2Icon className="w-3.5 h-3.5 opacity-0 group-hover:opacity-100 transition-opacity text-primary" />
											</div>
											<div className="text-xs text-muted-foreground leading-relaxed">
												{s.reason}
											</div>
										</div>
									))}
								</motion.div>
							)}

							{loading &&
								messages.length > 0 &&
								messages[messages.length - 1].role === "user" && (
									<motion.div
										initial={{ opacity: 0, y: 10 }}
										animate={{ opacity: 1, y: 0 }}
										className="flex justify-start"
									>
										<div className="bg-muted/60 backdrop-blur-sm border border-border/30 rounded-2xl rounded-bl-sm px-4 py-3 flex items-center gap-2.5">
											<div className="flex gap-1">
												<span
													className="w-2 h-2 bg-primary rounded-full animate-bounce"
													style={{ animationDelay: "0ms" }}
												/>
												<span
													className="w-2 h-2 bg-primary rounded-full animate-bounce"
													style={{ animationDelay: "150ms" }}
												/>
												<span
													className="w-2 h-2 bg-primary rounded-full animate-bounce"
													style={{ animationDelay: "300ms" }}
												/>
											</div>
											<span className="text-xs text-muted-foreground font-medium">
												Thinking...
											</span>
										</div>
									</motion.div>
								)}
							<div ref={messagesEndRef} />
						</div>
					</ScrollArea>

					{/* Input Area */}
					<div className="p-5 bg-background/80 backdrop-blur-sm border-t border-border/50">
						<div className="relative flex items-center gap-2">
							<Input
								value={input}
								onChange={(e) => setInput(e.target.value)}
								onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
								placeholder={
									mode === "chat"
										? "Describe what you want to change..."
										: "What node should come next?"
								}
								className="pr-11 rounded-xl border-border/50 focus-visible:ring-2 focus-visible:ring-primary/20 focus-visible:border-primary/50 bg-background/80 backdrop-blur-sm transition-all duration-200"
							/>
							<Button
								size="icon"
								onClick={handleSubmit}
								disabled={loading || !input.trim()}
								className="absolute right-1.5 h-9 w-9 rounded-lg shadow-md bg-linear-to-br from-primary to-purple-600 hover:shadow-lg hover:shadow-primary/20 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
							>
								<SendIcon className="w-4 h-4" />
							</Button>
						</div>
					</div>
				</motion.div>
			)}
		</AnimatePresence>
	);
}
