import type {
	IBoard,
	ILayer,
} from "@flow-like/flow-like-ui/lib/schema/flow/board";
import {
	ICommentType,
	ILayerType,
} from "@flow-like/flow-like-ui/lib/schema/flow/board";
import type { IBoardSyncResponse } from "@flow-like/flow-like-ui/lib/board-sync";
import { beforeEach, describe, expect, test, vi } from "vitest";
const mocks = vi.hoisted(() => ({ apiPost: vi.fn() }));
vi.mock("@flow-like/flow-like-ui", () => ({
	finishAllProgressToasts: vi.fn(),
}));
vi.mock("../oauth-db", () => ({ oauthConsentStore: {}, oauthTokenStore: {} }));
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
	apiPost: mocks.apiPost,
	getApiBaseUrl: () => "https://api.example.test",
}));
import { WebBoardState } from "./board-state";

function response(): IBoardSyncResponse {
	return {
		manifest: {
			meta: "m1",
			variables: "v1",
			comments: "c1",
			layers: { layer: "l1" },
			segments: {},
		},
		meta: {
			id: "board",
			name: "Board",
			description: "",
			viewport: [0, 0, 1],
			version: [0, 0, 1],
			stage: "Dev",
			log_level: "Info",
			execution_mode: "Hybrid",
			page_ids: [],
			created_at: { secs_since_epoch: 1, nanos_since_epoch: 0 },
			updated_at: { secs_since_epoch: 1, nanos_since_epoch: 0 },
		},
		comments: {},
		variables: {},
		refs: {},
		segments: {},
		layers: {
			layer: {
				id: "layer",
				name: "Media",
				type: ILayerType.Collapsed,
				coordinates: [0, 0, 0],
				nodes: {},
				pins: {},
				variables: {},
				comments: {
					image: {
						id: "image",
						content: "image.png",
						comment_type: ICommentType.Image,
						coordinates: [0, 0, 0],
						timestamp: { secs_since_epoch: 1, nanos_since_epoch: 0 },
					},
				},
			} as ILayer,
		},
	} as IBoardSyncResponse;
}

beforeEach(() => {
	mocks.apiPost.mockReset();
});
describe("web board media", () => {
	test("resolves layer-only media in one request and preserves the sync source", async () => {
		const initial = response();
		let syncs = 0;
		mocks.apiPost.mockImplementation(async (path: string) => {
			if (path.endsWith("/sync"))
				return syncs++ === 0 ? initial : { manifest: initial.manifest };
			if (path.endsWith("/data/download"))
				return [
					{
						prefix: "apps/app/upload/boards/board/image.png",
						url: "signed-image",
					},
				];
			throw new Error(`Unexpected request ${path}`);
		});
		const state = new WebBoardState({ auth: {} } as never);
		const board = await state.getBoard("app", "board");
		expect(board.layers.layer.comments.image.presigned_url).toBe(
			"signed-image",
		);
		expect(initial.layers!.layer.comments.image.presigned_url).toBeUndefined();
		expect(await state.getBoard("app", "board")).toBe(board);
		expect(mocks.apiPost.mock.calls[1].slice(0, 2)).toEqual([
			"apps/app/data/download",
			{ prefixes: ["apps/app/upload/boards/board/image.png"] },
		]);
	});

	test("merged command responses publish resolved media without editing previous snapshots", async () => {
		const initial = response();
		let signings = 0;
		mocks.apiPost.mockImplementation(async (path: string) => {
			if (path.endsWith("/sync")) return initial;
			if (path.endsWith("/data/download"))
				return [
					{
						prefix: "apps/app/upload/boards/board/image.png",
						url: `signed-${++signings}`,
					},
				];
			if (path === "apps/app/board/board")
				return {
					commands: [],
					sync: {
						manifest: initial.manifest,
						meta: { ...initial.meta, name: "Changed" },
					},
				};
			throw new Error(`Unexpected request ${path}`);
		});
		const state = new WebBoardState({ auth: {} } as never);
		const first = await state.getBoard("app", "board");
		let updated: IBoard | undefined;
		await state.executeCommands("app", "board", [], {
			onBoard: (board) => {
				updated = board;
			},
		});
		expect(updated?.name).toBe("Changed");
		expect(updated?.layers.layer.comments.image.presigned_url).toBe("signed-2");
		expect(first.layers.layer.comments.image.presigned_url).toBe("signed-1");
		expect(initial.layers!.layer.comments.image.presigned_url).toBeUndefined();
	});

	test("a read awaiting media URLs cannot replace a newer command snapshot", async () => {
		const initial = response();
		const signingStarted = Promise.withResolvers<void>();
		const slowSigning =
			Promise.withResolvers<{ prefix: string; url: string }[]>();
		let syncs = 0;
		let signings = 0;
		mocks.apiPost.mockImplementation(async (path: string) => {
			if (path.endsWith("/sync"))
				return syncs++ === 0 ? initial : { manifest: initial.manifest };
			if (path.endsWith("/data/download")) {
				if (++signings === 2) {
					signingStarted.resolve();
					return slowSigning.promise;
				}
				return [
					{
						prefix: "apps/app/upload/boards/board/image.png",
						url: `signed-${signings}`,
					},
				];
			}
			if (path === "apps/app/board/board")
				return {
					commands: [],
					sync: {
						manifest: { ...initial.manifest, meta: "m2" },
						meta: { ...initial.meta, name: "Changed" },
					},
				};
			throw new Error(`Unexpected request ${path}`);
		});
		const state = new WebBoardState({ auth: {} } as never);
		await state.getBoard("app", "board");
		const pendingRead = state.getBoard("app", "board");
		await signingStarted.promise;
		let updated: IBoard | undefined;
		await state.executeCommands("app", "board", [], {
			onBoard: (board) => {
				updated = board;
			},
		});
		expect(updated?.name).toBe("Changed");
		slowSigning.resolve([
			{ prefix: "apps/app/upload/boards/board/image.png", url: "late-old-url" },
		]);
		const completedRead = await pendingRead;
		expect(completedRead.name).toBe("Changed");
		expect(completedRead.layers.layer.comments.image.presigned_url).not.toBe(
			"late-old-url",
		);
	});
});
