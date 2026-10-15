export {
	ROOT_SEGMENT,
	applyBoardSync,
	catalogByName,
	nodeSegment,
	resolveManifest,
	toNode,
	type CatalogByName,
	type IAppliedBoardSync,
	type BoardChangeSet,
} from "./apply";
export { BoardMediaProjector } from "./media";
export {
	BoardSyncClient,
	type BoardSyncTransport,
	type BoardSyncUpdate,
	requestMatchesManifest,
} from "./client";
export type {
	IBoardMeta,
	IBoardSyncManifest,
	IBoardSyncManifestDelta,
	IBoardSyncRequest,
	IBoardSyncResponse,
	ISyncNode,
	ISyncPin,
	ISyncSegment,
} from "./types";
