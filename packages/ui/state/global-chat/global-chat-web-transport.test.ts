import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	replyToChannel: vi.fn(async () => undefined),
}));

vi.mock("../../lib/channel", () => ({
	cancelChannel: vi.fn(async () => undefined),
	isChannelHandle: vi.fn(() => true),
	replyToChannel: mocks.replyToChannel,
	steerChannel: vi.fn(async () => undefined),
}));

import {
	type WebToolRequest,
	dispatchSpecialistToolRequest,
} from "./global-chat-web-transport";

function toolRequest(parentRequestId?: string) {
	return JSON.stringify({
		requestId: "nested-tool-1",
		toolName: "apply_home_layout",
		arguments: { layout: { version: 1, widgets: [] } },
		channel: {
			channel_id: "specialist-channel",
			request_id: "nested-tool-1",
			expires_at: 4_000_000_000,
			transport: { type: "in_process" },
		},
		...(parentRequestId ? { parentRequestId } : {}),
	});
}

describe("dispatchSpecialistToolRequest", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("stamps the known outer request onto hosted specialist tools", async () => {
		const onToolRequest = vi.fn(async (request: WebToolRequest) => ({
			requestId: request.requestId,
			approved: true,
			result: { status: "staged" },
		}));

		await dispatchSpecialistToolRequest({
			data: toolRequest(),
			parentRequestId: "outer-home-1",
			onToolRequest,
		});

		expect(onToolRequest).toHaveBeenCalledWith(
			expect.objectContaining({
				requestId: "nested-tool-1",
				parentRequestId: "outer-home-1",
			}),
		);
		expect(mocks.replyToChannel).toHaveBeenCalledOnce();
	});

	it("keeps an authoritative parent already carried by the frame", async () => {
		const onToolRequest = vi.fn(async (request: WebToolRequest) => ({
			requestId: request.requestId,
			approved: true,
		}));

		await dispatchSpecialistToolRequest({
			data: toolRequest("server-parent"),
			parentRequestId: "fallback-parent",
			onToolRequest,
		});

		expect(onToolRequest).toHaveBeenCalledWith(
			expect.objectContaining({ parentRequestId: "server-parent" }),
		);
	});

	it("pins Home ownership to the launching host even if a frame carries different metadata", async () => {
		const onToolRequest = vi.fn(async (request: WebToolRequest) => ({
			requestId: request.requestId,
			approved: true,
		}));
		await dispatchSpecialistToolRequest({
			data: JSON.stringify({
				...JSON.parse(toolRequest("wrong-parent")),
				context: { profileId: "profile-b" },
			}),
			parentRequestId: "home-parent",
			profileId: "profile-a",
			onToolRequest,
		});
		expect(onToolRequest).toHaveBeenCalledWith(
			expect.objectContaining({
				parentRequestId: "home-parent",
				context: expect.objectContaining({ profileId: "profile-a" }),
			}),
		);
	});
});
