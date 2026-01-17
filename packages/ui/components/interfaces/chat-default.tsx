"use client";

import { createId } from "@paralleldrive/cuid2";
import { useLiveQuery } from "dexie-react-hooks";
import {
	ChevronDownIcon,
	HistoryIcon,
	HomeIcon,
	SquarePenIcon,
} from "lucide-react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
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
import {
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	HoverCard,
	HoverCardContent,
	HoverCardTrigger,
} from "../ui";
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

	// Clear the streaming message BEFORE writing to Dexie to prevent duplicates
	chatRef.current?.clearCurrentMessageUpdate();

	await chatDb.messages.put(responseMessage);

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
	const router = useRouter();
	const backend = useBackend();
	const executionEngine = useExecutionEngine();
	const searchParams = useSearchParams();
	const pathname = usePathname();
	const sessionIdParameter = searchParams.get("sessionId") ?? "";
	const setQueryParams = useSetQueryParams();
	const chatRef = useRef<IChatRef>(null);
	const activeSubscriptions = useRef<string[]>([]);
	const processedCompletedStreams = useRef<Set<string>>(new Set());
	const [isSendingFromWelcome, setIsSendingFromWelcome] = useState(false);
	const lastNavigateToRef = useRef<string | null>(null);

	const buildUseNavigationUrl = useCallback(
		(route: string, queryParams?: Record<string, string>): string => {
			let navUrl = route;

			if (!route) {
				return `/use?id=${appId}&route=/`;
			}

			if (appId && !route.startsWith("/use") && !route.startsWith("http")) {
				const [routePath, routeQueryString] = route.split("?");
				const params = new URLSearchParams();
				params.set("id", appId);
				params.set("route", routePath || "/");
				params.delete("eventId");

				if (routeQueryString) {
					const routeParams = new URLSearchParams(routeQueryString);
					routeParams.forEach((value, key) => {
						params.set(key, value);
					});
				}

				if (queryParams) {
					for (const [key, value] of Object.entries(queryParams)) {
						params.set(key, value);
					}
				}
				return `/use?${params.toString()}`;
			}

			if (queryParams && Object.keys(queryParams).length > 0) {
				const params = new URLSearchParams(queryParams);
				const separator = navUrl.includes("?") ? "&" : "?";
				navUrl = `${navUrl}${separator}${params.toString()}`;
			}

			return navUrl;
		},
		[appId],
	);

	const handleNavigateTo = useCallback(
		(route: string, replace: boolean, queryParams?: Record<string, string>) => {
			const navUrl = buildUseNavigationUrl(route, queryParams);
			if (replace) {
				router.replace(navUrl);
			} else {
				router.push(navUrl);
			}
		},
		[buildUseNavigationUrl, router],
	);

	const handleNavigationEvents = useCallback(
		(events: any[]) => {
			for (const ev of events) {
				if (ev?.event_type !== "a2ui") continue;
				const message = ev?.payload;
				if (!message || message.type !== "navigateTo") continue;

				const { route, replace, queryParams } = message as {
					route: string;
					replace: boolean;
					queryParams?: Record<string, string>;
				};

				const key = `${route}::${replace ? "r" : "p"}::${JSON.stringify(queryParams ?? {})}`;
				if (lastNavigateToRef.current === key) continue;
				lastNavigateToRef.current = key;

				handleNavigateTo(route, replace, queryParams);
			}
		},
		[handleNavigateTo],
	);

	// Store pending message data for OAuth retry
	const pendingMessageRef = useRef<{
		content: string;
		filesAttached?: File[];
		activeTools?: string[];
		audioFile?: File;
	} | null>(null);

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

	const toolbarElements = useMemo(() => {
		const normalizeRoute = (value: string): string => {
			const trimmed = value.trim();
			if (!trimmed) return "";
			return trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
		};

		const configuredRoutes = (() => {
			const rawArray = (config as any)?.navigate_to_routes;
			const raw: string[] = Array.isArray(rawArray) ? rawArray : [];
			const normalized = raw
				.map((r) => normalizeRoute(String(r)))
				.filter((r) => !!r);
			return Array.from(new Set(normalized));
		})();

		const elements = [
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
		];

		const getRouteLabel = (path: string): string => {
			if (path === "/") return "Home";
			return path.replace(/^\//, "").replace(/-/g, " ").replace(/\//g, " / ");
		};

		const getRouteIcon = (path: string) => {
			if (path === "/") return <HomeIcon className="h-4 w-4" />;
			return null;
		};

		// Single route: pill button
		if (configuredRoutes.length === 1) {
			const route = configuredRoutes[0];
			const icon = getRouteIcon(route);
			elements.push(
				<Button
					key={`navigate-${route}`}
					variant="outline"
					size="sm"
					onClick={() => handleNavigateTo(route, false)}
					className="rounded-full px-4 gap-2 font-medium"
				>
					{icon}
					{getRouteLabel(route)}
				</Button>,
			);
		} else if (configuredRoutes.length === 2) {
			// Two routes: segmented control
			elements.push(
				<div
					key="route-nav"
					className="inline-flex items-center rounded-full bg-muted/50 p-0.5"
				>
					{configuredRoutes.map((route) => {
						const icon = getRouteIcon(route);
						return (
							<button
								key={route}
								type="button"
								onClick={() => handleNavigateTo(route, false)}
								className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full transition-all text-muted-foreground hover:text-foreground hover:bg-background hover:shadow-sm"
							>
								{icon}
								{getRouteLabel(route)}
							</button>
						);
					})}
				</div>,
			);
		} else if (configuredRoutes.length >= 3) {
			// 3+ routes: dropdown
			elements.push(
				<DropdownMenu key="navigate-menu">
					<DropdownMenuTrigger asChild>
						<Button
							variant="outline"
							size="sm"
							className="rounded-full px-4 gap-2 font-medium"
						>
							Navigate
							<ChevronDownIcon className="h-3.5 w-3.5 opacity-60" />
						</Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent align="start" className="min-w-40">
						{configuredRoutes.map((route) => {
							const icon = getRouteIcon(route);
							return (
								<DropdownMenuItem
									key={route}
									onSelect={() => handleNavigateTo(route, false)}
									className="gap-2"
								>
									{icon}
									{getRouteLabel(route)}
								</DropdownMenuItem>
							);
						})}
					</DropdownMenuContent>
				</DropdownMenu>,
			);
		}

		return elements;
	}, [config, handleSidebarToggle, handleNewChat, handleNavigateTo]);

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

	// Reconnect to active stream or process completed stream when component mounts or session changes
	useEffect(() => {
		if (!sessionIdParameter) return;
		// Wait for messages to be loaded from IndexedDB
		if (messages === undefined) return;

		const streamId = sessionIdParameter;

		// Check if there's a stream (active or completed) for this session
		if (!executionEngine.hasStream(streamId)) return;

		// Prevent processing the same completed stream multiple times
		if (
			executionEngine.isStreamComplete(streamId) &&
			processedCompletedStreams.current.has(streamId)
		) {
			return;
		}

		const subscriberId = `chat-reconnect-${sessionIdParameter}`;

		// Skip if we already have an active subscription for this stream (from handleSendMessage)
		// This prevents duplicate message creation when the reconnection effect re-runs
		if (activeSubscriptions.current.length > 0) {
			return;
		}

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

		// If stream is already complete, save to IndexedDB directly
		// (chatRef may not be mounted yet since Chat only renders when messages exist)
		if (executionEngine.isStreamComplete(streamId)) {
			// Mark as processed before saving to prevent duplicate processing
			processedCompletedStreams.current.add(streamId);

			const accumulatedEvents = executionEngine.getAccumulatedEvents(streamId);
			if (accumulatedEvents.length > 0) {
				handleNavigationEvents(accumulatedEvents);

				// Pass done: false so that chat_stream_partial and chat_stream events are processed
				// to extract the message content from the accumulated events
				const result = processChatEvents(accumulatedEvents, {
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

				// Save directly to IndexedDB - useLiveQuery will pick it up
				chatDb.messages.put(result.responseMessage);
				executionEngine.unsubscribeFromEventStream(streamId, subscriberId);
			}
			return;
		}

		// For active streams, wait for Chat component to be mounted (messages.length > 0)
		// before subscribing, since we need chatRef to push updates
		if (messages.length === 0) return;

		// For active streams, subscribe to receive events
		executionEngine.subscribeToEventStream(
			streamId,
			subscriberId,
			(events) => {
				handleNavigationEvents(events);

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
						...responseMessage,
					});
					chatRef.current?.scrollToBottom();
				}
			},
			async (events) => {
				handleNavigationEvents(events);

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
	}, [
		sessionIdParameter,
		appId,
		event.id,
		event.name,
		executionEngine,
		handleNavigationEvents,
		messages,
	]);

	// Internal function to execute the chat (called after OAuth is confirmed)
	const executeChatMessage = useCallback(
		async (
			content: string,
			filesAttached?: File[],
			activeTools?: string[],
			audioFile?: File,
			skipConsentCheck?: boolean,
		) => {
			const isOffline = await backend.isOffline(appId);
			const history_elements =
				parseUint8ArrayToJson(event.config)?.history_elements ?? 5;

			// Check OAuth BEFORE adding message to DB (skip if consent was just granted)
			console.log(
				"[Chat] Checking OAuth. isOffline:",
				isOffline,
				"skipConsentCheck:",
				skipConsentCheck,
			);
			if (!skipConsentCheck && backend.eventState.checkEventOAuth) {
				const oauthResult = await backend.eventState.checkEventOAuth(
					appId,
					event,
				);
				console.log(
					"[Chat] OAuth check result:",
					oauthResult.missingProviders.length,
					"missing providers",
				);
				if (oauthResult.missingProviders.length > 0) {
					// Store pending message for retry
					pendingMessageRef.current = {
						content,
						filesAttached,
						activeTools,
						audioFile,
					};
					// Emit OAuth required event
					window.dispatchEvent(
						new CustomEvent("flow:oauth-required", {
							detail: {
								missingProviders: oauthResult.missingProviders,
								appId,
								boardId: event.board_id,
								nodeId: event.node_id,
								payload: {},
							},
						}),
					);
					return; // Don't add message to DB yet
				}
			}

			// Clear pending message since OAuth is satisfied
			pendingMessageRef.current = null;

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
				path: `${pathname}?id=${appId}&eventId=${event.id}&sessionId=${sessionIdParameter}`,
				title: event.name || "Chat",
				interfaceType: "chat",
				skipConsentCheck,
			});
			executionEngine.subscribeToEventStream(
				streamId,
				subscriberId,
				(events) => {
					handleNavigationEvents(events);

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
					handleNavigationEvents(events);

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
		},
		[
			backend,
			executionEngine,
			sessionIdParameter,
			appId,
			event,
			messages,
			localState,
			globalState,
			handleNavigationEvents,
			pathname,
		],
	);

	// Listen for OAuth retry events
	useEffect(() => {
		const handleOAuthRetry = (e: Event) => {
			const retryEvent = e as CustomEvent<{
				appId: string;
				boardId?: string;
				nodeId?: string;
				skipConsentCheck?: boolean;
			}>;

			const { appId: eventAppId, boardId } = retryEvent.detail;
			console.log("[Chat] OAuth retry event received:", {
				eventAppId,
				boardId,
				appId,
				eventBoardId: event.board_id,
			});

			// Only handle if this is for our app (boardId may be undefined from execution engine)
			if (eventAppId !== appId) {
				console.log("[Chat] OAuth retry event not for this app, ignoring");
				return;
			}

			// If boardId is provided, also check it matches
			if (boardId && boardId !== event.board_id) {
				console.log("[Chat] OAuth retry event not for this board, ignoring");
				return;
			}

			const pending = pendingMessageRef.current;
			if (!pending) {
				console.log("[Chat] No pending message to retry");
				return;
			}

			console.log("[Chat] Re-sending pending message with skipConsentCheck");

			// Re-execute - consent was just granted so skip the check
			executeChatMessage(
				pending.content,
				pending.filesAttached,
				pending.activeTools,
				pending.audioFile,
				true, // skipConsentCheck
			).catch((err) => {
				console.error("Failed to retry chat message after OAuth:", err);
				toast.error("Failed to send message. Please try again.");
			});
		};

		window.addEventListener("flow:oauth-retry", handleOAuthRetry);
		return () => {
			window.removeEventListener("flow:oauth-retry", handleOAuthRetry);
		};
	}, [appId, event.board_id, executeChatMessage]);

	const handleSendMessage: ISendMessageFunction = useCallback(
		async (
			content,
			filesAttached,
			activeTools?: string[],
			audioFile?: File,
		) => {
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
				await executeChatMessage(
					content,
					filesAttached,
					activeTools,
					audioFile,
				);
			} catch (error) {
				// OAuth errors are handled by execution engine - don't show error toast for those
				if (!(error as any)?.isOAuthError) {
					console.error("Error sending message:", error);
					toast.error("Failed to send message. Please try again.");
				}
			} finally {
				setIsSendingFromWelcome(false);
			}
		},
		[sessionIdParameter, messages, executeChatMessage],
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
