import type {
	IBit,
	IComment,
	IDownloadProgress,
	IExecutionStage,
	IFileMetadata,
	ILogLevel,
	INode,
	IProfile,
	IRun,
} from "../lib";
import type { IRunUpdateEvent } from "./run-execution-state";
import { create } from "zustand";

export interface IBackendState {
	getCatalog(): Promise<INode[]>;
	getBoard(id: string): Promise<INode>;
	// [BoardId, BoardName]
	getOpenBoards(): Promise<[string, string][]>;
	getBoardSettings(): Promise<"straight" | "step" | "simpleBezier">;

	createRun(boardId: string, startIds: string[]): Promise<string>;
	executeRun(runId: string): Promise<void>;
	getRun(runId: string): Promise<IRun>;
	finalizeRun(runId: string): Promise<void>;
	onRunUpdate(
		runId: string,
		callback: (run: IRun) => IRunUpdateEvent[],
	): () => void;

	undoBoard(boardId: string): Promise<void>;
	redoBoard(boardId: string): Promise<void>;

	updateBoardMeta(
		boardId: string,
		name: string,
		description: string,
		logLevel: ILogLevel,
		stage: IExecutionStage,
	): Promise<void>;

	closeBoard(boardId: string): Promise<void>;

	// Profile Operations
	getProfile(): Promise<IProfile>;

	// Board Operations
	upsertComment(
		boardId: string,
		comment: IComment,
		append: boolean,
	): Promise<void>;
	updateNode(boardId: string, node: INode, append: boolean): Promise<void>;
	moveNode(
		boardId: string,
		nodeId: string,
		coordinates: [number, number, number],
		append: boolean,
	): Promise<void>;
	addNodeToBoard(boardId: string, node: INode, append: boolean): Promise<INode>;
	connectPins(
		boardId: string,
		fromNode: string,
		fromPin: string,
		toNode: string,
		toPin: string,
		append: boolean,
	): Promise<void>;
	disconnectPins(
		boardId: string,
		fromNode: string,
		fromPin: string,
		toNode: string,
		toPin: string,
		append: boolean,
	): Promise<void>;
	removeNode(boardId: string, nodeId: INode, append: boolean): Promise<void>;
	removeComment(
		boardId: string,
		commentId: IComment,
		append: boolean,
	): Promise<void>;

	// Additional Functionality
	gefFolderMeta(folderPath: string): Promise<IFileMetadata[]>;
	openFileOrFolderMenu(
		multiple: boolean,
		directory: boolean,
		recursive: boolean,
	): Promise<string[] | string | undefined>;

	getInstalledBit(bits: IBit[]): Promise<IBit[]>;
	getPackFromBit(bit: IBit): Promise<{
		bits: IBit[];
	}>;
	downloadBit(bit: IBit): Promise<void>;
	getPackSize(bits: IBit[]): Promise<number>;
	getBitSize(bit: IBit): Promise<number>;
	isBitInstalled(bit: IBit): Promise<boolean>;
	onBitInstallProgress(
		callback: (progress: IDownloadProgress) => void,
	): () => void;
}

interface BackendStoreState {
	backend: IBackendState | null;
	setBackend: (backend: IBackendState) => void;
}

export const useBackendStore = create<BackendStoreState>((set) => ({
	backend: null,
	setBackend: (backend: IBackendState) => set({ backend }),
}));

export function useBackend(): IBackendState {
	const backend = useBackendStore((state) => state.backend);
	if (!backend) throw new Error("Backend not initialized");
	return backend;
}
