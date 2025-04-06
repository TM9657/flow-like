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
	ILogLevel,
	INode,
	IProfile,
	IRun,
} from "../lib";
import type { ISettingsProfile } from "../types";
import type { IRunUpdateEvent } from "./run-execution-state";

export interface IBackendState {
	getApps(): Promise<IApp[]>;
	getApp(appId: string): Promise<IApp>;
	getBoards(appId: string): Promise<IBoard[]>;
	getCatalog(): Promise<INode[]>;
	getBoard(appId: string, boardId: string): Promise<IBoard>;
	// [BoardId, BoardName]
	getOpenBoards(): Promise<[string, string][]>;
	getBoardSettings(): Promise<"straight" | "step" | "simpleBezier">;

	executeBoard(
		appId: string,
		boardId: string,
		startIds: string[],
		cb?: (event: IIntercomEvent[]) => void,
	): Promise<string>;

	getRun(appId: string, runId: string): Promise<IRun>;
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
