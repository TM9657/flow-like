import { describe, expect, test, vi } from "vitest";

import type { IBoardState } from "../../state/backend-state/board-state";
import { createFlowPilotOntologyQueryTextCompletion } from "./flowpilot-adapter";

const response = {
	message:
		'{"language":"cypher","query":"MATCH (n) RETURN n","params":{},"presentation":"graph"}',
	commands: [],
	components: [],
	suggestions: [],
	active_scope: "DataStudio" as const,
};

function boardState(
	copilotChat = vi.fn(async () => response),
	cancelCopilotChat = vi.fn(async () => undefined),
) {
	return {
		copilot_chat: copilotChat,
		cancelCopilotChat,
	} as unknown as Pick<IBoardState, "copilot_chat" | "cancelCopilotChat">;
}

describe("FlowPilot ontology query completion adapter", () => {
	test("uses the streamed, read-only DataStudio path with immutable host context", async () => {
		const state = boardState();
		const complete = createFlowPilotOntologyQueryTextCompletion({
			boardState: state,
			target: {
				appId: "app-1",
				overlayId: "overlay-1",
				userScoped: true,
				surfaceInstanceId: "surface-1",
			},
			provider: "codex",
			modelId: "gpt-5",
			reasoningEffort: "high",
			token: "token-1",
		});
		const result = await complete({
			requestId: "request-1",
			attempt: 1,
			sourcePrompt: "Show everyone",
			systemPrompt: "Owned by the backend",
			userPrompt: '{"question":"Show everyone"}',
			signal: new AbortController().signal,
		});

		expect(result).toBe(response.message);
		expect(state.copilot_chat).toHaveBeenCalledOnce();
		const args = vi.mocked(state.copilot_chat).mock.calls[0];
		expect(args?.slice(0, 11)).toEqual([
			"DataStudio",
			null,
			undefined,
			[],
			null,
			null,
			[],
			'{"question":"Show everyone"}',
			[],
			undefined,
			expect.any(Function),
		]);
		expect(args?.slice(11, 18)).toEqual([
			"codex:gpt-5",
			"high",
			"token-1",
			undefined,
			undefined,
			true,
			true,
		]);
		expect(args?.[18]).toEqual({
			appId: "app-1",
			overlayId: "overlay-1",
			userScoped: true,
			parentRequestId: "request-1",
			conversationId: "surface-1",
			sourceUserPrompt: "Show everyone",
		});
		expect(args?.slice(19)).toEqual(["request-1", "Show everyone", "app-1"]);

		// Calling the callback proves it is a real function while keeping the
		// generated response isolated from generic FlowPilot stream parsing.
		expect((args?.[10] as (token: string) => void)("ignored")).toBeUndefined();
	});

	test("keeps a Bits model id unprefixed", async () => {
		const state = boardState();
		const complete = createFlowPilotOntologyQueryTextCompletion({
			boardState: state,
			target: {
				appId: "app-1",
				overlayId: "overlay-1",
				userScoped: false,
				surfaceInstanceId: "surface-1",
			},
			provider: "bits",
			modelId: "profile-model",
		});

		await complete({
			requestId: "request-1",
			attempt: 1,
			sourcePrompt: "Count people",
			systemPrompt: "backend prompt",
			userPrompt: "payload",
			signal: new AbortController().signal,
		});

		expect(vi.mocked(state.copilot_chat).mock.calls[0]?.[11]).toBe(
			"profile-model",
		);
	});

	test("propagates AbortSignal through the stable FlowPilot request id", async () => {
		let resolveChat!: (value: typeof response) => void;
		const pending = new Promise<typeof response>((resolve) => {
			resolveChat = resolve;
		});
		const copilotChat = vi.fn(async () => pending);
		const cancelCopilotChat = vi.fn(async () => undefined);
		const state = boardState(copilotChat, cancelCopilotChat);
		const complete = createFlowPilotOntologyQueryTextCompletion({
			boardState: state,
			target: {
				appId: "app-1",
				overlayId: "overlay-1",
				userScoped: true,
				surfaceInstanceId: "surface-1",
			},
			provider: "github-copilot",
			modelId: "gpt-4.1",
		});
		const controller = new AbortController();
		const completion = complete({
			requestId: "stable-request-1",
			attempt: 1,
			sourcePrompt: "Show everyone",
			systemPrompt: "backend prompt",
			userPrompt: "payload",
			signal: controller.signal,
		});
		controller.abort();
		resolveChat(response);

		await expect(completion).rejects.toMatchObject({ name: "AbortError" });
		expect(cancelCopilotChat).toHaveBeenCalledOnce();
		expect(cancelCopilotChat).toHaveBeenCalledWith("stable-request-1");
	});

	test("does not start a backend run for an already-aborted signal", async () => {
		const state = boardState();
		const complete = createFlowPilotOntologyQueryTextCompletion({
			boardState: state,
			target: {
				appId: "app-1",
				overlayId: "overlay-1",
				userScoped: true,
				surfaceInstanceId: "surface-1",
			},
			provider: "codex",
			modelId: "default",
		});
		const controller = new AbortController();
		controller.abort();

		await expect(
			complete({
				requestId: "cancelled-before-start",
				attempt: 1,
				sourcePrompt: "Show everyone",
				systemPrompt: "backend prompt",
				userPrompt: "payload",
				signal: controller.signal,
			}),
		).rejects.toMatchObject({ name: "AbortError" });
		expect(state.copilot_chat).not.toHaveBeenCalled();
	});

	test("reports a missing agent model when completion starts", async () => {
		const state = boardState();
		const complete = createFlowPilotOntologyQueryTextCompletion({
			boardState: state,
			target: {
				appId: "app-1",
				overlayId: "overlay-1",
				userScoped: false,
				surfaceInstanceId: "surface-1",
			},
			provider: "claude-code",
		});

		await expect(
			complete({
				requestId: "missing-model",
				attempt: 1,
				sourcePrompt: "Show everyone",
				systemPrompt: "backend prompt",
				userPrompt: "payload",
				signal: new AbortController().signal,
			}),
		).rejects.toThrow("modelId is required");
		expect(state.copilot_chat).not.toHaveBeenCalled();
	});
});
