import { type IBoard, type IComment, ICommentType } from "../schema/flow/board";
import { setEntry } from "./apply";
import { isBoardSnapshot, markBoardSnapshot } from "./identity";

type Comments = IBoard["comments"];

/** Signed URLs belong to the display snapshot, never the sync revision. */
export class BoardMediaProjector {
	private readonly media = new WeakMap<Comments, IComment[]>();
	private readonly projectedComments = new WeakMap<
		Comments,
		Map<string, Comments>
	>();
	private readonly projectedLayers = new WeakMap<
		IBoard["layers"],
		Map<string, IBoard["layers"]>
	>();
	private readonly projectedLayer = new WeakMap<
		IBoard["layers"][string],
		Map<string, IBoard["layers"][string]>
	>();
	private readonly projectedBoards = new WeakMap<IBoard, Map<string, IBoard>>();

	private commentsMedia(comments: Comments): IComment[] {
		let media = this.media.get(comments);
		if (!media) {
			media = Object.values(comments).filter(
				(comment) =>
					comment.comment_type === ICommentType.Image ||
					comment.comment_type === ICommentType.Video,
			);
			this.media.set(comments, media);
		}
		return media;
	}

	files(board: IBoard): string[] {
		return [
			...new Set(
				[
					...this.commentsMedia(board.comments),
					...Object.values(board.layers).flatMap((layer) =>
						this.commentsMedia(layer.comments),
					),
				].map((comment) => comment.content),
			),
		];
	}

	project(
		board: IBoard,
		scope: string,
		urls: ReadonlyMap<string, string>,
	): IBoard {
		const withUrls = (source: Comments): Comments => {
			let scopes = this.projectedComments.get(source);
			if (!scopes) {
				scopes = new Map();
				this.projectedComments.set(source, scopes);
			}
			const previous = scopes.get(scope) ?? source;
			let next = previous;
			for (const comment of this.commentsMedia(source)) {
				const url = urls.get(comment.content);
				if (!url || url === previous[comment.id]?.presigned_url) continue;
				if (next === previous) next = { ...previous };
				setEntry(next, comment.id, { ...comment, presigned_url: url });
			}
			scopes.set(scope, next);
			return next;
		};
		const comments = withUrls(board.comments);
		let layerScopes = this.projectedLayers.get(board.layers);
		if (!layerScopes) {
			layerScopes = new Map();
			this.projectedLayers.set(board.layers, layerScopes);
		}
		const previousLayers = layerScopes.get(scope) ?? board.layers;
		let layers = previousLayers;
		for (const [id, layer] of Object.entries(board.layers)) {
			const comments = withUrls(layer.comments);
			let scopes = this.projectedLayer.get(layer);
			if (!scopes) {
				scopes = new Map();
				this.projectedLayer.set(layer, scopes);
			}
			const previous = scopes.get(scope) ?? layer;
			const next =
				comments === previous.comments ? previous : { ...layer, comments };
			scopes.set(scope, next);
			if (next === previousLayers[id]) continue;
			if (layers === previousLayers) layers = { ...previousLayers };
			setEntry(layers, id, next);
		}
		layerScopes.set(scope, layers);
		let boardScopes = this.projectedBoards.get(board);
		if (!boardScopes) {
			boardScopes = new Map();
			this.projectedBoards.set(board, boardScopes);
		}
		const previous = boardScopes.get(scope) ?? board;
		if (comments === previous.comments && layers === previous.layers)
			return previous;
		const display = { ...board, comments, layers };
		if (isBoardSnapshot(board)) markBoardSnapshot(display);
		boardScopes.set(scope, display);
		return display;
	}
}
