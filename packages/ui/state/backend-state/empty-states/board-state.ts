import type {
	IBoard,
	IBoardState,
	IConnectionMode,
	IExecutionStage,
	IGenericCommand,
	IIntercomEvent,
	ILog,
	ILogLevel,
	ILogMetadata,
	INode,
	IRunPayload,
	IVersionType,
} from "../../../";
import type { IJwks, IRealtimeAccess } from "../../../";
import type {
	ChatMessage,
	CopilotResponse,
} from "../../../components/flow/flow-copilot";

export class EmptyBoardState implements IBoardState {
	getBoards(appId: string): Promise<IBoard[]> {
		throw new Error("Method not implemented.");
	}
	getCatalog(): Promise<INode[]> {
		throw new Error("Method not implemented.");
	}
	getBoard(
		appId: string,
		boardId: string,
		version?: [number, number, number],
	): Promise<IBoard> {
		throw new Error("Method not implemented.");
	}
	getRealtimeAccess(appId: string, boardId: string): Promise<IRealtimeAccess> {
		throw new Error("Method not implemented.");
	}
	getRealtimeJwks(appId: string, boardId: string): Promise<IJwks> {
		throw new Error("Method not implemented.");
	}
	createBoardVersion(
		appId: string,
		boardId: string,
		versionType: IVersionType,
	): Promise<[number, number, number]> {
		throw new Error("Method not implemented.");
	}
	getBoardVersions(
		appId: string,
		boardId: string,
	): Promise<[number, number, number][]> {
		throw new Error("Method not implemented.");
	}
	deleteBoard(appId: string, boardId: string): Promise<void> {
		throw new Error("Method not implemented.");
	}
	getOpenBoards(): Promise<[string, string, string][]> {
		throw new Error("Method not implemented.");
	}
	getBoardSettings(): Promise<IConnectionMode> {
		throw new Error("Method not implemented.");
	}
	executeBoard(
		appId: string,
		boardId: string,
		payload: IRunPayload,
		streamState?: boolean,
		eventId?: (id: string) => void,
		cb?: (event: IIntercomEvent[]) => void,
	): Promise<ILogMetadata | undefined> {
		throw new Error("Method not implemented.");
	}
	listRuns(
		appId: string,
		boardId: string,
		nodeId?: string,
		from?: number,
		to?: number,
		status?: ILogLevel,
		lastMeta?: ILogMetadata,
		offset?: number,
		limit?: number,
	): Promise<ILogMetadata[]> {
		throw new Error("Method not implemented.");
	}
	queryRun(
		logMeta: ILogMetadata,
		query: string,
		offset?: number,
		limit?: number,
	): Promise<ILog[]> {
		throw new Error("Method not implemented.");
	}
	undoBoard(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	): Promise<void> {
		throw new Error("Method not implemented.");
	}
	redoBoard(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	): Promise<void> {
		throw new Error("Method not implemented.");
	}
	upsertBoard(
		appId: string,
		boardId: string,
		name: string,
		description: string,
		logLevel: ILogLevel,
		stage: IExecutionStage,
		template?: IBoard,
	): Promise<void> {
		throw new Error("Method not implemented.");
	}
	closeBoard(boardId: string): Promise<void> {
		throw new Error("Method not implemented.");
	}
	executeCommand(
		appId: string,
		boardId: string,
		command: IGenericCommand,
	): Promise<IGenericCommand> {
		throw new Error("Method not implemented.");
	}
	executeCommands(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	): Promise<IGenericCommand[]> {
		throw new Error("Method not implemented.");
	}

	flowpilot_chat(
		board: IBoard,
		selectedNodeIds: string[],
		userPrompt: string,
		history: ChatMessage[],
		onToken?: (token: string) => void,
		modelId?: string,
		token?: string,
	): Promise<CopilotResponse> {
		throw new Error("Method not implemented.");
	}
}
