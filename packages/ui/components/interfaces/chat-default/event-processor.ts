import { createId } from "@paralleldrive/cuid2";
import { IRole, Response } from "../../../lib";
import type { IAttachment, IMessage } from "./chat-db";

export interface ProcessChatEventsResult {
	intermediateResponse: Response;
	responseMessage: IMessage;
	attachments: Map<string, IAttachment>;
	tmpLocalState: any;
	tmpGlobalState: any;
	done: boolean;
	shouldUpdate: boolean;
}

export function processChatEvents(
	events: any[],
	initialState: {
		intermediateResponse: Response;
		responseMessage: IMessage;
		attachments: Map<string, IAttachment>;
		tmpLocalState: any;
		tmpGlobalState: any;
		done: boolean;
		appId: string;
		eventId: string;
		sessionId: string;
	},
): ProcessChatEventsResult {
	let {
		intermediateResponse,
		responseMessage,
		attachments,
		tmpLocalState,
		tmpGlobalState,
		done,
	} = initialState;
	let shouldUpdate = false;
	const { appId, eventId, sessionId } = initialState;

	const addAttachments = (newAttachments: IAttachment[]) => {
		for (const attachment of newAttachments) {
			if (typeof attachment === "string" && !attachments.has(attachment)) {
				attachments.set(attachment, attachment);
			}

			if (typeof attachment !== "string" && !attachments.has(attachment.url)) {
				attachments.set(attachment.url, attachment);
			}
		}
		responseMessage.files = Array.from(attachments.values());
	};

	for (const ev of events) {
		if (ev.event_type === "chat_stream_partial") {
			if (done) continue;
			if (ev.payload.chunk) {
				intermediateResponse.pushChunk(ev.payload.chunk);
				shouldUpdate = true;
			}
			const lastMessage = intermediateResponse.lastMessageOfRole(
				IRole.Assistant,
			);
			if (lastMessage) {
				responseMessage.inner.content = lastMessage.content ?? "";
			}
			if (ev.payload.attachments) {
				addAttachments(ev.payload.attachments);
				shouldUpdate = true;
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
					shouldUpdate = true;
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
				shouldUpdate = true;
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
					eventId: eventId,
					sessionId: sessionId,
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
					eventId: eventId,
					globalState: ev.payload,
				};
			}
		}
	}

	return {
		intermediateResponse,
		responseMessage,
		attachments,
		tmpLocalState,
		tmpGlobalState,
		done,
		shouldUpdate,
	};
}
