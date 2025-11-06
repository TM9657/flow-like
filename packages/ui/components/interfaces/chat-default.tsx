"use client";

import { createId } from "@paralleldrive/cuid2";
import { useLiveQuery } from "dexie-react-hooks";
import { HistoryIcon, SquarePenIcon } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { memo, useCallback, useEffect, useMemo, useRef } from "react";
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
import { Button, HoverCard, HoverCardContent, HoverCardTrigger } from "../ui";
import { fileToAttachment } from "./chat-default/attachment";
import { Chat, type IChatRef } from "./chat-default/chat";
import {
	type IAttachment,
	type IMessage,
	chatDb,
} from "./chat-default/chat-db";
import type { ISendMessageFunction } from "./chat-default/chatbox";
import { ChatHistory } from "./chat-default/history";
import { ChatWelcome } from "./chat-default/welcome";
import type { IUseInterfaceProps } from "./interfaces";

export const ChatInterfaceMemoized = memo(function ChatInterface({
	appId,
	event,
	config = {},
	toolbarRef,
	sidebarRef,
}: Readonly<IUseInterfaceProps>) {
	const backend = useBackend();
	const searchParams = useSearchParams();
	const sessionIdParameter = searchParams.get("sessionId") ?? "";
	const setQueryParams = useSetQueryParams();
	const chatRef = useRef<IChatRef>(null);

	useEffect(() => {
		if (!sessionIdParameter || sessionIdParameter === "") {
			const newSessionId = createId();
			setQueryParams("sessionId", newSessionId);
		}
	}, [sessionIdParameter, setQueryParams]);

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
						// Handle chat history toggle
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

			const lastMessages = messages?.slice(-history_elements) ?? [];

			if (!sessionIdParameter || sessionIdParameter === "") {
				toast.error("Session ID is not set. Please start a new chat.");
				return;
			}
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

			const userMessage: IMessage = {
				id: createId(),
				sessionId: sessionIdParameter,
				appId,
				files: otherAttachments,
				inner: historyMessage,
				timestamp: Date.now(),
				tools: activeTools ?? [],
				actions: [],
			};

			const sessionExists = await chatDb.sessions
				.where("id")
				.equals(sessionIdParameter)
				.count();

			if (sessionExists <= 0) {
				await chatDb.sessions.add({
					id: sessionIdParameter,
					appId,
					summarization: content,
					createdAt: Date.now(),
					updatedAt: Date.now(),
				});
			} else {
				await chatDb.sessions.update(sessionIdParameter, {
					updatedAt: Date.now(),
				});
			}

			await chatDb.messages.add(userMessage);

			let intermediateResponse = Response.default();

			const payload = {
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

			let done = false;
			responseMessage.inner.content = "";
			chatRef.current?.pushCurrentMessageUpdate({ ...responseMessage });
			chatRef.current?.scrollToBottom();

			let tmpLocalState = localState;
			let tmpGlobalState = globalState;

			const attachments: Map<string, IAttachment> = new Map();

			const addAttachments = (newAttachments: IAttachment[]) => {
				for (const attachment of newAttachments) {
					if (typeof attachment === "string" && !attachments.has(attachment)) {
						attachments.set(attachment, attachment);
					}

					if (
						typeof attachment !== "string" &&
						!attachments.has(attachment.url)
					) {
						attachments.set(attachment.url, attachment);
					}
				}

				responseMessage.files = Array.from(attachments.values());

				chatRef.current?.pushCurrentMessageUpdate({
					...responseMessage,
				});

				chatRef.current?.scrollToBottom();
			};

			await backend.eventState.executeEvent(
				appId,
				event.id,
				{
					id: event.node_id,
					payload: payload,
				},
				false,
				(execution_id: string) => {},
				(events) => {
					for (const ev of events) {
						if (ev.event_type === "chat_stream_partial") {
							if (done) continue;
							if (ev.payload.chunk)
								intermediateResponse.pushChunk(ev.payload.chunk);
							const lastMessage = intermediateResponse.lastMessageOfRole(
								IRole.Assistant,
							);
							if (lastMessage) {
								responseMessage.inner.content = lastMessage.content ?? "";
								chatRef.current?.pushCurrentMessageUpdate({
									...responseMessage,
								});
								chatRef.current?.scrollToBottom();
							}
							if (ev.payload.attachments) {
								addAttachments(ev.payload.attachments);
							}
							continue;
						}
						if (ev.event_type === "chat_stream") {
							if (done) continue;
							if (ev.payload.response) {
								intermediateResponse = Response.fromObject(ev.payload.response);
								const lastMessage = intermediateResponse.lastMessageOfRole(
									IRole.Assistant,
								);
								if (lastMessage) {
									responseMessage.inner.content = lastMessage.content ?? "";
									chatRef.current?.pushCurrentMessageUpdate({
										...responseMessage,
									});
									chatRef.current?.scrollToBottom();
								}
								continue;
							}
						}
						if (ev.event_type === "chat_out") {
							done = true;
							if (ev.payload.response) {
								intermediateResponse = Response.fromObject(ev.payload.response);
							}

							if (ev.payload.attachments) {
								addAttachments(ev.payload.attachments);
							}
						}

						if (ev.event_type === "chat_local_session") {
							if (tmpLocalState) {
								tmpLocalState = {
									...tmpLocalState,
									localState: ev.payload,
								};
							} else {
								tmpLocalState = {
									id: createId(),
									appId,
									eventId: event.id,
									sessionId: sessionIdParameter,
									localState: ev.payload,
								};
							}
						}

						if (ev.event_type === "chat_global_session") {
							if (tmpGlobalState) {
								tmpGlobalState = {
									...tmpGlobalState,
									globalState: ev.payload,
								};
							} else {
								tmpGlobalState = {
									id: createId(),
									appId,
									eventId: event.id,
									globalState: ev.payload,
								};
							}
						}
						console.log("Event received:", ev);
					}
				},
			);

			if (tmpLocalState) {
				await chatDb.localStage.put(tmpLocalState);
			}

			if (tmpGlobalState) {
				await chatDb.globalState.put(tmpGlobalState);
			}

			const lastMessage = intermediateResponse.lastMessageOfRole(
				IRole.Assistant,
			);
			if (lastMessage) {
				responseMessage.inner.content = lastMessage.content ?? "";
			}
			await chatDb.messages.add(responseMessage);
			chatRef.current?.clearCurrentMessageUpdate();
			await new Promise((resolve) => setTimeout(resolve, 100));
			chatRef.current?.scrollToBottom();
		},
		[
			backend,
			sessionIdParameter,
			appId,
			event.name,
			messages,
			localState,
			globalState,
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
