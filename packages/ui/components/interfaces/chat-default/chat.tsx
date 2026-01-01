"use client";

import { useTheme } from "next-themes";
import {
	forwardRef,
	memo,
	useCallback,
	useEffect,
	useImperativeHandle,
	useRef,
	useState,
} from "react";
import PuffLoader from "react-spinners/PuffLoader";
import type { IEventPayloadChat } from "../../../lib";
import type { IMessage } from "./chat-db";
import { ChatBox, type ChatBoxRef, type ISendMessageFunction } from "./chatbox";
import { MessageComponent } from "./message";

function getMessageTextContent(message: IMessage): string {
	const content = message.inner.content;
	if (typeof content === "string") return content;
	const textContent = content.find((c) => c.type === "text");
	return textContent?.text ?? "";
}

export interface IChatProps {
	messages: IMessage[];
	onSendMessage: ISendMessageFunction;
	onMessageUpdate?: (
		messageId: string,
		updates: Partial<IMessage>,
	) => void | Promise<void>;
	config?: Partial<IEventPayloadChat>;
	sessionId?: string;
}

export interface IChatRef {
	pushCurrentMessageUpdate: (message: IMessage) => void;
	clearCurrentMessageUpdate: () => void;
	pushMessage: (message: IMessage) => void;
	sendMessage: ISendMessageFunction;
	scrollToBottom: () => void;
	clearMessages: () => void;
	focusInput: () => void;
}

const ChatInner = forwardRef<IChatRef, IChatProps>(
	(
		{ messages, onSendMessage, onMessageUpdate, config = {}, sessionId },
		ref,
	) => {
		const { resolvedTheme } = useTheme();
		const messagesEndRef = useRef<HTMLDivElement>(null);
		const scrollContainerRef = useRef<HTMLDivElement>(null);
		const [shouldAutoScroll, setShouldAutoScroll] = useState(true);
		const [currentMessage, setCurrentMessage] = useState<IMessage | null>(null);
		const [localMessages, setLocalMessages] = useState<IMessage[]>(messages);
		const [hasInitiallyScrolled, setHasInitiallyScrolled] = useState(false);
		const chatBox = useRef<ChatBoxRef>(null);
		const isScrollingProgrammatically = useRef(false);
		const [defaultActiveTools, setDefaultActiveTools] = useState<string[]>();
		const [isSending, setIsSending] = useState(false);
		const [sendingContent, setSendingContent] = useState("");

		// Sync external messages with local state
		useEffect(() => {
			setLocalMessages(messages);

			// Clear optimistic sending state when the user message appears in DB
			if (isSending) {
				const lastMessage = messages[messages.length - 1];
				if (lastMessage?.inner.role === "user") {
					setIsSending(false);
					setSendingContent("");
				}
			}

			const lastUserMessage = messages
				.slice()
				.reverse()
				.find((msg) => msg.inner.role === "user");

			if (lastUserMessage) {
				const availableTools = config?.tools ?? [];
				const lastActiveTools = lastUserMessage.tools ?? [];
				const newActiveTools = lastActiveTools.filter((tool) =>
					availableTools.includes(tool),
				);

				setDefaultActiveTools(newActiveTools);
				return;
			}

			setDefaultActiveTools(config?.default_tools ?? []);
		}, [messages, config?.tools, isSending]);

		// Initial scroll to bottom when messages first load
		useEffect(() => {
			if (localMessages.length > 0 && !hasInitiallyScrolled) {
				setTimeout(() => {
					scrollToBottom();
					setHasInitiallyScrolled(true);
				}, 100);
			}
		}, [localMessages.length, hasInitiallyScrolled]);

		const scrollToBottom = useCallback(() => {
			if (!messagesEndRef.current) return;
			if (!shouldAutoScroll) return;
			isScrollingProgrammatically.current = true;
			messagesEndRef.current.scrollIntoView({
				behavior: "instant",
				block: "end",
			});
			// Reset the flag after scroll animation completes
			setTimeout(() => {
				isScrollingProgrammatically.current = false;
			}, 500);
		}, [shouldAutoScroll]);

		const isAtBottom = useCallback(() => {
			if (!scrollContainerRef.current) return false;
			const { scrollTop, scrollHeight, clientHeight } =
				scrollContainerRef.current;
			const threshold = 100; // Larger threshold for better detection
			return Math.abs(scrollHeight - scrollTop - clientHeight) < threshold;
		}, []);

		const handleScroll = useCallback(() => {
			// Don't update auto-scroll state if we're programmatically scrolling
			const atBottom = isAtBottom();
			if (isScrollingProgrammatically.current) {
				console.log("Ignoring scroll - programmatic");
				if (!atBottom) {
					setShouldAutoScroll(false);
				}
				return;
			}

			console.log("Scroll event detected, at bottom:", atBottom);

			setShouldAutoScroll(atBottom);
		}, [isAtBottom]);

		// Auto-scroll when new messages arrive or current message updates, but only if should auto-scroll
		useEffect(() => {
			if (shouldAutoScroll && hasInitiallyScrolled) {
				scrollToBottom();
			}
		}, [
			localMessages,
			currentMessage,
			shouldAutoScroll,
			hasInitiallyScrolled,
			scrollToBottom,
		]);

		// When user sends a message, always scroll to bottom and enable auto-scroll
		const handleSendMessage = useCallback(
			async (
				content: string,
				filesAttached?: File[],
				activeTools?: string[],
				audioFile?: File,
			) => {
				setShouldAutoScroll(true);
				setIsSending(true);
				setSendingContent(content);

				// Scroll immediately to show the optimistic message
				setTimeout(() => {
					scrollToBottom();
				}, 50);

				try {
					await onSendMessage(content, filesAttached, activeTools, audioFile);
				} finally {
					setIsSending(false);
					setSendingContent("");
				}
				// Scroll after a brief delay to ensure the message is rendered
				setTimeout(() => {
					scrollToBottom();
				}, 50);
			},
			[onSendMessage, scrollToBottom],
		);

		// iOS keyboard/open focus handling to reduce layout jump and zoom
		useEffect(() => {
			const onFocusIn = (e: FocusEvent) => {
				const target = e.target as HTMLElement | null;
				if (!target) return;
				// Ensure the input stays visible when keyboard opens
				if (target.tagName === "TEXTAREA" || target.tagName === "INPUT") {
					setTimeout(() => {
						try {
							messagesEndRef.current?.scrollIntoView({
								block: "end",
								behavior: "smooth",
							});
						} catch {}
					}, 100);
				}
			};
			document.addEventListener("focusin", onFocusIn);
			return () => document.removeEventListener("focusin", onFocusIn);
		}, []);

		// Dismiss keyboard when tapping outside inputs on iOS
		useEffect(() => {
			const onTouchStart = (e: TouchEvent) => {
				const active = document.activeElement as HTMLElement | null;
				if (!active) return;
				const tag = active.tagName;
				if (tag === "INPUT" || tag === "TEXTAREA") {
					const target = e.target as Node | null;
					if (target && active && !active.contains(target)) {
						setTimeout(() => {
							try {
								active.blur();
							} catch {}
						}, 50);
					}
				}
			};
			document.addEventListener("touchstart", onTouchStart, {
				passive: true,
				capture: true,
			} as AddEventListenerOptions);
			return () =>
				document.removeEventListener("touchstart", onTouchStart, true as any);
		}, []);

		// Expose methods via ref
		useImperativeHandle(
			ref,
			() => ({
				pushCurrentMessageUpdate: (message: IMessage) => {
					setCurrentMessage(message);
				},
				clearCurrentMessageUpdate: () => {
					setCurrentMessage(null);
				},
				pushMessage: (message: IMessage) => {
					setLocalMessages((prev) => [...prev, message]);
				},
				sendMessage: handleSendMessage,
				scrollToBottom,
				clearMessages: () => {
					setLocalMessages([]);
					setHasInitiallyScrolled(false);
					setShouldAutoScroll(true);
				},
				focusInput: () => {
					chatBox.current?.focusInput?.();
				},
			}),
			[handleSendMessage, scrollToBottom, shouldAutoScroll],
		);

		return (
			<main
				className="flex flex-col h-dvh w-full items-center flex-grow bg-background max-h-dvh overflow-hidden"
				style={{
					WebkitOverflowScrolling: "touch",
					touchAction: "manipulation",
				}}
			>
				<div className="h-full flex-grow flex flex-col bg-background max-h-dvh w-full overflow-hidden">
					{/* Messages Container */}
					<div
						ref={scrollContainerRef}
						onScroll={handleScroll}
						className="flex-1 overflow-y-auto overscroll-contain p-4 pb-[max(theme(spacing.4),env(safe-area-inset-bottom))] space-y-8 flex flex-col items-center flex-grow max-h-full"
						style={{ WebkitOverflowScrolling: "touch" }}
					>
						{localMessages.map((message) => (
							<div className="w-full max-w-screen-lg px-4" key={message.id}>
								<MessageComponent
									message={message}
									onMessageUpdate={onMessageUpdate}
								/>
							</div>
						))}
						{isSending &&
							!localMessages.some(
								(m) =>
									m.inner.role === "user" &&
									getMessageTextContent(m) === sendingContent,
							) && (
								<div className="w-full max-w-screen-lg px-4 flex flex-col items-end space-y-1 animate-in fade-in slide-in-from-bottom-2 duration-200">
									<div className="bg-muted dark:bg-muted/30 text-foreground px-4 py-2 rounded-xl rounded-tr-sm max-w-3xl shadow-sm">
										<p className="whitespace-pre-wrap text-sm">
											{sendingContent}
										</p>
									</div>
									<div className="flex items-center gap-2 pr-1">
										<PuffLoader
											size={16}
											color={resolvedTheme === "dark" ? "white" : "black"}
										/>
										<span className="text-xs text-muted-foreground">
											Processing...
										</span>
									</div>
								</div>
							)}
						{currentMessage &&
							!localMessages.some((m) => m.id === currentMessage.id) && (
								<div
									className="w-full max-w-screen-lg px-4 relative"
									key={currentMessage.id}
								>
									<PuffLoader
										color={resolvedTheme === "dark" ? "white" : "black"}
										className="mt-2 absolute left-0 top-0 translate-y-[2.5rem] translate-x-[-100%]"
										size={30}
									/>
									<MessageComponent loading message={currentMessage} />
								</div>
							)}
						<div ref={messagesEndRef} />
					</div>

					{/* ChatBox */}
					<div className="bg-transparent pb-[max(theme(spacing.4),env(safe-area-inset-bottom))] max-w-screen-lg w-full mx-auto">
						{defaultActiveTools && (
							<ChatBox
								ref={chatBox}
								availableTools={config?.tools ?? []}
								defaultActiveTools={defaultActiveTools}
								onSendMessage={handleSendMessage}
								fileUpload={config?.allow_file_upload ?? false}
								audioInput={config?.allow_voice_input ?? true}
							/>
						)}
					</div>
				</div>
			</main>
		);
	},
);

export const Chat = memo(ChatInner);
Chat.displayName = "Chat";
