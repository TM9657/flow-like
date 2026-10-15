import { describe, expect, test } from "bun:test";
import { type IBoard, type IComment, ICommentType } from "../schema/flow/board";
import { isBoardSnapshot, markBoardSnapshot } from "./identity";
import { BoardMediaProjector } from "./media";

const comment = (
	id: string,
	content: string,
	type = ICommentType.Image,
): IComment => ({
	id,
	content,
	comment_type: type,
	coordinates: [0, 0, 0],
	timestamp: { secs_since_epoch: 0, nanos_since_epoch: 0 },
});
function board(): IBoard {
	return markBoardSnapshot({
		nodes: {},
		refs: {},
		variables: {},
		comments: {},
		layers: {
			media: {
				id: "media",
				comments: {
					image: comment("image", "picture.png"),
					video: comment("video", "movie.mp4", ICommentType.Video),
				},
			},
			empty: { id: "empty", comments: {} },
		},
	} as unknown as IBoard);
}

describe("board media projection", () => {
	test("collects and signs layer-only media without mutating the sync snapshot", () => {
		const projector = new BoardMediaProjector();
		const source = board();
		Object.freeze(source.layers.media.comments.image);
		expect(projector.files(source)).toEqual(["picture.png", "movie.mp4"]);
		const display = projector.project(
			source,
			"app/board",
			new Map([
				["picture.png", "signed-image"],
				["movie.mp4", "signed-video"],
			]),
		);
		expect(display.layers.media.comments.image.presigned_url).toBe(
			"signed-image",
		);
		expect(display.layers.media.comments.video.presigned_url).toBe(
			"signed-video",
		);
		expect(source.layers.media.comments.image.presigned_url).toBeUndefined();
		expect(display.layers.empty).toBe(source.layers.empty);
		expect(display.nodes).toBe(source.nodes);
		expect(isBoardSnapshot(display)).toBe(true);
	});

	test("retains display identities across unchanged refreshes and unrelated graph edits", () => {
		const projector = new BoardMediaProjector();
		const source = board();
		const urls = new Map([["picture.png", "signed"]]);
		const display = projector.project(source, "app/board", urls);
		expect(projector.project(source, "app/board", urls)).toBe(display);
		expect(projector.project(source, "app/board", new Map())).toBe(display);
		const edited = markBoardSnapshot({
			...source,
			nodes: { changed: {} },
		} as unknown as IBoard);
		const next = projector.project(edited, "app/board", urls);
		expect(next.nodes).toBe(edited.nodes);
		expect(next.layers).toBe(display.layers);
		const layerEdit = {
			...edited,
			layers: {
				...source.layers,
				empty: { ...source.layers.empty, name: "Renamed" },
			},
		};
		const nextLayer = projector.project(layerEdit, "app/board", urls);
		expect(nextLayer.layers.media).toBe(display.layers.media);
		expect(nextLayer.layers.empty.name).toBe("Renamed");
	});

	test("URL renewal changes only the affected media and scopes stay separate", () => {
		const projector = new BoardMediaProjector();
		const source = board();
		const first = projector.project(
			source,
			"online",
			new Map([["picture.png", "old"]]),
		);
		const second = projector.project(
			source,
			"online",
			new Map([["picture.png", "new"]]),
		);
		expect(first.layers.media.comments.image.presigned_url).toBe("old");
		expect(second.layers.media.comments.image.presigned_url).toBe("new");
		expect(second.layers.media.comments.video).toBe(
			first.layers.media.comments.video,
		);
		expect(projector.project(source, "offline", new Map())).toBe(source);
	});

	test("deduplicates media paths, ignores text, and does not trust arbitrary boards", () => {
		const projector = new BoardMediaProjector();
		const source = {
			...board(),
			comments: {
				duplicate: comment("duplicate", "picture.png"),
				text: comment("text", "text", ICommentType.Text),
			},
		};
		expect(projector.files(source)).toEqual(["picture.png", "movie.mp4"]);
		expect(
			isBoardSnapshot(
				projector.project(
					source,
					"scope",
					new Map([["picture.png", "signed"]]),
				),
			),
		).toBe(false);
	});
});
