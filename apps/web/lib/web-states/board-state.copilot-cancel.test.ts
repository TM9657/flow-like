import type { IChannelHandle } from "@flow-like/flow-like-ui/lib/schema/channel";
import type { CopilotToolContext } from "@flow-like/flow-like-ui/lib/schema/copilot";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	cancelChannel: vi.fn<(handle: unknown) => Promise<void>>(async () => {}),
	dispatchSpecialistToolRequest: vi.fn(async () => undefined),
	isChannelHandle: vi.fn((value: unknown) => Boolean(value)),
}));

vi.mock("@flow-like/flow-like-ui", () => ({}));
vi.mock("@flow-like/flow-like-ui/lib/channel", () => ({
	cancelChannel: mocks.cancelChannel,
	isChannelHandle: mocks.isChannelHandle,
}));
vi.mock(
	"@flow-like/flow-like-ui/state/global-chat/global-chat-run-control",
	() => ({ globalChatTransportRunId: vi.fn() }),
);
vi.mock(
	"@flow-like/flow-like-ui/state/global-chat/global-chat-tool-registry",
	() => ({ runGlobalChatTool: vi.fn() }),
);
vi.mock(
	"@flow-like/flow-like-ui/state/global-chat/global-chat-web-transport",
	() => ({
		dispatchSpecialistToolRequest: mocks.dispatchSpecialistToolRequest,
	}),
);
vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../oauth-db", () => ({
	oauthConsentStore: {},
	oauthTokenStore: {},
}));
vi.mock("../oauth-service", () => ({
	getOAuthApiBaseUrl: vi.fn(),
	getOAuthService: vi.fn(),
}));
vi.mock("./api-utils", () => ({
	apiDelete: vi.fn(),
	apiGet: vi.fn(),
	apiPatch: vi.fn(),
	apiPost: vi.fn(),
	apiPut: vi.fn(),
	getApiBaseUrl: () => "https://api.example.test",
}));

import { WebBoardState } from "./board-state";

const encoder = new TextEncoder();

function channel(id: string): IChannelHandle {
	return {
		channel_id: id,
		expires_at: 4_000_000_000,
		transport: {
			type: "http",
			push_url: `https://api.example.test/channels/${id}`,
			token: "channel-token",
		},
	};
}

function copilotStream(runChannel: IChannelHandle, announceRun = true) {
	let controller: ReadableStreamDefaultController<Uint8Array> | undefined;
	const sendRun = () =>
		controller?.enqueue(
			encoder.encode(
				`event: run\ndata: ${JSON.stringify({ runId: runChannel.channel_id, channel: runChannel })}\n\n`,
			),
		);
	const body = new ReadableStream<Uint8Array>({
		start(streamController) {
			controller = streamController;
			if (announceRun) sendRun();
		},
	});
	return {
		response: new Response(body, {
			headers: { "content-type": "text/event-stream" },
		}),
		failOnAbort(signal: AbortSignal) {
			signal.addEventListener(
				"abort",
				() => controller?.error(new DOMException("Aborted", "AbortError")),
				{ once: true },
			);
		},
		sendRun,
		sendToken(token: string) {
			controller?.enqueue(encoder.encode(`event: token\ndata: ${token}\n\n`));
		},
		sendToolRequest(request: Record<string, unknown>) {
			controller?.enqueue(
				encoder.encode(
					`event: tool_request\ndata: ${JSON.stringify(request)}\n\n`,
				),
			);
		},
		finish(result: Record<string, unknown> = { message: "done" }) {
			controller?.enqueue(
				encoder.encode(`event: final\ndata: ${JSON.stringify(result)}\n\n`),
			);
			controller?.close();
		},
	};
}

function startChat(
	state: WebBoardState,
	requestId: string,
	onToken: (token: string) => void = () => undefined,
	toolContext?: CopilotToolContext,
) {
	return state.copilot_chat(
		"DataStudio",
		null,
		undefined,
		[],
		null,
		null,
		[],
		"Find active customers",
		[],
		undefined,
		onToken,
		"openai:gpt-5",
		undefined,
		undefined,
		undefined,
		undefined,
		true,
		true,
		toolContext,
		requestId,
	);
}

describe("WebBoardState Copilot cancellation", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	test("cancels the server run through its channel before aborting the stream", async () => {
		const runChannel = channel("run-1");
		const stream = copilotStream(runChannel);
		let requestSignal: AbortSignal | undefined;
		vi.stubGlobal(
			"fetch",
			vi.fn(async (_url: string, init?: RequestInit) => {
				requestSignal = init?.signal as AbortSignal;
				stream.failOnAbort(requestSignal);
				return stream.response;
			}),
		);
		let signalWasAbortedAtDelivery: boolean | undefined;
		mocks.cancelChannel.mockImplementationOnce(async () => {
			signalWasAbortedAtDelivery = requestSignal?.aborted;
		});

		const state = new WebBoardState({ auth: undefined } as never);
		const chat = startChat(state, "query-1").catch((error) => error);
		await vi.waitFor(() => expect(mocks.isChannelHandle).toHaveBeenCalled());

		await state.cancelCopilotChat("query-1");

		expect(mocks.cancelChannel).toHaveBeenCalledOnce();
		expect(mocks.cancelChannel).toHaveBeenCalledWith(runChannel);
		expect(signalWasAbortedAtDelivery).toBe(false);
		expect(requestSignal?.aborted).toBe(true);
		expect(await chat).toBeInstanceOf(Error);
	});

	test("keeps a just-started stream alive long enough to receive its cancel channel", async () => {
		const runChannel = channel("run-delayed");
		const stream = copilotStream(runChannel, false);
		let requestSignal: AbortSignal | undefined;
		vi.stubGlobal(
			"fetch",
			vi.fn(async (_url: string, init?: RequestInit) => {
				requestSignal = init?.signal as AbortSignal;
				stream.failOnAbort(requestSignal);
				return stream.response;
			}),
		);
		const onToken = vi.fn();
		const state = new WebBoardState({ auth: undefined } as never);
		const chat = startChat(state, "query-delayed", onToken).catch(
			(error) => error,
		);
		await vi.waitFor(() => expect(fetch).toHaveBeenCalled());

		const cancellation = state.cancelCopilotChat("query-delayed");
		expect(requestSignal?.aborted).toBe(false);
		stream.sendToken("must not reach the caller");
		stream.sendRun();
		await cancellation;

		expect(onToken).not.toHaveBeenCalled();
		expect(mocks.cancelChannel).toHaveBeenCalledWith(runChannel);
		expect(requestSignal?.aborted).toBe(true);
		expect(await chat).toBeInstanceOf(Error);
	});

	test("a stale stream cannot clear the control for its replacement", async () => {
		const firstChannel = channel("run-1");
		const secondChannel = channel("run-2");
		const streams = [copilotStream(firstChannel), copilotStream(secondChannel)];
		let fetchIndex = 0;
		vi.stubGlobal(
			"fetch",
			vi.fn(async (_url: string, init?: RequestInit) => {
				const stream = streams[fetchIndex++];
				stream.failOnAbort(init?.signal as AbortSignal);
				return stream.response;
			}),
		);

		const state = new WebBoardState({ auth: undefined } as never);
		const first = startChat(state, "query-1").catch((error) => error);
		await vi.waitFor(() =>
			expect(mocks.isChannelHandle).toHaveBeenCalledTimes(1),
		);
		const second = startChat(state, "query-1").catch((error) => error);
		await vi.waitFor(() =>
			expect(mocks.isChannelHandle).toHaveBeenCalledTimes(2),
		);

		await state.cancelCopilotChat("query-1");

		expect(mocks.cancelChannel.mock.calls.map(([handle]) => handle)).toEqual([
			firstChannel,
			secondChannel,
		]);
		expect(await first).toBeInstanceOf(Error);
		expect(await second).toBeInstanceOf(Error);
	});

	test("carries the pinned Home profile into model resolution and nested tool dispatch", async () => {
		const runChannel = channel("run-home");
		const stream = copilotStream(runChannel);
		vi.stubGlobal(
			"fetch",
			vi.fn(async () => stream.response),
		);

		const state = new WebBoardState({
			auth: undefined,
			profile: { id: "profile-b" },
		} as never);
		const chat = startChat(state, "home-agent", () => undefined, {
			parentRequestId: "outer-home-1",
			profileId: "profile-a",
		});
		await vi.waitFor(() => expect(mocks.isChannelHandle).toHaveBeenCalled());
		expect(
			JSON.parse(vi.mocked(fetch).mock.calls[0][1]?.body as string),
		).toMatchObject({ profile_id: "profile-a" });
		stream.sendToolRequest({
			requestId: "home-apply-1",
			toolName: "apply_home_layout",
			arguments: {},
			channel: channel("home-tool"),
		});

		await vi.waitFor(() =>
			expect(mocks.dispatchSpecialistToolRequest).toHaveBeenCalledWith(
				expect.objectContaining({
					parentRequestId: "outer-home-1",
					profileId: "profile-a",
				}),
			),
		);
		stream.finish();
		await chat;
	});

	test("sends the selected profile when the caller has no pinned Home context", async () => {
		const stream = copilotStream(channel("run-selected-profile"));
		vi.stubGlobal(
			"fetch",
			vi.fn(async () => stream.response),
		);
		const state = new WebBoardState({
			auth: undefined,
			profile: { id: "selected-profile" },
		} as never);
		const chat = startChat(state, "selected-profile-agent");
		await vi.waitFor(() => expect(fetch).toHaveBeenCalled());
		expect(
			JSON.parse(vi.mocked(fetch).mock.calls[0][1]?.body as string),
		).toMatchObject({ profile_id: "selected-profile" });
		stream.finish();
		await chat;
	});
});
