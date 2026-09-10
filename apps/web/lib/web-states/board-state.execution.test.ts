import {
	BOARD_FORMAT_HEADER,
	CURRENT_BOARD_FORMAT_VERSION,
} from "@flow-like/flow-like-ui/lib/board-format";
import { afterEach, describe, expect, test, vi } from "vitest";

vi.mock("@flow-like/flow-like-ui", () => ({
	finishAllProgressToasts: vi.fn(),
}));
vi.mock("../oauth-db", () => ({
	oauthConsentStore: {},
	oauthTokenStore: {},
}));
vi.mock("../oauth-service", () => ({}));
vi.mock(
	"@flow-like/flow-like-ui/state/global-chat/global-chat-tool-registry",
	() => ({ runGlobalChatTool: vi.fn() }),
);
vi.mock(
	"@flow-like/flow-like-ui/state/global-chat/global-chat-web-transport",
	() => ({ dispatchSpecialistToolRequest: vi.fn() }),
);
vi.mock("./api-utils", () => ({
	getApiBaseUrl: () => "https://api.example.test",
}));

import { WebBoardState } from "./board-state";

function boardState() {
	return new WebBoardState({
		auth: { user: { access_token: "client-token" } },
		profile: { id: "profile-1" },
	} as never);
}

describe("web board execution transport", () => {
	afterEach(() => {
		vi.unstubAllGlobals();
	});

	test("can invoke a format-v2 board with authentication and the selected profile", async () => {
		const fetchMock = vi.fn(async (_url: string, init?: RequestInit) => {
			const supported = Number(
				new Headers(init?.headers).get(BOARD_FORMAT_HEADER) ?? 1,
			);
			return supported < 2
				? Response.json(
						{ error: { code: "BOARD_FORMAT_UPGRADE_REQUIRED" } },
						{ status: 426 },
					)
				: new Response(null, { status: 200 });
		});
		vi.stubGlobal("fetch", fetchMock);

		await expect(
			boardState().executeBoardRemote("app-1", "board-1", { id: "start" }),
		).resolves.toBeUndefined();

		const [url, init] = fetchMock.mock.calls[0];
		expect(url).toBe(
			"https://api.example.test/api/v1/apps/app-1/board/board-1/invoke",
		);
		const headers = new Headers(init?.headers);
		expect(headers.get(BOARD_FORMAT_HEADER)).toBe(
			String(CURRENT_BOARD_FORMAT_VERSION),
		);
		expect(headers.get("Authorization")).toBe("Bearer client-token");
		expect(JSON.parse(init?.body as string)).toMatchObject({
			node_id: "start",
			profile_id: "profile-1",
		});
	});

	test("preserves the server reason and reference when a board invocation fails", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn(async () =>
				Response.json(
					{
						error: {
							code: "BOARD_FORMAT_UPGRADE_REQUIRED",
							message: "This board requires format version 3.",
							id: "error-1",
						},
					},
					{ status: 426 },
				),
			),
		);

		await expect(
			boardState().executeBoardRemote("app-1", "board-1", { id: "start" }),
		).rejects.toMatchObject({
			status: 426,
			code: "BOARD_FORMAT_UPGRADE_REQUIRED",
			errorId: "error-1",
			serverMessage: expect.stringContaining(
				"This board requires format version 3.",
			),
		});
	});

	test("delivers trailing output in the same network chunk as completion", async () => {
		const events = [
			{ event_type: "completed", payload: {} },
			{ event_type: "chat_out", payload: { text: "Final answer" } },
			{ event_type: "usage", payload: { tokens: 12 } },
		];
		vi.stubGlobal(
			"fetch",
			vi.fn(
				async () =>
					new Response(
						events
							.map((event) => `data: ${JSON.stringify(event)}\n\n`)
							.join(""),
						{ headers: { "content-type": "text/event-stream" } },
					),
			),
		);
		const onEvents = vi.fn();

		await boardState().executeBoardRemote(
			"app-1",
			"board-1",
			{ id: "start" },
			true,
			undefined,
			onEvents,
		);

		expect(onEvents.mock.calls.flatMap(([batch]) => batch)).toEqual(events);
	});
});
