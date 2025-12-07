"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	ArrowDownIcon,
	BotIcon,
	BrainCircuitIcon,
	CheckCircle2Icon,
	ChevronDown,
	EditIcon,
	ImageIcon,
	ListChecksIcon,
	Loader2Icon,
	SendIcon,
	SparklesIcon,
	SquarePenIcon,
	Wand2Icon,
	WrenchIcon,
	XIcon,
} from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "../../ui/button";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../../ui/collapsible";
import { ScrollArea } from "../../ui/scroll-area";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../ui/tooltip";

import { useInvoke } from "../../../hooks";
import { IBitTypes } from "../../../lib";
import { useBackend } from "../../../state/backend-state";

import { MessageContent } from "./message-content";
import { PendingCommandsView } from "./pending-commands-view";
import { PlanStepsView } from "./plan-steps-view";
import { StatusPill } from "./status-pill";
import type { FlowCopilotProps, LoadingPhase } from "./types";
import { getCommandIcon } from "./utils";

import type {
	BoardCommand,
	ChatMessage,
	PlanStep,
	Suggestion,
} from "../../../lib/schema/flow/copilot";

interface AttachedImage {
	data: string; // Base64 without prefix
	mediaType: string;
	preview: string; // Data URL for preview
}

interface Message {
	role: "user" | "assistant";
	content: string;
	images?: AttachedImage[];
	agentType?: string;
	planSteps?: PlanStep[];
	executedCommands?: BoardCommand[];
}

export function FlowCopilot({
	board,
	selectedNodeIds,
	onAcceptSuggestion,
	onExecuteCommands,
	mode = "chat",
	embedded = false,
	runContext,
	onFocusNode,
	onClose,
}: FlowCopilotProps) {
	const [isOpen, setIsOpen] = useState(mode === "panel" || embedded);
	const [messages, setMessages] = useState<Message[]>([]);
	const [input, setInput] = useState("");
	const [loading, setLoading] = useState(false);
	const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
	const [pendingCommands, setPendingCommands] = useState<BoardCommand[]>([]);
	const [autocompleteEnabled, setAutocompleteEnabled] = useState(false);
	const [selectedModelId, setSelectedModelId] = useState("");
	const [loadingPhase, setLoadingPhase] =
		useState<LoadingPhase>("initializing");
	const [loadingStartTime, setLoadingStartTime] = useState<number | null>(null);
	const [tokenCount, setTokenCount] = useState(0);
	const [currentToolCall, setCurrentToolCall] = useState<string | null>(null);
	const [planSteps, setPlanSteps] = useState<PlanStep[]>([]);
	const [userScrolledUp, setUserScrolledUp] = useState(false);
	const [attachedImages, setAttachedImages] = useState<AttachedImage[]>([]);

	const scrollContainerRef = useRef<HTMLDivElement>(null);
	const messagesEndRef = useRef<HTMLDivElement>(null);
	const imageInputRef = useRef<HTMLInputElement>(null);

	// Use backend context to fetch models
	const backendContext = useBackend();

	// Fetch user profile
	const profile = useInvoke(
		backendContext.userState.getSettingsProfile,
		backendContext.userState,
		[],
		isOpen,
	);

	// Fetch available models (LLMs and VLMs)
	const foundBits = useInvoke(
		backendContext.bitState.searchBits,
		backendContext.bitState,
		[
			{
				bit_types: [IBitTypes.Llm, IBitTypes.Vlm],
			},
		],
		isOpen && !!profile.data,
		[profile.data?.hub_profile.id],
	);

	const models = foundBits.data || [];

	// Calculate elapsed time
	const [elapsedSeconds, setElapsedSeconds] = useState(0);
	useEffect(() => {
		if (!loadingStartTime) {
			setElapsedSeconds(0);
			return;
		}
		const interval = setInterval(() => {
			setElapsedSeconds(Math.floor((Date.now() - loadingStartTime) / 1000));
		}, 1000);
		return () => clearInterval(interval);
	}, [loadingStartTime]);

	// Set default model when models are loaded
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

	const scrollToBottom = useCallback((force?: boolean) => {
		if (force) {
			setUserScrolledUp(false);
		}
		messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, []);

	const handleScroll = useCallback(() => {
		const container = scrollContainerRef.current;
		if (!container) return;

		const { scrollTop, scrollHeight, clientHeight } = container;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
		const isAtBottom = distanceFromBottom < 30;

		setUserScrolledUp(!isAtBottom);
	}, []);

	// Auto-scroll on new messages (only if user hasn't scrolled up)
	useEffect(() => {
		if (!userScrolledUp) {
			scrollToBottom();
			const timeout = setTimeout(() => scrollToBottom(), 100);
			return () => clearTimeout(timeout);
		}
	}, [messages, suggestions, loading, userScrolledUp, scrollToBottom]);

	const handleNewChat = () => {
		setMessages([]);
		setSuggestions([]);
		setPendingCommands([]);
		setPlanSteps([]);
		setInput("");
		setAttachedImages([]);
	};

	const handleImageSelect = useCallback(
		(e: React.ChangeEvent<HTMLInputElement>) => {
			const files = e.target.files;
			if (!files) return;

			Array.from(files).forEach((file) => {
				if (!file.type.startsWith("image/")) return;

				const reader = new FileReader();
				reader.onload = (event) => {
					const dataUrl = event.target?.result as string;
					if (!dataUrl) return;

					// Extract base64 data without the data URL prefix
					const base64Data = dataUrl.split(",")[1];
					const mediaType = file.type;

					setAttachedImages((prev) => [
						...prev,
						{
							data: base64Data,
							mediaType,
							preview: dataUrl,
						},
					]);
				};
				reader.readAsDataURL(file);
			});

			// Reset input so same file can be selected again
			if (imageInputRef.current) {
				imageInputRef.current.value = "";
			}
		},
		[],
	);

	const handleRemoveImage = useCallback((index: number) => {
		setAttachedImages((prev) => prev.filter((_, i) => i !== index));
	}, []);

	const handlePaste = useCallback((e: React.ClipboardEvent) => {
		const items = e.clipboardData?.items;
		if (!items) return;

		for (const item of Array.from(items)) {
			if (item.type.startsWith("image/")) {
				e.preventDefault();
				const file = item.getAsFile();
				if (!file) continue;

				const reader = new FileReader();
				reader.onload = (event) => {
					const dataUrl = event.target?.result as string;
					if (!dataUrl) return;

					const base64Data = dataUrl.split(",")[1];
					const mediaType = file.type;

					setAttachedImages((prev) => [
						...prev,
						{
							data: base64Data,
							mediaType,
							preview: dataUrl,
						},
					]);
				};
				reader.readAsDataURL(file);
			}
		}
	}, []);

	const handleKeyDown = useCallback(
		(e: React.KeyboardEvent<HTMLTextAreaElement>) => {
			// Submit on Enter without Shift
			if (e.key === "Enter" && !e.shiftKey) {
				e.preventDefault();
				handleSubmitRef.current?.();
			}
		},
		[],
	);

	// Store handleSubmit in ref for use in handleKeyDown
	const handleSubmitRef = useRef<(() => void) | null>(null);

	const handleExecuteCommands = () => {
		if (onExecuteCommands && pendingCommands.length > 0) {
			onExecuteCommands(pendingCommands);
			setMessages((prev) => {
				const newMessages = [...prev];
				for (let i = newMessages.length - 1; i >= 0; i--) {
					if (newMessages[i].role === "assistant") {
						const existingCommands = newMessages[i].executedCommands || [];
						newMessages[i] = {
							...newMessages[i],
							executedCommands: [...existingCommands, ...pendingCommands],
						};
						break;
					}
				}
				return newMessages;
			});
			setPendingCommands([]);
		}
	};

	const handleExecuteSingle = (index: number) => {
		if (onExecuteCommands && pendingCommands[index]) {
			const command = pendingCommands[index];
			onExecuteCommands([command]);
			setMessages((prev) => {
				const newMessages = [...prev];
				for (let i = newMessages.length - 1; i >= 0; i--) {
					if (newMessages[i].role === "assistant") {
						const existingCommands = newMessages[i].executedCommands || [];
						newMessages[i] = {
							...newMessages[i],
							executedCommands: [...existingCommands, command],
						};
						break;
					}
				}
				return newMessages;
			});
			setPendingCommands((prev) => prev.filter((_, i) => i !== index));
		}
	};

	const handleDismissCommands = () => {
		setPendingCommands([]);
	};

	const handleSubmit = useCallback(async () => {
		if (!input.trim() && attachedImages.length === 0) return;

		let userMsg = input;
		if (runContext) {
			const runInfo = {
				run_id: runContext.run_id,
				app_id: runContext.app_id,
				board_id: runContext.board_id,
				event_id: runContext.event_id,
			};
			userMsg = `[RUN CONTEXT - User is asking about a flow execution run. Use the query_logs tool to fetch relevant logs.]
\`\`\`json
${JSON.stringify(runInfo, null, 2)}
\`\`\`

${input}`;
		}

		// Capture current images before clearing
		const currentImages = [...attachedImages];

		setMessages((prev) => [
			...prev,
			{
				role: "user",
				content: input,
				images: currentImages.length > 0 ? currentImages : undefined,
			},
		]);
		setInput("");
		setAttachedImages([]);
		setLoading(true);
		setLoadingPhase("initializing");
		setLoadingStartTime(Date.now());
		setTokenCount(0);
		setSuggestions([]);
		setPendingCommands([]);
		setPlanSteps([]);
		setUserScrolledUp(false);

		try {
			if (!board || !backendContext) return;

			let currentMessageContent = "";
			setMessages((prev) => [...prev, { role: "assistant", content: "" }]);

			setTimeout(() => setLoadingPhase("analyzing"), 300);

			const onToken = (token: string) => {
				setTokenCount((prev) => prev + 1);

				const planStepMatch = token.match(/<plan_step>([\s\S]*?)<\/plan_step>/);
				if (planStepMatch) {
					try {
						const eventData = JSON.parse(planStepMatch[1]);
						if (eventData.PlanStep) {
							const step = eventData.PlanStep;
							if (step.tool_name === "think") {
								setLoadingPhase("reasoning");
							} else if (
								step.tool_name?.includes("search") ||
								step.tool_name?.includes("catalog")
							) {
								setLoadingPhase("searching");
							} else if (step.tool_name === "emit_commands") {
								setLoadingPhase("generating");
							}

							setPlanSteps((prev) => {
								const existingIndex = prev.findIndex((s) => s.id === step.id);
								if (existingIndex >= 0) {
									const updated = [...prev];
									updated[existingIndex] = step;
									return updated;
								}
								return [...prev, step];
							});
						}
					} catch {
						// Not valid JSON, ignore
					}
					return;
				}

				if (token.includes("<commands>") && token.includes("</commands>")) {
					return;
				}

				if (token.includes("tool_call:")) {
					const match = token.match(/tool_call:(\w+)/);
					if (match) {
						const toolName = match[1];
						setCurrentToolCall(toolName);
						if (
							toolName.includes("search") ||
							toolName.includes("catalog") ||
							toolName.includes("filter")
						) {
							setLoadingPhase("searching");
						} else if (toolName === "think") {
							setLoadingPhase("reasoning");
						} else if (toolName === "emit_commands") {
							setLoadingPhase("generating");
						}
					}
					return;
				}
				if (token.includes("tool_result:")) {
					setCurrentToolCall(null);
					return;
				}

				if (currentMessageContent.length === 0 && token.trim()) {
					setLoadingPhase("generating");
				}

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

			const chatHistory: ChatMessage[] = messages.map((m) => ({
				role: m.role === "user" ? "User" : "Assistant",
				content: m.content,
				images: m.images?.map((img) => ({
					data: img.data,
					media_type: img.mediaType,
				})),
			}));

			// Add current message with images to history for this request
			if (currentImages.length > 0) {
				chatHistory.push({
					role: "User",
					content: userMsg,
					images: currentImages.map((img) => ({
						data: img.data,
						media_type: img.mediaType,
					})),
				});
			}

			const backendRunContext = runContext
				? {
						run_id: runContext.run_id,
						app_id: runContext.app_id,
						board_id: runContext.board_id,
					}
				: undefined;

			const response = await backendContext.boardState.flowpilot_chat(
				board,
				selectedNodeIds,
				userMsg,
				chatHistory,
				onToken,
				selectedModelId,
				undefined,
				backendRunContext,
			);

			setMessages((prev) => {
				const newMessages = [...prev];
				const lastMessage = newMessages[newMessages.length - 1];
				if (lastMessage && lastMessage.role === "assistant") {
					lastMessage.agentType = response.agent_type;
					lastMessage.planSteps = planSteps.filter(
						(s) => s.status === "Completed",
					);
					if (!lastMessage.content.trim()) {
						lastMessage.content = response.message;
					}
				}
				return newMessages;
			});

			setPendingCommands(response.commands);

			if (response.suggestions.length > 0) {
				setSuggestions(response.suggestions);
			}

			setLoadingPhase("finalizing");
		} catch (e) {
			console.error("[FlowCopilot] Error:", e);
			const errorMessage = e instanceof Error ? e.message : String(e);
			setMessages((prev) => [
				...prev,
				{
					role: "assistant",
					content: `Sorry, I encountered an error: ${errorMessage}`,
				},
			]);
		} finally {
			setLoading(false);
			setLoadingStartTime(null);
			setCurrentToolCall(null);
			setTokenCount(0);
		}
	}, [
		input,
		attachedImages,
		runContext,
		board,
		backendContext,
		messages,
		selectedNodeIds,
		selectedModelId,
		planSteps,
	]);

	// Keep ref updated for keydown handler
	useEffect(() => {
		handleSubmitRef.current = handleSubmit;
	}, [handleSubmit]);

	// Panel mode - renders directly as a panel (controlled externally)
	if (mode === "panel") {
		return (
			<PanelView
				messages={messages}
				input={input}
				setInput={setInput}
				loading={loading}
				loadingPhase={loadingPhase}
				elapsedSeconds={elapsedSeconds}
				suggestions={suggestions}
				pendingCommands={pendingCommands}
				planSteps={planSteps}
				currentToolCall={currentToolCall}
				userScrolledUp={userScrolledUp}
				scrollContainerRef={scrollContainerRef}
				messagesEndRef={messagesEndRef}
				handleScroll={handleScroll}
				scrollToBottom={scrollToBottom}
				handleNewChat={handleNewChat}
				handleSubmit={handleSubmit}
				handleExecuteCommands={handleExecuteCommands}
				handleExecuteSingle={handleExecuteSingle}
				handleDismissCommands={handleDismissCommands}
				onAcceptSuggestion={onAcceptSuggestion}
				onFocusNode={onFocusNode}
				onClose={onClose}
				board={board}
				models={models}
				selectedModelId={selectedModelId}
				setSelectedModelId={setSelectedModelId}
				attachedImages={attachedImages}
				imageInputRef={imageInputRef}
				handleImageSelect={handleImageSelect}
				handleRemoveImage={handleRemoveImage}
				handlePaste={handlePaste}
				handleKeyDown={handleKeyDown}
				runContext={runContext}
			/>
		);
	}

	// Embedded mode - renders inline in the logs panel
	if (embedded) {
		return (
			<EmbeddedView
				messages={messages}
				input={input}
				setInput={setInput}
				loading={loading}
				loadingPhase={loadingPhase}
				elapsedSeconds={elapsedSeconds}
				suggestions={suggestions}
				pendingCommands={pendingCommands}
				planSteps={planSteps}
				currentToolCall={currentToolCall}
				userScrolledUp={userScrolledUp}
				scrollContainerRef={scrollContainerRef}
				messagesEndRef={messagesEndRef}
				handleScroll={handleScroll}
				scrollToBottom={scrollToBottom}
				handleNewChat={handleNewChat}
				handleSubmit={handleSubmit}
				handleExecuteCommands={handleExecuteCommands}
				handleExecuteSingle={handleExecuteSingle}
				handleDismissCommands={handleDismissCommands}
				onAcceptSuggestion={onAcceptSuggestion}
				onFocusNode={onFocusNode}
				board={board}
				models={models}
				selectedModelId={selectedModelId}
				setSelectedModelId={setSelectedModelId}
				attachedImages={attachedImages}
				imageInputRef={imageInputRef}
				handleImageSelect={handleImageSelect}
				handleRemoveImage={handleRemoveImage}
				handlePaste={handlePaste}
				handleKeyDown={handleKeyDown}
			/>
		);
	}

	// Legacy floating button mode (when not in panel/embedded mode)
	if (!isOpen) {
		return (
			<motion.div
				initial={{ scale: 0, opacity: 0 }}
				animate={{ scale: 1, opacity: 1 }}
				className="absolute bottom-4 right-4 z-40 md:top-20 md:bottom-auto"
			>
				<div className="relative group">
					<div className="absolute -inset-2 bg-primary/30 rounded-full blur-xl opacity-0 group-hover:opacity-50 transition-all duration-300" />

					<Button
						className="relative rounded-full w-12 h-12 p-0 shadow-lg bg-background/90 backdrop-blur-sm border border-border/50 hover:border-primary/50 hover:bg-background hover:shadow-xl transition-all duration-200"
						onClick={() => setIsOpen(true)}
					>
						<SparklesIcon className="w-5 h-5 text-primary" />
					</Button>

					{runContext && (
						<motion.div
							initial={{ scale: 0 }}
							animate={{ scale: 1 }}
							className="absolute -bottom-0.5 -right-0.5 w-4 h-4 bg-amber-500 border-2 border-background rounded-full flex items-center justify-center shadow-md"
							title="Log context available"
						>
							<SparklesIcon className="w-2 h-2 text-white" />
						</motion.div>
					)}

					{autocompleteEnabled && !runContext && (
						<motion.div
							initial={{ scale: 0 }}
							animate={{ scale: 1 }}
							className="absolute -bottom-0.5 -right-0.5 w-4 h-4 bg-green-500 border-2 border-background rounded-full flex items-center justify-center shadow-md"
						>
							<Wand2Icon className="w-2.5 h-2.5 text-white" />
						</motion.div>
					)}
				</div>
			</motion.div>
		);
	}

	// Main floating panel view
	return (
		<AnimatePresence>
			{isOpen && (
				<FloatingPanelView
					messages={messages}
					input={input}
					setInput={setInput}
					loading={loading}
					loadingPhase={loadingPhase}
					elapsedSeconds={elapsedSeconds}
					suggestions={suggestions}
					pendingCommands={pendingCommands}
					planSteps={planSteps}
					currentToolCall={currentToolCall}
					userScrolledUp={userScrolledUp}
					autocompleteEnabled={autocompleteEnabled}
					setAutocompleteEnabled={setAutocompleteEnabled}
					scrollContainerRef={scrollContainerRef}
					messagesEndRef={messagesEndRef}
					handleScroll={handleScroll}
					scrollToBottom={scrollToBottom}
					handleNewChat={handleNewChat}
					handleSubmit={handleSubmit}
					handleExecuteCommands={handleExecuteCommands}
					handleExecuteSingle={handleExecuteSingle}
					handleDismissCommands={handleDismissCommands}
					onAcceptSuggestion={onAcceptSuggestion}
					onFocusNode={onFocusNode}
					onClose={() => setIsOpen(false)}
					board={board}
					mode={mode}
					models={models}
					selectedModelId={selectedModelId}
					setSelectedModelId={setSelectedModelId}
					attachedImages={attachedImages}
					imageInputRef={imageInputRef}
					handleImageSelect={handleImageSelect}
					handleRemoveImage={handleRemoveImage}
					handlePaste={handlePaste}
					handleKeyDown={handleKeyDown}
				/>
			)}
		</AnimatePresence>
	);
}

// Embedded view component - memoized to prevent re-renders
interface EmbeddedViewProps {
	messages: Message[];
	input: string;
	setInput: (value: string) => void;
	loading: boolean;
	loadingPhase: LoadingPhase;
	elapsedSeconds: number;
	suggestions: Suggestion[];
	pendingCommands: BoardCommand[];
	planSteps: PlanStep[];
	currentToolCall: string | null;
	userScrolledUp: boolean;
	scrollContainerRef: React.RefObject<HTMLDivElement | null>;
	messagesEndRef: React.RefObject<HTMLDivElement | null>;
	handleScroll: () => void;
	scrollToBottom: (force?: boolean) => void;
	handleNewChat: () => void;
	handleSubmit: () => void;
	handleExecuteCommands: () => void;
	handleExecuteSingle: (index: number) => void;
	handleDismissCommands: () => void;
	onAcceptSuggestion: (suggestion: Suggestion) => void;
	onFocusNode?: (nodeId: string) => void;
	board: any;
	models: any[];
	selectedModelId: string;
	setSelectedModelId: (id: string) => void;
	attachedImages: AttachedImage[];
	imageInputRef: React.RefObject<HTMLInputElement | null>;
	handleImageSelect: (e: React.ChangeEvent<HTMLInputElement>) => void;
	handleRemoveImage: (index: number) => void;
	handlePaste: (e: React.ClipboardEvent) => void;
	handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
}

const EmbeddedView = memo(function EmbeddedView({
	messages,
	input,
	setInput,
	loading,
	loadingPhase,
	elapsedSeconds,
	suggestions,
	pendingCommands,
	planSteps,
	currentToolCall,
	userScrolledUp,
	scrollContainerRef,
	messagesEndRef,
	handleScroll,
	scrollToBottom,
	handleNewChat,
	handleSubmit,
	handleExecuteCommands,
	handleExecuteSingle,
	handleDismissCommands,
	onAcceptSuggestion,
	onFocusNode,
	board,
	models,
	selectedModelId,
	setSelectedModelId,
	attachedImages,
	imageInputRef,
	handleImageSelect,
	handleRemoveImage,
	handlePaste,
	handleKeyDown,
}: EmbeddedViewProps) {
	return (
		<motion.div
			layoutId="flow-copilot"
			initial={{ opacity: 0, x: 100 }}
			animate={{ opacity: 1, x: 0 }}
			exit={{ opacity: 0, x: 100 }}
			transition={{ type: "spring", stiffness: 400, damping: 30 }}
			className="h-full w-full flex flex-col bg-background/95 backdrop-blur-xl border-l border-border/40 overflow-hidden"
		>
			{/* Compact Header */}
			<div className="relative overflow-hidden shrink-0">
				<div className="absolute inset-0 bg-linear-to-br from-primary/8 via-violet-500/5 to-pink-500/5" />
				{loading && (
					<motion.div
						className="absolute inset-0 opacity-30"
						style={{
							background:
								"radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
						}}
						animate={{
							background: [
								"radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
								"radial-gradient(circle at 70% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 30% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
							],
						}}
						transition={{
							duration: 3,
							repeat: Number.POSITIVE_INFINITY,
							repeatType: "reverse",
						}}
					/>
				)}

				<div className="relative px-3 py-2.5 flex items-center justify-between">
					<div className="flex items-center gap-2">
						<div className="relative">
							<motion.div
								className="absolute inset-0 bg-linear-to-br from-primary to-violet-600 rounded-lg blur-md opacity-50"
								animate={
									loading
										? { scale: [1, 1.2, 1], opacity: [0.5, 0.8, 0.5] }
										: {}
								}
								transition={{ duration: 2, repeat: Number.POSITIVE_INFINITY }}
							/>
							<div className="relative p-1.5 bg-linear-to-br from-primary via-violet-600 to-pink-600 rounded-lg shadow-md">
								<SparklesIcon className="w-3.5 h-3.5 text-white" />
							</div>
						</div>
						<div>
							<h3 className="text-[10px] font-bold">FlowPilot</h3>
							{loading ? (
								<StatusPill phase={loadingPhase} elapsed={elapsedSeconds} />
							) : (
								<div className="flex items-center gap-1 text-[10px] text-muted-foreground">
									<span className="relative flex h-1.5 w-1.5">
										<span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
										<span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-green-500" />
									</span>
									Ready
								</div>
							)}
						</div>
					</div>
					<div className="flex items-center gap-1">
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="ghost"
									size="icon"
									className="h-6 w-6 rounded-md hover:bg-accent/50"
									onClick={handleNewChat}
								>
									<SquarePenIcon className="w-3 h-3" />
								</Button>
							</TooltipTrigger>
							<TooltipContent side="bottom" className="text-xs">
								New chat
							</TooltipContent>
						</Tooltip>
					</div>
				</div>

				{/* Model selector */}
				<div className="relative px-3 pb-2">
					<Select value={selectedModelId} onValueChange={setSelectedModelId}>
						<SelectTrigger className="h-7 text-[10px] bg-background/60 backdrop-blur-sm border-border/30 hover:border-primary/30 transition-all duration-200 rounded-lg focus:ring-2 focus:ring-primary/20">
							<div className="flex items-center gap-1.5">
								<BotIcon className="w-3 h-3 text-muted-foreground" />
								<SelectValue placeholder="Select Model" />
							</div>
						</SelectTrigger>
						<SelectContent className="rounded-lg">
							{models.map((model) => (
								<SelectItem
									key={model.id}
									value={model.id}
									className="text-[10px] rounded-md"
								>
									{model.meta?.en?.name || model.id}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>

				{loading && (
					<motion.div
						className="absolute bottom-0 left-0 right-0 h-0.5 bg-muted/30"
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
					>
						<motion.div
							className="h-full bg-linear-to-r from-primary via-violet-500 to-pink-500"
							initial={{ width: "0%" }}
							animate={{ width: "100%" }}
							transition={{ duration: 30, ease: "linear" }}
						/>
					</motion.div>
				)}
			</div>

			{/* Messages area */}
			<ScrollArea
				className="flex-1 min-h-0 px-3"
				viewportRef={scrollContainerRef}
				onScroll={handleScroll}
			>
				<div className="py-3 space-y-3 min-w-0">
					<MessagesArea
						messages={messages}
						loading={loading}
						planSteps={planSteps}
						currentToolCall={currentToolCall}
						pendingCommands={pendingCommands}
						suggestions={suggestions}
						onFocusNode={onFocusNode}
						onAcceptSuggestion={onAcceptSuggestion}
						handleExecuteCommands={handleExecuteCommands}
						handleExecuteSingle={handleExecuteSingle}
						handleDismissCommands={handleDismissCommands}
						board={board}
						setInput={setInput}
						embedded={true}
					/>
					<div ref={messagesEndRef} />
				</div>
			</ScrollArea>

			{/* Scroll to bottom indicator */}
			<AnimatePresence>
				{userScrolledUp && messages.length > 0 && (
					<motion.div
						initial={{ opacity: 0, y: 10 }}
						animate={{ opacity: 1, y: 0 }}
						exit={{ opacity: 0, y: 10 }}
						className="absolute bottom-16 left-1/2 -translate-x-1/2 z-10"
					>
						<Button
							size="sm"
							variant="secondary"
							className="rounded-full shadow-lg border border-border/50 gap-1 px-2 h-6 text-[10px]"
							onClick={() => scrollToBottom(true)}
						>
							<ArrowDownIcon className="w-3 h-3" />
							New
						</Button>
					</motion.div>
				)}
			</AnimatePresence>

			{/* Input area */}
			<div className="shrink-0 p-2.5 border-t border-border/30 bg-background/80 backdrop-blur-sm">
				{/* Image previews */}
				{attachedImages.length > 0 && (
					<div className="flex gap-1.5 mb-2 flex-wrap">
						{attachedImages.map((img, idx) => (
							<div key={idx} className="relative group">
								<img
									src={img.preview}
									alt={`Attached ${idx + 1}`}
									className="h-12 w-12 object-cover rounded-md border border-border/50"
								/>
								<button
									type="button"
									onClick={() => handleRemoveImage(idx)}
									className="absolute -top-1 -right-1 w-4 h-4 bg-destructive text-destructive-foreground rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
								>
									<XIcon className="w-2.5 h-2.5" />
								</button>
							</div>
						))}
					</div>
				)}
				<div className="relative flex items-start gap-1.5">
					<input
						type="file"
						ref={imageInputRef}
						accept="image/*"
						multiple
						onChange={handleImageSelect}
						className="hidden"
					/>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								className="h-8 w-8 shrink-0 rounded-lg hover:bg-accent/50 mt-0.5"
								onClick={() => imageInputRef.current?.click()}
								disabled={loading}
							>
								<ImageIcon className="w-3.5 h-3.5" />
							</Button>
						</TooltipTrigger>
						<TooltipContent side="top" className="text-xs">
							Attach image (paste supported)
						</TooltipContent>
					</Tooltip>
					<textarea
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyDown={handleKeyDown}
						onPaste={handlePaste}
						placeholder="Ask about the logs... (Shift+Enter for new line)"
						className="flex-1 min-h-8 max-h-[120px] text-[10px] py-2 px-3 pr-9 rounded-lg bg-background/80 border border-border/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/20 focus-visible:border-primary/50 resize-none"
						disabled={loading}
						rows={1}
					/>
					<Button
						size="icon"
						onClick={handleSubmit}
						disabled={loading || (!input.trim() && attachedImages.length === 0)}
						className="absolute right-0 top-0.5 h-8 w-8 rounded-lg shadow-md bg-linear-to-br from-primary to-purple-600 hover:shadow-lg hover:shadow-primary/20 transition-all duration-200 disabled:opacity-50"
					>
						{loading ? (
							<Loader2Icon className="w-3.5 h-3.5 animate-spin" />
						) : (
							<SendIcon className="w-3.5 h-3.5" />
						)}
					</Button>
				</div>
			</div>
		</motion.div>
	);
});

// Panel view component - for use when opened from header button (with close button)
interface PanelViewProps extends EmbeddedViewProps {
	onClose?: () => void;
	runContext?: any;
}

const PanelView = memo(function PanelView({
	messages,
	input,
	setInput,
	loading,
	loadingPhase,
	elapsedSeconds,
	suggestions,
	pendingCommands,
	planSteps,
	currentToolCall,
	userScrolledUp,
	scrollContainerRef,
	messagesEndRef,
	handleScroll,
	scrollToBottom,
	handleNewChat,
	handleSubmit,
	handleExecuteCommands,
	handleExecuteSingle,
	handleDismissCommands,
	onAcceptSuggestion,
	onFocusNode,
	onClose,
	board,
	models,
	selectedModelId,
	setSelectedModelId,
	attachedImages,
	imageInputRef,
	handleImageSelect,
	handleRemoveImage,
	handlePaste,
	handleKeyDown,
	runContext,
}: PanelViewProps) {
	return (
		<motion.div
			layoutId="flow-copilot-panel"
			initial={{ opacity: 0, scale: 0.95, y: -10 }}
			animate={{ opacity: 1, scale: 1, y: 0 }}
			exit={{ opacity: 0, scale: 0.95, y: -10 }}
			transition={{ type: "spring", stiffness: 400, damping: 30 }}
			className="h-full w-full flex flex-col bg-background/95 backdrop-blur-xl border border-border/40 rounded-2xl overflow-hidden shadow-2xl"
		>
			{/* Header */}
			<div className="relative overflow-hidden shrink-0">
				<div className="absolute inset-0 bg-linear-to-br from-primary/8 via-violet-500/5 to-pink-500/5" />
				{loading && (
					<motion.div
						className="absolute inset-0 opacity-30"
						style={{
							background:
								"radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
						}}
						animate={{
							background: [
								"radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
								"radial-gradient(circle at 70% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 30% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
							],
						}}
						transition={{
							duration: 3,
							repeat: Number.POSITIVE_INFINITY,
							repeatType: "reverse",
						}}
					/>
				)}

				<div className="relative px-4 py-3 flex items-center justify-between">
					<div className="flex items-center gap-3">
						<div className="relative">
							<motion.div
								className="absolute inset-0 bg-linear-to-br from-primary to-violet-600 rounded-xl blur-lg opacity-50"
								animate={
									loading
										? { scale: [1, 1.2, 1], opacity: [0.5, 0.8, 0.5] }
										: {}
								}
								transition={{ duration: 2, repeat: Number.POSITIVE_INFINITY }}
							/>
							<div className="relative p-2 bg-linear-to-br from-primary via-violet-600 to-pink-600 rounded-xl shadow-lg">
								<SparklesIcon className="w-4 h-4 text-white" />
							</div>
						</div>
						<div>
							<h3 className="text-sm font-bold">FlowPilot</h3>
							{loading ? (
								<StatusPill phase={loadingPhase} elapsed={elapsedSeconds} />
							) : (
								<div className="flex items-center gap-1.5 text-xs text-muted-foreground">
									<span className="relative flex h-1.5 w-1.5">
										<span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
										<span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-green-500" />
									</span>
									{runContext ? "Log context active" : "Ready"}
								</div>
							)}
						</div>
					</div>
					<div className="flex items-center gap-1">
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="ghost"
									size="icon"
									className="h-7 w-7 rounded-lg hover:bg-accent/50"
									onClick={handleNewChat}
								>
									<SquarePenIcon className="w-3.5 h-3.5" />
								</Button>
							</TooltipTrigger>
							<TooltipContent side="bottom" className="text-xs">
								New chat
							</TooltipContent>
						</Tooltip>
						{onClose && (
							<Tooltip>
								<TooltipTrigger asChild>
									<Button
										variant="ghost"
										size="icon"
										className="h-7 w-7 rounded-lg hover:bg-accent/50"
										onClick={onClose}
									>
										<XIcon className="w-3.5 h-3.5" />
									</Button>
								</TooltipTrigger>
								<TooltipContent side="bottom" className="text-xs">
									Close
								</TooltipContent>
							</Tooltip>
						)}
					</div>
				</div>

				{/* Model selector */}
				<div className="relative px-4 pb-3">
					<Select value={selectedModelId} onValueChange={setSelectedModelId}>
						<SelectTrigger className="h-8 text-xs bg-background/60 backdrop-blur-sm border-border/30 hover:border-primary/30 transition-all duration-200 rounded-lg focus:ring-2 focus:ring-primary/20">
							<div className="flex items-center gap-1.5">
								<BotIcon className="w-3.5 h-3.5 text-muted-foreground" />
								<SelectValue placeholder="Select Model" />
							</div>
						</SelectTrigger>
						<SelectContent className="rounded-lg">
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

				{loading && (
					<motion.div
						className="absolute bottom-0 left-0 right-0 h-0.5 bg-muted/30"
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
					>
						<motion.div
							className="h-full bg-linear-to-r from-primary via-violet-500 to-pink-500"
							initial={{ width: "0%" }}
							animate={{ width: "100%" }}
							transition={{ duration: 30, ease: "linear" }}
						/>
					</motion.div>
				)}
			</div>

			{/* Chat Area */}
			<ScrollArea
				className="flex-1 min-h-0 p-4 flex flex-col"
				viewportRef={scrollContainerRef}
				onScroll={handleScroll}
			>
				{messages.length === 0 ? (
					<div className="flex-1 flex items-center justify-center py-8">
						<div className="text-center max-w-[280px]">
							<div className="relative inline-flex p-4 rounded-2xl bg-linear-to-br from-primary/10 via-violet-500/5 to-pink-500/5 mb-4">
								<SparklesIcon className="w-8 h-8 text-primary" />
							</div>
							<h4 className="font-semibold text-sm mb-1.5">
								Welcome to FlowPilot
							</h4>
							<p className="text-xs text-muted-foreground leading-relaxed">
								{runContext
									? "I can help you analyze logs and debug your flow. Ask me anything about the current run!"
									: "Ask questions, get help building flows, or let me analyze your workflow."}
							</p>
						</div>
					</div>
				) : (
					<div className="space-y-4">
						{messages.map((message, index) => (
							<div
								key={`${message.role}-${index}`}
								className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
							>
								<div
									className={`px-3 py-2 rounded-xl text-sm wrap-break-word overflow-hidden min-w-0 ${
										message.role === "user"
											? "bg-muted/60 text-foreground rounded-br-sm max-w-[85%] border border-border/40"
											: "bg-muted/40 backdrop-blur-sm rounded-bl-sm max-w-full border border-border/30"
									}`}
								>
									{message.content && (
										<MessageContent
											content={message.content}
											onFocusNode={onFocusNode}
											board={board}
										/>
									)}
									{/* Show loading indicator for last assistant message */}
									{message.role === "assistant" &&
										index === messages.length - 1 &&
										loading &&
										!message.content && (
											<div className="flex items-center gap-2">
												<div className="flex gap-0.5">
													{[0, 1, 2].map((j) => (
														<span
															key={j}
															className="w-1 h-1 bg-primary rounded-full animate-pulse"
															style={{ animationDelay: `${j * 0.2}s` }}
														/>
													))}
												</div>
												<span className="text-xs text-muted-foreground">
													{currentToolCall
														? `Using ${currentToolCall.replace(/_/g, " ")}...`
														: "Processing..."}
												</span>
											</div>
										)}
								</div>
							</div>
						))}
						{/* Pending commands */}
						{pendingCommands.length > 0 && (
							<PendingCommandsView
								commands={pendingCommands}
								onExecute={handleExecuteCommands}
								onExecuteSingle={handleExecuteSingle}
								onDismiss={handleDismissCommands}
							/>
						)}
						{/* Plan steps */}
						{loading && planSteps.length > 0 && (
							<PlanStepsView steps={planSteps} />
						)}
					</div>
				)}
				<div ref={messagesEndRef} />
			</ScrollArea>

			{/* Scroll to bottom button */}
			<AnimatePresence>
				{userScrolledUp && messages.length > 0 && (
					<motion.div
						initial={{ opacity: 0, y: 10 }}
						animate={{ opacity: 1, y: 0 }}
						exit={{ opacity: 0, y: 10 }}
						className="absolute bottom-24 left-1/2 -translate-x-1/2"
					>
						<Button
							size="sm"
							variant="secondary"
							onClick={() => scrollToBottom(true)}
							className="rounded-full shadow-lg text-xs px-3 h-7"
						>
							<ArrowDownIcon className="w-3 h-3 mr-1" />
							Latest
						</Button>
					</motion.div>
				)}
			</AnimatePresence>

			{/* Input Area */}
			<div className="shrink-0 p-4 border-t border-border/30 bg-background/50">
				{/* Attached images preview */}
				{attachedImages.length > 0 && (
					<div className="flex flex-wrap gap-2 mb-2">
						{attachedImages.map((img, index) => (
							<div
								key={`img-${index}`}
								className="relative w-12 h-12 rounded-lg overflow-hidden border border-border/50 group"
							>
								<img
									src={img.preview}
									alt={`Attached ${index + 1}`}
									className="w-full h-full object-cover"
								/>
								<button
									type="button"
									onClick={() => handleRemoveImage(index)}
									className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center"
								>
									<XIcon className="w-4 h-4 text-white" />
								</button>
							</div>
						))}
					</div>
				)}
				<div className="relative flex items-start gap-2">
					<input
						type="file"
						ref={imageInputRef}
						accept="image/*"
						multiple
						onChange={handleImageSelect}
						className="hidden"
					/>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								className="h-9 w-9 shrink-0 rounded-lg hover:bg-accent/50 mt-0.5"
								onClick={() => imageInputRef.current?.click()}
								disabled={loading}
							>
								<ImageIcon className="w-4 h-4" />
							</Button>
						</TooltipTrigger>
						<TooltipContent side="top" className="text-xs">
							Attach image
						</TooltipContent>
					</Tooltip>
					<textarea
						value={input}
						onChange={(e) => setInput(e.target.value)}
						onKeyDown={handleKeyDown}
						onPaste={handlePaste}
						placeholder={
							runContext ? "Ask about the logs..." : "Ask anything..."
						}
						className="flex-1 min-h-9 max-h-[120px] text-sm py-2 px-3 pr-11 rounded-xl bg-background/80 border border-border/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/20 focus-visible:border-primary/50 resize-none"
						disabled={loading}
						rows={1}
					/>
					<Button
						size="icon"
						onClick={handleSubmit}
						disabled={loading || (!input.trim() && attachedImages.length === 0)}
						className="absolute right-1 top-0.5 h-8 w-8 rounded-lg shadow-md bg-linear-to-br from-primary to-purple-600 hover:shadow-lg hover:shadow-primary/20 transition-all duration-200 disabled:opacity-50"
					>
						{loading ? (
							<Loader2Icon className="w-4 h-4 animate-spin" />
						) : (
							<SendIcon className="w-4 h-4" />
						)}
					</Button>
				</div>
			</div>
		</motion.div>
	);
});

// Floating panel view component - memoized to prevent re-renders
interface FloatingPanelViewProps extends EmbeddedViewProps {
	autocompleteEnabled: boolean;
	setAutocompleteEnabled: (value: boolean) => void;
	onClose: () => void;
	mode: "chat" | "autocomplete" | "embedded";
}

const FloatingPanelView = memo(function FloatingPanelView({
	messages,
	input,
	setInput,
	loading,
	loadingPhase,
	elapsedSeconds,
	suggestions,
	pendingCommands,
	planSteps,
	currentToolCall,
	userScrolledUp,
	autocompleteEnabled,
	setAutocompleteEnabled,
	scrollContainerRef,
	messagesEndRef,
	handleScroll,
	scrollToBottom,
	handleNewChat,
	handleSubmit,
	handleExecuteCommands,
	handleExecuteSingle,
	handleDismissCommands,
	onAcceptSuggestion,
	onFocusNode,
	onClose,
	board,
	mode,
	models,
	selectedModelId,
	setSelectedModelId,
	attachedImages,
	imageInputRef,
	handleImageSelect,
	handleRemoveImage,
	handlePaste,
	handleKeyDown,
}: FloatingPanelViewProps) {
	return (
		<motion.div
			layoutId="flow-copilot"
			initial={{ opacity: 0, y: -20, scale: 0.95 }}
			animate={{ opacity: 1, y: 0, scale: 1 }}
			exit={{ opacity: 0, y: -20, scale: 0.95 }}
			transition={{ type: "spring", stiffness: 400, damping: 30 }}
			className="absolute top-4 right-4 left-4 z-40 md:left-auto md:top-20 md:w-[420px] h-[calc(100dvh-8rem)] md:h-[560px]"
		>
			{/* Subtle pulsating glow */}
			{loading && (
				<motion.div
					className="absolute -inset-1 rounded-[28px] pointer-events-none bg-primary/20"
					style={{ filter: "blur(16px)" }}
					animate={{ opacity: [0.3, 0.6, 0.3], scale: [1, 1.02, 1] }}
					transition={{
						duration: 2,
						repeat: Number.POSITIVE_INFINITY,
						ease: "easeInOut",
					}}
				/>
			)}

			{/* Main container */}
			<div className="absolute inset-0 bg-background/95 backdrop-blur-xl rounded-3xl border border-border/40 flex flex-col overflow-hidden shadow-2xl">
				{/* Enhanced Header */}
				<div className="relative overflow-hidden">
					<div className="absolute inset-0 bg-linear-to-br from-primary/8 via-violet-500/5 to-pink-500/5" />

					{loading && (
						<motion.div
							className="absolute inset-0 opacity-30"
							style={{
								background:
									"radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
							}}
							animate={{
								background: [
									"radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
									"radial-gradient(circle at 70% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 30% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
								],
							}}
							transition={{
								duration: 3,
								repeat: Number.POSITIVE_INFINITY,
								repeatType: "reverse",
							}}
						/>
					)}

					<div className="relative p-4 pb-3">
						<div className="flex justify-between items-start">
							<div className="flex items-center gap-3">
								<div className="relative">
									<motion.div
										className="absolute inset-0 bg-linear-to-br from-primary to-violet-600 rounded-xl blur-lg opacity-50"
										animate={
											loading
												? { scale: [1, 1.2, 1], opacity: [0.5, 0.8, 0.5] }
												: {}
										}
										transition={{
											duration: 2,
											repeat: Number.POSITIVE_INFINITY,
										}}
									/>
									<div className="relative p-2.5 bg-linear-to-br from-primary via-violet-600 to-pink-600 rounded-xl shadow-lg">
										<SparklesIcon className="w-5 h-5 text-white" />
									</div>
								</div>
								<div>
									<h3 className="font-bold text-base tracking-tight">
										FlowPilot
									</h3>
									<div className="flex items-center gap-2">
										{loading ? (
											<StatusPill
												phase={loadingPhase}
												elapsed={elapsedSeconds}
											/>
										) : (
											<div className="flex items-center gap-1.5 text-xs text-muted-foreground">
												<span className="relative flex h-2 w-2">
													<span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
													<span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
												</span>
												Ready to help
											</div>
										)}
									</div>
								</div>
							</div>

							<div className="flex items-center gap-1">
								<Tooltip>
									<TooltipTrigger asChild>
										<Button
											variant={autocompleteEnabled ? "default" : "ghost"}
											size="icon"
											className={`h-8 w-8 rounded-lg transition-all duration-200 ${
												autocompleteEnabled
													? "bg-primary/90 hover:bg-primary shadow-md shadow-primary/20"
													: "hover:bg-accent/50"
											}`}
											onClick={() =>
												setAutocompleteEnabled(!autocompleteEnabled)
											}
										>
											<Wand2Icon
												className={`w-4 h-4 ${autocompleteEnabled ? "text-primary-foreground" : ""}`}
											/>
										</Button>
									</TooltipTrigger>
									<TooltipContent side="bottom" className="text-xs">
										{autocompleteEnabled ? "Disable" : "Enable"} autocomplete
									</TooltipContent>
								</Tooltip>

								<Tooltip>
									<TooltipTrigger asChild>
										<Button
											variant="ghost"
											size="icon"
											className="h-8 w-8 rounded-lg hover:bg-accent/50"
											onClick={handleNewChat}
										>
											<SquarePenIcon className="w-4 h-4" />
										</Button>
									</TooltipTrigger>
									<TooltipContent side="bottom" className="text-xs">
										New chat
									</TooltipContent>
								</Tooltip>

								<Tooltip>
									<TooltipTrigger asChild>
										<Button
											variant="ghost"
											size="icon"
											className="h-8 w-8 rounded-lg hover:bg-destructive/10 hover:text-destructive"
											onClick={onClose}
										>
											<XIcon className="w-4 h-4" />
										</Button>
									</TooltipTrigger>
									<TooltipContent side="bottom" className="text-xs">
										Close
									</TooltipContent>
								</Tooltip>
							</div>
						</div>
					</div>

					{/* Model selector row */}
					<div className="relative px-4 pb-3">
						<Select value={selectedModelId} onValueChange={setSelectedModelId}>
							<SelectTrigger className="h-9 text-xs bg-background/60 backdrop-blur-sm border-border/30 hover:border-primary/30 transition-all duration-200 rounded-xl focus:ring-2 focus:ring-primary/20">
								<div className="flex items-center gap-2">
									<BotIcon className="w-3.5 h-3.5 text-muted-foreground" />
									<SelectValue placeholder="Select Model" />
								</div>
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

					{loading && (
						<motion.div
							className="absolute bottom-0 left-0 right-0 h-0.5 bg-muted/30"
							initial={{ opacity: 0 }}
							animate={{ opacity: 1 }}
						>
							<motion.div
								className="h-full bg-linear-to-r from-primary via-violet-500 to-pink-500"
								initial={{ width: "0%" }}
								animate={{ width: "100%" }}
								transition={{ duration: 30, ease: "linear" }}
							/>
						</motion.div>
					)}
				</div>

				{/* Chat Area */}
				<ScrollArea
					className="flex-1 min-h-0 p-4 flex flex-col"
					viewportRef={scrollContainerRef}
					onScroll={handleScroll}
				>
					<div className="space-y-4 min-w-0">
						<MessagesArea
							messages={messages}
							loading={loading}
							planSteps={planSteps}
							currentToolCall={currentToolCall}
							pendingCommands={pendingCommands}
							suggestions={suggestions}
							onFocusNode={onFocusNode}
							onAcceptSuggestion={onAcceptSuggestion}
							handleExecuteCommands={handleExecuteCommands}
							handleExecuteSingle={handleExecuteSingle}
							handleDismissCommands={handleDismissCommands}
							board={board}
							setInput={setInput}
							embedded={false}
						/>
						<div ref={messagesEndRef} />
					</div>
				</ScrollArea>

				{/* Scroll to bottom indicator */}
				<AnimatePresence>
					{userScrolledUp && messages.length > 0 && (
						<motion.div
							initial={{ opacity: 0, y: 10 }}
							animate={{ opacity: 1, y: 0 }}
							exit={{ opacity: 0, y: 10 }}
							className="absolute bottom-20 left-1/2 -translate-x-1/2 z-10"
						>
							<Button
								size="sm"
								variant="secondary"
								className="rounded-full shadow-lg border border-border/50 gap-1.5 px-3 h-8 text-xs"
								onClick={() => scrollToBottom(true)}
							>
								<ArrowDownIcon className="w-3.5 h-3.5" />
								New messages
							</Button>
						</motion.div>
					)}
				</AnimatePresence>

				{/* Input Area */}
				<div className="p-4 bg-background/80 backdrop-blur-sm border-t border-border/50">
					{/* Image previews */}
					{attachedImages.length > 0 && (
						<div className="flex gap-2 mb-3 flex-wrap">
							{attachedImages.map((img, idx) => (
								<div key={idx} className="relative group">
									<img
										src={img.preview}
										alt={`Attached ${idx + 1}`}
										className="h-16 w-16 object-cover rounded-lg border border-border/50"
									/>
									<button
										type="button"
										onClick={() => handleRemoveImage(idx)}
										className="absolute -top-1.5 -right-1.5 w-5 h-5 bg-destructive text-destructive-foreground rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity shadow-md"
									>
										<XIcon className="w-3 h-3" />
									</button>
								</div>
							))}
						</div>
					)}
					<div className="relative flex items-center gap-2">
						<input
							type="file"
							ref={imageInputRef}
							accept="image/*"
							multiple
							onChange={handleImageSelect}
							className="hidden"
						/>
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									type="button"
									variant="ghost"
									size="icon"
									className="h-9 w-9 shrink-0 rounded-lg hover:bg-accent/50"
									onClick={() => imageInputRef.current?.click()}
									disabled={loading}
								>
									<ImageIcon className="w-4 h-4" />
								</Button>
							</TooltipTrigger>
							<TooltipContent side="top" className="text-xs">
								Attach image (for vision models)
							</TooltipContent>
						</Tooltip>
						<textarea
							value={input}
							onChange={(e) => setInput(e.target.value)}
							onPaste={handlePaste}
							onKeyDown={handleKeyDown}
							placeholder={
								mode === "chat"
									? "Describe what you want to change..."
									: "What node should come next?"
							}
							rows={1}
							className="flex-1 pr-11 rounded-xl border border-border/50 focus-visible:ring-2 focus-visible:ring-primary/20 focus-visible:border-primary/50 bg-background/80 backdrop-blur-sm transition-all duration-200 resize-none min-h-9 max-h-32 py-2 px-3 text-sm"
						/>
						<Button
							size="icon"
							onClick={handleSubmit}
							disabled={
								loading || (!input.trim() && attachedImages.length === 0)
							}
							className="absolute right-0 bottom-0 h-9 w-9 rounded-lg shadow-md bg-linear-to-br from-primary to-purple-600 hover:shadow-lg hover:shadow-primary/20 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							<SendIcon className="w-4 h-4" />
						</Button>
					</div>
				</div>
			</div>
		</motion.div>
	);
});

// Messages area component - memoized to prevent re-renders during streaming
interface MessagesAreaProps {
	messages: Message[];
	loading: boolean;
	planSteps: PlanStep[];
	currentToolCall: string | null;
	pendingCommands: BoardCommand[];
	suggestions: Suggestion[];
	onFocusNode?: (nodeId: string) => void;
	onAcceptSuggestion: (suggestion: Suggestion) => void;
	handleExecuteCommands: () => void;
	handleExecuteSingle: (index: number) => void;
	handleDismissCommands: () => void;
	board: any;
	setInput: (value: string) => void;
	embedded: boolean;
}

const MessagesArea = memo(function MessagesArea({
	messages,
	loading,
	planSteps,
	currentToolCall,
	pendingCommands,
	suggestions,
	onFocusNode,
	onAcceptSuggestion,
	handleExecuteCommands,
	handleExecuteSingle,
	handleDismissCommands,
	board,
	setInput,
	embedded,
}: MessagesAreaProps) {
	// Memoize computed styles to prevent recalculation on every render
	const styles = useMemo(
		() => ({
			textSize: embedded ? "text-[10px]" : "text-sm",
			smallTextSize: embedded ? "text-[10px]" : "text-xs",
			iconSize: embedded ? "w-2.5 h-2.5" : "w-3 h-3",
			padding: embedded ? "p-2.5" : "p-3.5",
			borderRadius: embedded ? "rounded-xl" : "rounded-2xl",
		}),
		[embedded],
	);

	const { textSize, smallTextSize, iconSize, padding, borderRadius } = styles;

	if (messages.length === 0) {
		return (
			<motion.div
				initial={{ opacity: 0, y: 10 }}
				animate={{ opacity: 1, y: 0 }}
				className={`flex flex-col items-center justify-center ${embedded ? "py-6" : "py-12"} text-center`}
			>
				<div className="relative inline-block mb-3">
					<div className="absolute inset-0 bg-linear-to-br from-primary/30 via-violet-500/20 to-pink-500/30 blur-2xl rounded-full scale-150" />
					<motion.div
						animate={{ rotate: [0, 5, -5, 0] }}
						transition={{
							duration: 4,
							repeat: Number.POSITIVE_INFINITY,
							ease: "easeInOut",
						}}
					>
						<SparklesIcon
							className={`${embedded ? "w-10 h-10" : "w-14 h-14"} relative text-primary/50`}
						/>
					</motion.div>
				</div>
				<p className={`${textSize} font-medium text-foreground mb-1`}>
					How can I help?
				</p>
				<p className={`${smallTextSize} text-muted-foreground max-w-[180px]`}>
					{embedded
						? "Ask about errors, trace issues, or get insights from your logs"
						: "Describe what you want to build or modify in your flow"}
				</p>
				<div className="flex flex-wrap gap-1.5 justify-center pt-3">
					{(embedded
						? ["Find errors", "Trace execution", "Explain logs"]
						: ["Add a node", "Connect components", "Explain my flow"]
					).map((suggestion, i) => (
						<motion.button
							key={suggestion}
							initial={{ opacity: 0, y: 5 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ delay: 0.2 + i * 0.1 }}
							onClick={() => setInput(suggestion)}
							className={`px-2 py-1 ${smallTextSize} font-medium text-muted-foreground bg-muted/50 hover:bg-muted border border-border/50 hover:border-primary/30 rounded-full transition-all duration-200 hover:text-foreground`}
						>
							{suggestion}
						</motion.button>
					))}
				</div>
			</motion.div>
		);
	}

	return (
		<>
			{messages.map((m, i) => (
				<motion.div
					initial={{ opacity: 0, y: 10 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ delay: i === messages.length - 1 ? 0.1 : 0 }}
					key={i}
					className={`flex min-w-0 ${m.role === "user" ? "justify-end" : "justify-start"}`}
				>
					<div
						className={`${padding} ${borderRadius} ${textSize} wrap-break-word overflow-hidden min-w-0 ${
							m.role === "user"
								? `bg-muted/60 text-foreground ${embedded ? "rounded-br-sm" : "rounded-br-md"} max-w-[85%] border border-border/40`
								: `bg-muted/40 backdrop-blur-sm ${embedded ? "rounded-bl-sm" : "rounded-bl-md"} max-w-full border border-border/30`
						}`}
					>
						{m.role === "assistant" && m.agentType && (
							<div
								className={`flex items-center gap-1.5 mb-2 pb-1.5 border-b border-border/30`}
							>
								<div
									className={`p-0.5 rounded ${m.agentType === "Explain" ? "bg-blue-500/15" : "bg-amber-500/15"}`}
								>
									{m.agentType === "Explain" ? (
										<BrainCircuitIcon className={iconSize + " text-blue-500"} />
									) : (
										<EditIcon className={iconSize + " text-amber-500"} />
									)}
								</div>
								<span
									className={`${smallTextSize} font-medium ${m.agentType === "Explain" ? "text-blue-500" : "text-amber-500"}`}
								>
									{m.agentType === "Explain" ? "Explain" : "Edit"}
									{!embedded && " Mode"}
								</span>
							</div>
						)}
						{/* Show attached images for user messages */}
						{m.role === "user" && m.images && m.images.length > 0 && (
							<div className="flex gap-1.5 mb-2 flex-wrap">
								{m.images.map((img, idx) => (
									<img
										key={idx}
										src={img.preview}
										alt={`Attached ${idx + 1}`}
										className={`${embedded ? "h-16 w-16" : "h-20 w-20"} object-cover rounded-md border border-border/30`}
									/>
								))}
							</div>
						)}
						{m.content ? (
							<>
								<MessageContent
									content={m.content}
									onFocusNode={onFocusNode}
									board={board}
								/>
								{m.planSteps && m.planSteps.length > 0 && (
									<Collapsible className="mt-2 pt-2 border-t border-border/30">
										<CollapsibleTrigger
											className={`flex items-center gap-1 ${smallTextSize} text-muted-foreground hover:text-foreground transition-colors w-full`}
										>
											<ChevronDown
												className={`${iconSize} transition-transform duration-200 group-data-[state=open]:rotate-180`}
											/>
											<ListChecksIcon className={iconSize} />
											<span>{m.planSteps.length} steps completed</span>
										</CollapsibleTrigger>
										<CollapsibleContent>
											<div className="mt-1.5 space-y-0.5">
												{m.planSteps.map((step, idx) => (
													<div
														key={idx}
														className={`flex items-center gap-1.5 ${smallTextSize} text-muted-foreground`}
													>
														<CheckCircle2Icon
															className={`${iconSize} text-green-500 shrink-0`}
														/>
														<span className="truncate">{step.description}</span>
													</div>
												))}
											</div>
										</CollapsibleContent>
									</Collapsible>
								)}
							</>
						) : (
							<div className="space-y-2">
								{planSteps.some(
									(s) => s.tool_name === "think" && s.status === "InProgress",
								) ? (
									<div className="space-y-1.5">
										<div className="flex items-center gap-1.5">
											<BrainCircuitIcon
												className={`${iconSize} text-violet-500 animate-pulse`}
											/>
											<span
												className={`${smallTextSize} font-medium text-violet-500`}
											>
												Reasoning...
											</span>
										</div>
										<div
											className={`${smallTextSize} text-muted-foreground font-mono bg-violet-500/5 rounded-md p-2 max-h-20 overflow-y-auto border border-violet-500/20`}
										>
											{
												planSteps.find((s) => s.tool_name === "think")
													?.description
											}
											<span className="inline-block w-1 h-2.5 ml-0.5 bg-violet-500/60 animate-pulse rounded-sm" />
										</div>
									</div>
								) : (
									<div className="flex items-center gap-2">
										<div className="flex gap-0.5">
											{[0, 1, 2].map((j) => (
												<motion.span
													key={j}
													className="w-1 h-1 bg-primary rounded-full"
													animate={{ opacity: [0.3, 1, 0.3] }}
													transition={{
														duration: 1,
														repeat: Number.POSITIVE_INFINITY,
														delay: j * 0.2,
													}}
												/>
											))}
										</div>
										<span className={`${smallTextSize} text-muted-foreground`}>
											{currentToolCall
												? `Using ${currentToolCall.replace(/_/g, " ")}...`
												: "Processing..."}
										</span>
									</div>
								)}
								{planSteps.length > 0 &&
									!planSteps.some(
										(s) => s.tool_name === "think" && s.status === "InProgress",
									) && <PlanStepsView steps={planSteps} />}
							</div>
						)}
						{m.executedCommands && m.executedCommands.length > 0 && (
							<motion.div
								initial={{ opacity: 0, height: 0 }}
								animate={{ opacity: 1, height: "auto" }}
								className="mt-2 pt-2 border-t border-border/30"
							>
								<div
									className={`flex items-center gap-1 mb-1.5 ${smallTextSize} text-muted-foreground`}
								>
									<div className="p-0.5 bg-green-500/20 rounded">
										<CheckCircle2Icon
											className={`${iconSize} text-green-500`}
										/>
									</div>
									<span className="font-medium">
										Applied {m.executedCommands.length} change
										{m.executedCommands.length > 1 ? "s" : ""}
									</span>
								</div>
								<div className="space-y-0.5">
									{m.executedCommands.map((cmd, cmdIndex) => (
										<div
											key={cmdIndex}
											className={`${smallTextSize} bg-green-500/10 text-green-700 dark:text-green-400 px-1.5 py-0.5 rounded flex items-center gap-1 max-w-full overflow-hidden text-ellipsis whitespace-nowrap`}
											title={cmd.summary || cmd.command_type}
										>
											<span className="shrink-0">
												{getCommandIcon(cmd, iconSize)}
											</span>
											<span className="truncate">
												{cmd.summary || cmd.command_type}
											</span>
										</div>
									))}
								</div>
							</motion.div>
						)}
					</div>
				</motion.div>
			))}

			{pendingCommands.length > 0 && (
				<PendingCommandsView
					commands={pendingCommands}
					onExecute={handleExecuteCommands}
					onExecuteSingle={handleExecuteSingle}
					onDismiss={handleDismissCommands}
				/>
			)}

			{suggestions.length > 0 && (
				<motion.div
					initial={{ opacity: 0, y: 10 }}
					animate={{ opacity: 1, y: 0 }}
					className={`space-y-2 pl-2 border-l-2 border-primary/30 ml-1`}
				>
					<p
						className={`${smallTextSize} font-semibold text-foreground/70 mb-1.5 flex items-center gap-1`}
					>
						<Wand2Icon className={`${iconSize} text-primary`} />
						Suggestions
					</p>
					{suggestions.map((s, i) => (
						<div
							key={i}
							className={`group border border-border/50 bg-card/80 backdrop-blur-sm hover:bg-accent/30 hover:border-primary/40 ${padding} rounded-lg cursor-pointer transition-all duration-200 hover:shadow-md hover:shadow-primary/5`}
							onClick={() => onAcceptSuggestion(s)}
						>
							<div className="flex items-center justify-between mb-1">
								<span className={`font-semibold ${smallTextSize} text-primary`}>
									{s.node_type}
								</span>
								<Wand2Icon
									className={`${iconSize} opacity-0 group-hover:opacity-100 transition-opacity text-primary`}
								/>
							</div>
							<div
								className={`${smallTextSize} text-muted-foreground leading-relaxed`}
							>
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
						<div
							className={`bg-muted/60 backdrop-blur-sm border border-border/30 ${borderRadius} ${embedded ? "rounded-bl-sm" : "rounded-bl-sm"} px-3 py-2 flex items-center gap-2`}
						>
							<div className="flex gap-0.5">
								<span
									className={`${embedded ? "w-1.5 h-1.5" : "w-2 h-2"} bg-primary rounded-full animate-bounce`}
									style={{ animationDelay: "0ms" }}
								/>
								<span
									className={`${embedded ? "w-1.5 h-1.5" : "w-2 h-2"} bg-primary rounded-full animate-bounce`}
									style={{ animationDelay: "150ms" }}
								/>
								<span
									className={`${embedded ? "w-1.5 h-1.5" : "w-2 h-2"} bg-primary rounded-full animate-bounce`}
									style={{ animationDelay: "300ms" }}
								/>
							</div>
							<span
								className={`${smallTextSize} text-muted-foreground font-medium`}
							>
								{currentToolCall ? (
									<span className="flex items-center gap-1">
										<WrenchIcon className={`${iconSize} animate-spin`} />
										Using {currentToolCall.replace(/_/g, " ")}...
									</span>
								) : (
									"Thinking..."
								)}
							</span>
						</div>
					</motion.div>
				)}
		</>
	);
});
