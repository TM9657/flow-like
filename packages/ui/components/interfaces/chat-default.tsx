"use client";

import { createId } from "@paralleldrive/cuid2";
import { useLiveQuery } from "dexie-react-hooks";
import { HistoryIcon, SquarePenIcon } from "lucide-react";
import { usePathname, useSearchParams } from "next/navigation";
import {
	type RefObject,
	memo,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { toast } from "sonner";
import {
	type IContent,
	IContentType,
	type IHistoryMessage,
	IRole,
	Response,
} from "../../lib";
import { useSetQueryParams } from "../../lib/set-query-params";
import { parseUint8ArrayToJson } from "../../lib/uint8";
import { useBackend } from "../../state/backend-state";
import { useExecutionEngine } from "../../state/execution-engine-context";
import { Button, HoverCard, HoverCardContent, HoverCardTrigger } from "../ui";
import { fileToAttachment } from "./chat-default/attachment";
import { Chat, type IChatRef } from "./chat-default/chat";
import {
	type IAttachment,
	type IMessage,
	chatDb,
} from "./chat-default/chat-db";
import type { ISendMessageFunction } from "./chat-default/chatbox";
import { processChatEvents } from "./chat-default/event-processor";
import { ChatHistory } from "./chat-default/history";
import { ChatWelcome } from "./chat-default/welcome";
import type { IUseInterfaceProps } from "./interfaces";

async function prepareAttachments(
	filesAttached: File[] | undefined,
	audioFile: File | undefined,
	backend: any,
	isOffline: boolean,
) {
	const imageFiles =
		filesAttached?.filter((file) => file.type.startsWith("image/")) ?? [];
	const otherFiles =
		filesAttached?.filter((file) => !file.type.startsWith("image/")) ?? [];
	const imageAttachments = await fileToAttachment(
		imageFiles ?? [],
		backend,
		isOffline,
	);
	const otherAttachments = await fileToAttachment(
		otherFiles ?? [],
		backend,
		isOffline,
	);
	if (audioFile) {
		otherAttachments.push(
			...(await fileToAttachment([audioFile], backend, isOffline)),
		);
	}
	return { imageAttachments, otherAttachments };
}

function createHistoryMessage(
	content: string,
	imageAttachments: IAttachment[],
) {
	const historyMessage: IHistoryMessage = {
		content: [
			{
				type: IContentType.Text,
				text: content,
			},
		],
		role: IRole.User,
	};

	for (const image of imageAttachments) {
		const url = typeof image === "string" ? image : image.url;
		(historyMessage.content as IContent[]).push({
			type: IContentType.IImageURL,
			image_url: {
				url: url,
			},
		});
	}
	return historyMessage;
}

async function updateSession(
	sessionId: string,
	appId: string,
	content: string,
) {
	const sessionExists = await chatDb.sessions
		.where("id")
		.equals(sessionId)
		.count();

	if (sessionExists <= 0) {
		await chatDb.sessions.add({
			id: sessionId,
			appId,
			summarization: content,
			createdAt: Date.now(),
			updatedAt: Date.now(),
		});
	} else {
		await chatDb.sessions.update(sessionId, {
			updatedAt: Date.now(),
		});
	}
}

function createUserMessage(
	sessionId: string,
	appId: string,
	otherAttachments: IAttachment[],
	historyMessage: IHistoryMessage,
	activeTools: string[],
): IMessage {
	return {
		id: createId(),
		sessionId: sessionId,
		appId,
		files: otherAttachments,
		inner: historyMessage,
		timestamp: Date.now(),
		tools: activeTools ?? [],
		actions: [],
	};
}

function createPayload(
	userMessage: IMessage,
	lastMessages: IMessage[],
	historyMessage: IHistoryMessage,
	localState: any,
	globalState: any,
	activeTools: string[],
	otherAttachments: IAttachment[],
) {
	return {
		chat_id: userMessage.sessionId,
		messages: [
			...lastMessages.map((msg) => ({
				role: msg.inner.role,
				content:
					typeof msg.inner.content === "string"
						? msg.inner.content
						: msg.inner.content?.map((c) => ({
								type: c.type,
								text: c.text,
								image_url: c.image_url,
							})),
			})),
			historyMessage,
		],
		local_session: localState?.localState ?? {},
		global_session: globalState?.globalState ?? {},
		actions: [],
		tools: activeTools ?? [],
		attachments: otherAttachments,
	};
}

function createResponseMessage(
	sessionId: string,
	appId: string,
	eventName: string,
): IMessage {
	return {
		id: createId(),
		sessionId: sessionId,
		appId,
		files: [],
		inner: {
			role: IRole.Assistant,
			content: "",
		},
		explicit_name: eventName,
		timestamp: Date.now(),
		tools: [],
		actions: [],
	};
}

async function handleStreamCompletion(
	responseMessage: IMessage,
	chatRef: RefObject<IChatRef | null>,
	executionEngine: any,
	streamId: string,
	subscriberId: string,
	tmpLocalState?: any,
	tmpGlobalState?: any,
) {
	if (tmpLocalState) {
		await chatDb.localStage.put(tmpLocalState);
	}

	if (tmpGlobalState) {
		await chatDb.globalState.put(tmpGlobalState);
	}

	await chatDb.messages.put(responseMessage);

	// Wait for the message to appear in the UI before clearing the temporary one
	await new Promise((resolve) => setTimeout(resolve, 200));
	chatRef.current?.clearCurrentMessageUpdate();
	chatRef.current?.scrollToBottom();

	executionEngine.unsubscribeFromEventStream(streamId, subscriberId);
}

export const ChatInterfaceMemoized = memo(function ChatInterface({
	appId,
	event,
	config = {},
	toolbarRef,
	sidebarRef,
}: Readonly<IUseInterfaceProps>) {
	const backend = useBackend();
	const executionEngine = useExecutionEngine();
	const searchParams = useSearchParams();
	const pathname = usePathname();
	const sessionIdParameter = searchParams.get("sessionId") ?? "";
	const setQueryParams = useSetQueryParams();
	const chatRef = useRef<IChatRef>(null);
	const activeSubscriptions = useRef<string[]>([]);
	const [isSendingFromWelcome, setIsSendingFromWelcome] = useState(false);

	useEffect(() => {
		if (!sessionIdParameter || sessionIdParameter === "") {
			const newSessionId = createId();
			setQueryParams("sessionId", newSessionId);
		}
	}, [sessionIdParameter, setQueryParams]);

	// Cleanup active subscriptions on unmount or session change
	useEffect(() => {
		return () => {
			activeSubscriptions.current.forEach((subId) => {
				executionEngine.unsubscribeFromEventStream(sessionIdParameter, subId);
			});
			activeSubscriptions.current = [];
		};
	}, [sessionIdParameter, executionEngine]);

	const messages = useLiveQuery(
		() =>
			chatDb.messages
				.where("sessionId")
				.equals(sessionIdParameter)
				.sortBy("timestamp"),
		[sessionIdParameter],
	);

	const localState = useLiveQuery(
		() =>
			chatDb.localStage
				.where("[sessionId+eventId]")
				.equals([sessionIdParameter, event.id])
				.first(),
		[sessionIdParameter, event.id],
	);

	const globalState = useLiveQuery(
		() =>
			chatDb.globalState
				.where("[appId+eventId]")
				.equals([appId, event.id])
				.first(),
		[appId, event.id],
	);

	const updateSessionId = useCallback(
		(newSessionId: string) => {
			setQueryParams("sessionId", newSessionId);
		},
		[setQueryParams],
	);

	const handleSidebarToggle = useCallback(() => {
		sidebarRef?.current?.toggleOpen();
	}, [sidebarRef]);

	const handleNewChat = useCallback(() => {
		updateSessionId(createId());
	}, [updateSessionId]);

	const handleSessionChange = useCallback(
		(newSessionId: string) => {
			updateSessionId(newSessionId);
			chatRef.current?.scrollToBottom();
		},
		[updateSessionId],
	);

	const toolbarElements = useMemo(
		() => [
			<HoverCard key="chat-history" openDelay={200} closeDelay={100}>
				<HoverCardTrigger asChild>
					<Button
						variant="ghost"
						size="icon"
						className="hover:bg-accent hover:text-accent-foreground transition-colors"
						onClick={handleSidebarToggle}
					>
						<HistoryIcon className="w-4 h-4" />
					</Button>
				</HoverCardTrigger>
				<HoverCardContent
					side="bottom"
					align="center"
					className="w-auto p-2 bg-popover border shadow-lg"
					onClick={() => {
						console.log("Open chat history");
					}}
				>
					<div className="flex items-center gap-2 text-sm font-medium">
						<HistoryIcon className="w-3 h-3" />
						Chat History
					</div>
				</HoverCardContent>
			</HoverCard>,
			<HoverCard key="new-chat" openDelay={200} closeDelay={100}>
				<HoverCardTrigger asChild>
					<Button
						onClick={handleNewChat}
						variant="ghost"
						size="icon"
						className="hover:bg-accent hover:text-accent-foreground transition-colors"
					>
						<SquarePenIcon className="w-4 h-4" />
					</Button>
				</HoverCardTrigger>
				<HoverCardContent
					side="bottom"
					align="center"
					className="w-auto p-2 bg-popover border shadow-lg"
					onClick={handleNewChat}
				>
					<div className="flex items-center gap-2 text-sm font-medium">
						<SquarePenIcon className="w-3 h-3" />
						New Chat
					</div>
				</HoverCardContent>
			</HoverCard>,
		],
		[handleSidebarToggle, handleNewChat],
	);

	const sidebarContent = useMemo(
		() => (
			<ChatHistory
				key={sessionIdParameter}
				appId={appId}
				sessionId={sessionIdParameter}
				onSessionChange={handleSessionChange}
				sidebarRef={sidebarRef}
			/>
		),
		[sessionIdParameter, appId, handleSessionChange, sidebarRef],
	);

	useEffect(() => {
		toolbarRef?.current?.pushToolbarElements(toolbarElements);
		sidebarRef?.current?.pushSidebar(sidebarContent);
	}, [toolbarElements, sidebarContent, toolbarRef, sidebarRef]);

	// Reconnect to active stream when component mounts or session changes
	useEffect(() => {
		if (!sessionIdParameter) return;

		const streamId = sessionIdParameter;

		// Check if there's an active stream for this session
		if (!executionEngine.isStreamActive(streamId)) return;

		const subscriberId = `chat-reconnect-${sessionIdParameter}`;

		const responseMessage: IMessage = {
			id: createId(),
			sessionId: sessionIdParameter,
			appId,
			files: [],
			inner: {
				role: IRole.Assistant,
				content: "",
			},
			explicit_name: event.name,
			timestamp: Date.now(),
			tools: [],
			actions: [],
		};

		let intermediateResponse = Response.default();
		const attachments: Map<string, IAttachment> = new Map();

		executionEngine.subscribeToEventStream(
			streamId,
			subscriberId,
			(events) => {
				const result = processChatEvents(events, {
					intermediateResponse,
					responseMessage,
					attachments,
					tmpLocalState: null,
					tmpGlobalState: null,
					done: false,
					appId,
					eventId: event.id,
					sessionId: sessionIdParameter,
				});

				intermediateResponse = result.intermediateResponse;

				if (result.shouldUpdate) {
					chatRef.current?.pushCurrentMessageUpdate({
						...result.responseMessage,
					});
					chatRef.current?.scrollToBottom();
				}
			},
			async (events) => {
				await handleStreamCompletion(
					responseMessage,
					chatRef,
					executionEngine,
					streamId,
					subscriberId,
				);
			},
		);

		return () => {
			executionEngine.unsubscribeFromEventStream(streamId, subscriberId);
		};
	}, [sessionIdParameter, appId, event.id, event.name, executionEngine]);

		const handleSendMessage: ISendMessageFunction = useCallback(
		async (
			content,
			filesAttached,
			activeTools?: string[],
			audioFile?: File,
		) => {
			const isOffline = await backend.isOffline(appId);
			const history_elements =
				parseUint8ArrayToJson(event.config)?.history_elements ?? 5;

			if (!sessionIdParameter || sessionIdParameter === "") {
				toast.error("Session ID is not set. Please start a new chat.");
				return;
			}

			// Show loading state if sending from welcome screen
			const hasFiles = (filesAttached && filesAttached.length > 0) || audioFile;
			if (hasFiles && (!messages || messages.length === 0)) {
				setIsSendingFromWelcome(true);
			}

			try {
				const { imageAttachments, otherAttachments } = await prepareAttachments(
					filesAttached,
					audioFile,
					backend,
					isOffline,
				);

				const historyMessage = createHistoryMessage(content, imageAttachments);

				const userMessage = createUserMessage(
					sessionIdParameter,
					appId,
					otherAttachments,
					historyMessage,
					activeTools ?? [],
				);

				await updateSession(sessionIdParameter, appId, content);
				await chatDb.messages.add(userMessage);

				const lastMessages = messages?.slice(-history_elements) ?? [];

				const payload = createPayload(
					userMessage,
					lastMessages,
					historyMessage,
					localState,
					globalState,
					activeTools ?? [],
					otherAttachments,
				);

				const responseMessage = createResponseMessage(
					sessionIdParameter,
					appId,
					event.name,
				);

				chatRef.current?.pushCurrentMessageUpdate({ ...responseMessage });
				chatRef.current?.scrollToBottom();

				let intermediateResponse = Response.default();
				let tmpLocalState = localState;
				let tmpGlobalState = globalState;
				let done = false;
				const attachments: Map<string, IAttachment> = new Map();

				const streamId = sessionIdParameter;
				const subscriberId = `chat-${responseMessage.id}`;
				activeSubscriptions.current.push(subscriberId);

				// Start execution first to reset the stream state
				const executionPromise = executionEngine.executeEvent(streamId, {
					appId,
					eventId: event.id,
					payload: {
						id: event.node_id,
						payload: payload,
					},
					streamState: false,
					onExecutionStart: (execution_id: string) => {},
					path: pathname,
					title: event.name || "Chat",
					interfaceType: "chat",
				});

				executionEngine.subscribeToEventStream(
					streamId,
					subscriberId,
					(events) => {
						const result = processChatEvents(events, {
							intermediateResponse,
							responseMessage,
							attachments,
							tmpLocalState,
							tmpGlobalState,
							done,
							appId,
							eventId: event.id,
							sessionId: sessionIdParameter,
						});

						intermediateResponse = result.intermediateResponse;
						tmpLocalState = result.tmpLocalState;
						tmpGlobalState = result.tmpGlobalState;
						done = result.done;

						if (result.shouldUpdate) {
							chatRef.current?.pushCurrentMessageUpdate({
								...result.responseMessage,
							});
							chatRef.current?.scrollToBottom();
						}
					},
					async (events) => {
						try {
							await handleStreamCompletion(
								responseMessage,
								chatRef,
								executionEngine,
								streamId,
								subscriberId,
								tmpLocalState,
								tmpGlobalState,
							);
						} finally {
							activeSubscriptions.current = activeSubscriptions.current.filter(
								(id) => id !== subscriberId,
							);
						}
					},
				);

				await executionPromise;
			} catch (error) {
				console.error("Error sending message:", error);
				toast.error("Failed to send message. Please try again.");
			} finally {
				setIsSendingFromWelcome(false);
			}
		},
		[
			backend,
			executionEngine,
			sessionIdParameter,
			appId,
			event.id,
			event.name,
			event.node_id,
			event.config,
			messages,
			localState,
			globalState,
			pathname,
		],
	);

	const onMessageUpdate = useCallback(
		async (messageId: string, message: Partial<IMessage>) => {
			await chatDb.messages.update(messageId, {
				...message,
			});
		},
		[],
	);

	const showWelcome = useMemo(
		() => !messages || messages?.length === 0,
		[messages],
	);

	if (!messages) {
		return null;
	}

	return (
		<>
			{showWelcome ? (
				<ChatWelcome
					onSendMessage={handleSendMessage}
					event={event}
					config={config}
					isSending={isSendingFromWelcome}
				/>
			) : (
				<Chat
					key={sessionIdParameter}
					ref={chatRef}
					sessionId={sessionIdParameter}
					messages={messages}
					onSendMessage={handleSendMessage}
					onMessageUpdate={onMessageUpdate}
					config={config}
				/>
			)}
		</>
	);
});

export function ChatInterface({
	appId,
	event,
	config = {},
	toolbarRef,
	sidebarRef,
}: Readonly<IUseInterfaceProps>) {
	return (
		<ChatInterfaceMemoized
			appId={appId}
			event={event}
			config={config}
			toolbarRef={toolbarRef}
			sidebarRef={sidebarRef}
		/>
	);
}
