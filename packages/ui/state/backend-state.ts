import { create } from "zustand";
import type {
	IApp,
	IBit,
	IBitPack,
	IBitTypes,
	IBoard,
	IDownloadProgress,
	IExecutionStage,
	IFileMetadata,
	IGenericCommand,
	IIntercomEvent,
	ILog,
	ILogLevel,
	ILogMetadata,
	INode,
	IProfile,
	IRunPayload,
	IVersionType,
} from "../lib";
import type { ISettingsProfile } from "../types";

export interface IBackendState {
	getApps(): Promise<IApp[]>;
	getApp(appId: string): Promise<IApp>;
	getBoards(appId: string): Promise<IBoard[]>;
	getCatalog(): Promise<INode[]>;
	getBoard(
		appId: string,
		boardId: string,
		version?: [number, number, number],
	): Promise<IBoard>;
	createBoardVersion(
		appId: string,
		boardId: string,
		versionType: IVersionType,
	): Promise<[number, number, number]>;
	getBoardVersions(
		appId: string,
		boardId: string,
	): Promise<[number, number, number][]>;
	// [AppId, BoardId, BoardName]
	getOpenBoards(): Promise<[string, string, string][]>;
	getBoardSettings(): Promise<"straight" | "step" | "simpleBezier">;

	executeBoard(
		appId: string,
		boardId: string,
		payload: IRunPayload,
		cb?: (event: IIntercomEvent[]) => void,
	): Promise<ILogMetadata | undefined>;

	listRuns(
		appId: string,
		boardId: string,
		nodeId?: string,
		from?: number,
		to?: number,
		status?: ILogLevel,
		limit?: number,
		offset?: number,
		lastMeta?: ILogMetadata,
	): Promise<ILogMetadata[]>;
	queryRun(
		logMeta: ILogMetadata,
		query: string,
		limit?: number,
		offset?: number,
	): Promise<ILog[]>;

	finalizeRun(appId: string, runId: string): Promise<void>;
	undoBoard(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	): Promise<void>;
	redoBoard(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	): Promise<void>;

	updateBoardMeta(
		appId: string,
		boardId: string,
		name: string,
		description: string,
		logLevel: ILogLevel,
		stage: IExecutionStage,
	): Promise<void>;

	closeBoard(boardId: string): Promise<void>;

	// Profile Operations
	getProfile(): Promise<IProfile>;
	getSettingsProfile(): Promise<ISettingsProfile>;

	// Board Operations
	executeCommand(
		appId: string,
		boardId: string,
		command: IGenericCommand,
	): Promise<IGenericCommand>;

	executeCommands(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	): Promise<IGenericCommand[]>;

	registerEvent(
		appId: string,
		boardId: string,
		nodeId: string,
		eventType: string,
		eventId: string,
		ttl?: number,
	): Promise<void>;

	removeEvent(eventId: string, eventType: string): Promise<void>;

	// Additional Functionality
	getPathMeta(folderPath: string): Promise<IFileMetadata[]>;
	openFileOrFolderMenu(
		multiple: boolean,
		directory: boolean,
		recursive: boolean,
	): Promise<string[] | string | undefined>;

	getInstalledBit(bits: IBit[]): Promise<IBit[]>;
	getPackFromBit(bit: IBit): Promise<{
		bits: IBit[];
	}>;
	downloadBit(
		bit: IBit,
		pack: IBitPack,
		cb?: (progress: IDownloadProgress[]) => void,
	): Promise<IBit[]>;
	deleteBit(bit: IBit): Promise<void>;
	getBit(id: string, hub?: string): Promise<IBit>;
	addBit(bit: IBit, profile: ISettingsProfile): Promise<void>;
	removeBit(bit: IBit, profile: ISettingsProfile): Promise<void>;
	getPackSize(bits: IBit[]): Promise<number>;
	getBitSize(bit: IBit): Promise<number>;
	getBitsByCategory(type: IBitTypes): Promise<IBit[]>;
	isBitInstalled(bit: IBit): Promise<boolean>;
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
