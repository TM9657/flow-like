import { Channel, invoke } from "@tauri-apps/api/core";
import {
	type IBoard,
	type IBoardState,
	IConnectionMode,
	type IExecutionStage,
	type IGenericCommand,
	type IIntercomEvent,
	type ILog,
	type ILogLevel,
	type ILogMetadata,
	type INode,
	type IRunPayload,
	type ISettingsProfile,
	type IVersionType,
	injectDataFunction,
	isEqual,
} from "@tm9657/flow-like-ui";
import { isObject } from "lodash-es";
import { toast } from "sonner";
import { fetcher } from "../../lib/api";
import type { TauriBackend } from "../tauri-provider";

interface DiffEntry {
	path: string;
	local: any;
	remote: any;
}

const getDeepDifferences = (
	local: any,
	remote: any,
	path = "",
): DiffEntry[] => {
	const differences: DiffEntry[] = [];

	if (!isEqual(local, remote)) {
		if (!isObject(local) || !isObject(remote)) {
			differences.push({ path, local, remote });
		} else {
			const allKeys = new Set([
				...Object.keys(local || {}),
				...Object.keys(remote || {}),
			]);

			for (const key of allKeys) {
				const currentPath = path ? `${path}.${key}` : key;
				//@ts-ignore
				const localValue = local?.[key];
				//@ts-ignore
				const remoteValue = remote?.[key];

				if (!isEqual(localValue, remoteValue)) {
					differences.push(
						...getDeepDifferences(localValue, remoteValue, currentPath),
					);
				}
			}
		}
	}

	return differences;
};

const logBoardDifferences = (localBoard: IBoard, remoteBoard: IBoard) => {
	const differences = getDeepDifferences(localBoard, remoteBoard);

	if (differences.length === 0) {
		console.log("No differences found between local and remote board");
		return;
	}

	console.log(
		`Found ${differences.length} differences between local and remote board:`,
	);
	console.table(
		differences.map((diff) => ({
			path: diff.path,
			localType: typeof diff.local,
			remoteType: typeof diff.remote,
			localValue:
				JSON.stringify(diff.local)?.slice(0, 100) +
				(JSON.stringify(diff.local)?.length > 100 ? "..." : ""),
			remoteValue:
				JSON.stringify(diff.remote)?.slice(0, 100) +
				(JSON.stringify(diff.remote)?.length > 100 ? "..." : ""),
		})),
	);

	differences.forEach((diff) => {
		console.groupCollapsed(`Path: ${diff.path}`);
		console.log("Local value:", diff.local);
		console.log("Remote value:", diff.remote);
		console.groupEnd();
	});
};
export class BoardState implements IBoardState {
	constructor(private readonly backend: TauriBackend) {}

	async getBoards(appId: string): Promise<IBoard[]> {
		let boards: IBoard[] = await invoke("get_app_boards", {
			appId: appId,
		});
		boards = Array.from(new Map(boards.map((b) => [b.id, b])).values());

		const isOffline = await this.backend.isOffline(appId);

		if (isOffline) {
			return boards;
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			console.warn(
				"Profile, auth or query client not set. Returning local boards only.",
			);
			return boards;
		}

		const promise = injectDataFunction(
			async () => {
				const mergedBoards = new Map<string, IBoard>();
				const remoteData = await fetcher<IBoard[]>(
					this.backend.profile!,
					`apps/${appId}/board`,
					{
						method: "GET",
					},
					this.backend.auth,
				);

				for (const board of boards) {
					mergedBoards.set(board.id, board);
				}

				for (const board of remoteData) {
					if (!isEqual(board, mergedBoards.get(board.id))) {
						console.log("Board data changed, updating local state:");
						await invoke("upsert_board", {
							appId: appId,
							boardId: board.id,
							name: board.name,
							description: board.description,
							boardData: board,
						});
					}

					mergedBoards.set(board.id, board);
				}

				return Array.from(mergedBoards.values());
			},
			this,
			this.backend.queryClient,
			this.getBoards,
			[appId],
			[],
			boards,
		);

		this.backend.backgroundTaskHandler(promise);

		return boards;
	}
	async getCatalog(): Promise<INode[]> {
		const nodes: INode[] = await invoke("get_catalog");
		return nodes;
	}
	async getBoard(
		appId: string,
		boardId: string,
		version?: [number, number, number],
	): Promise<IBoard> {
		const board: IBoard = await invoke("get_board", {
			appId: appId,
			boardId: boardId,
			version: version,
		});

		const isOffline = await this.backend.isOffline(appId);

		if (
			isOffline ||
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			return board;
		}

		const getOfflineSyncCommands =
			this.backend.getOfflineSyncCommands.bind(this);
		const clearOfflineSyncCommands =
			this.backend.clearOfflineSyncCommands.bind(this);

		const promise = injectDataFunction(
			async () => {
				const unsyncedCommands = await getOfflineSyncCommands(appId, boardId);
				for (const commandSync of unsyncedCommands) {
					try {
						// Only sync commands up to a week old
						if (
							commandSync.createdAt.getTime() <
							Date.now() - 7 * 24 * 60 * 60 * 1000
						)
							await fetcher(
								this.backend.profile!,
								`apps/${appId}/board/${boardId}`,
								{
									method: "POST",
									body: JSON.stringify({
										commands: commandSync.commands,
									}),
								},
								this.backend.auth,
							);
						console.log(
							"Executed offline sync command:",
							commandSync.commandId,
						);
						await clearOfflineSyncCommands(
							commandSync.commandId,
							appId,
							boardId,
						);
					} catch (e) {
						console.warn("Failed to execute offline sync command:", e);
					}
				}

				const remoteData = await fetcher<IBoard>(
					this.backend.profile!,
					`apps/${appId}/board/${boardId}`,
					{
						method: "GET",
					},
					this.backend.auth,
				);

				if (!remoteData) {
					throw new Error("Failed to fetch board data");
				}

				remoteData.updated_at = board.updated_at;

				if (!isEqual(remoteData, board) && typeof version === "undefined") {
					console.log("Board Missmatch, updating local state:");

					logBoardDifferences(board, remoteData);

					await invoke("upsert_board", {
						appId: appId,
						boardId: boardId,
						name: remoteData.name,
						description: remoteData.description,
						boardData: remoteData,
					});
				} else {
					console.log("Board data is up to date, no update needed.");
				}

				return remoteData;
			},
			this,
			this.backend.queryClient,
			this.getBoard,
			[appId, boardId, version],
			[],
			board,
		);

		this.backend.backgroundTaskHandler(promise);

		return board;
	}
	async createBoardVersion(
		appId: string,
		boardId: string,
		versionType: IVersionType,
	): Promise<[number, number, number]> {
		const newVersion: [number, number, number] = await invoke(
			"create_board_version",
			{
				appId: appId,
				boardId: boardId,
				versionType: versionType,
			},
		);

		const isOffline = await this.backend.isOffline(appId);
		if (
			isOffline ||
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			return newVersion;
		}

		const promise = injectDataFunction(
			async () => {
				const remoteData = await fetcher<[number, number, number]>(
					this.backend.profile!,
					`apps/${appId}/board/${boardId}`,
					{
						method: "PATCH",
						body: JSON.stringify({
							version_type: versionType,
						}),
					},
					this.backend.auth,
				);

				return remoteData;
			},
			this,
			this.backend.queryClient,
			this.createBoardVersion,
			[appId, boardId, versionType],
			[],
			newVersion,
		);

		this.backend.backgroundTaskHandler(promise);

		return newVersion;
	}
	async getBoardVersions(
		appId: string,
		boardId: string,
	): Promise<[number, number, number][]> {
		const boardVersions: [number, number, number][] = await invoke(
			"get_board_versions",
			{
				appId: appId,
				boardId: boardId,
			},
		);

		const isOffline = await this.backend.isOffline(appId);
		if (
			isOffline ||
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			return boardVersions;
		}

		const promise = injectDataFunction(
			async () => {
				const remoteData = await fetcher<[number, number, number][]>(
					this.backend.profile!,
					`apps/${appId}/board/${boardId}/version`,
					{
						method: "GET",
					},
					this.backend.auth,
				);

				return remoteData;
			},
			this,
			this.backend.queryClient,
			this.getBoardVersions,
			[appId, boardId],
			[],
			boardVersions,
		);

		this.backend.backgroundTaskHandler(promise);

		return boardVersions;
	}
	async deleteBoard(appId: string, boardId: string): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);
		if (isOffline) {
			await invoke("delete_app_board", {
				appId: appId,
				boardId: boardId,
			});
			return;
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			throw new Error(
				"Profile, auth or query client not set. Cannot delete board.",
			);
		}

		await fetcher(
			this.backend.profile,
			`apps/${appId}/board/${boardId}`,
			{
				method: "DELETE",
			},
			this.backend.auth,
		);

		await invoke("delete_app_board", {
			appId: appId,
			boardId: boardId,
		});
	}
	async getOpenBoards(): Promise<[string, string, string][]> {
		const boards: [string, string, string][] = await invoke("get_open_boards");
		return boards;
	}
	async getBoardSettings(): Promise<IConnectionMode> {
		const profile: ISettingsProfile = await invoke("get_current_profile");
		return (
			profile?.hub_profile.settings?.connection_mode ?? IConnectionMode.Default
		);
	}

	async executeBoard(
		appId: string,
		boardId: string,
		payload: IRunPayload,
		streamState?: boolean,
		eventId?: (id: string) => void,
		cb?: (event: IIntercomEvent[]) => void,
	): Promise<ILogMetadata | undefined> {
		const channel = new Channel<IIntercomEvent[]>();
		let closed = false;
		let foundRunId = false;

		const isOffline = await this.backend.isOffline(appId);
		let credentials = undefined;

		if (!isOffline && this.backend.auth && this.backend.profile) {
			try {
				credentials = await fetcher(
					this.backend.profile,
					`apps/${appId}/invoke/presign`,
					{
						method: "GET",
					},
					this.backend.auth,
				);
			} catch (e) {
				console.warn(e);
			}
		}

		channel.onmessage = (events: IIntercomEvent[]) => {
			if (closed) {
				console.warn("Channel closed, ignoring events");
				return;
			}

			if (!foundRunId && events.length > 0 && eventId) {
				const runId_event = events.find(
					(event) => event.event_type === "run_initiated",
				);

				if (runId_event) {
					const runId = runId_event.payload.run_id;
					eventId(runId);
					foundRunId = true;
				}
			}

			if (cb) cb(events);
		};

		const token = this.backend.auth?.user?.access_token;
		console.log("Using token:", token);

		console.dir({
			id: this.backend.auth?.user?.id_token,
			access: this.backend.auth?.user?.access_token,
		});

		const metadata: ILogMetadata | undefined = await invoke("execute_board", {
			appId: appId,
			boardId: boardId,
			payload: payload,
			events: channel,
			streamState: streamState,
			credentials,
			token,
		});

		closed = true;

		return metadata;
	}

	async listRuns(
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
		const runs: ILogMetadata[] = await invoke("list_runs", {
			appId: appId,
			boardId: boardId,
			nodeId: nodeId,
			from: from,
			to: to,
			status: status,
			limit: limit,
			offset: offset,
			lastMeta: lastMeta,
		});
		return runs;
	}

	async queryRun(
		logMeta: ILogMetadata,
		query: string,
		offset?: number,
		limit?: number,
	): Promise<ILog[]> {
		const runs: ILog[] = await invoke("query_run", {
			logMeta: logMeta,
			query: query,
			limit: limit,
			offset: offset,
		});
		return runs;
	}

	async undoBoard(appId: string, boardId: string, commands: IGenericCommand[]) {
		const isOffline = await this.backend.isOffline(appId);

		if (isOffline) {
			await invoke("undo_board", {
				appId: appId,
				boardId: boardId,
				commands: commands,
			});
			return;
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			toast.error("Undo only works when you are online.");
			throw new Error(
				"Profile, auth or query client not set. Cannot push board update.",
			);
		}

		await fetcher(
			this.backend.profile,
			`apps/${appId}/board/${boardId}/undo`,
			{
				method: "PATCH",
				body: JSON.stringify({
					commands: commands,
				}),
			},
			this.backend.auth,
		);
	}
	async redoBoard(appId: string, boardId: string, commands: IGenericCommand[]) {
		const isOffline = await this.backend.isOffline(appId);

		if (isOffline) {
			await invoke("redo_board", {
				appId: appId,
				boardId: boardId,
				commands: commands,
			});
			return;
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			toast.error("Undo only works when you are online.");
			throw new Error(
				"Profile, auth or query client not set. Cannot push board update.",
			);
		}

		await fetcher(
			this.backend.profile,
			`apps/${appId}/board/${boardId}/redo`,
			{
				method: "PATCH",
				body: JSON.stringify({
					commands: commands,
				}),
			},
			this.backend.auth,
		);
	}

	async upsertBoard(
		appId: string,
		boardId: string,
		name: string,
		description: string,
		logLevel: ILogLevel,
		stage: IExecutionStage,
		template?: IBoard,
	) {
		const isOffline = await this.backend.isOffline(appId);

		if (isOffline) {
			await invoke("upsert_board", {
				appId: appId,
				boardId: boardId,
				name: name,
				description: description,
				logLevel: logLevel,
				stage: stage,
				template: template,
			});
			return;
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			throw new Error(
				"Profile, auth or query client not set. Cannot push board update.",
			);
		}

		const boardUpdate = await fetcher<{ id: string }>(
			this.backend.profile,
			`apps/${appId}/board/${boardId}`,
			{
				method: "PUT",
				body: JSON.stringify({
					name: name,
					description: description,
					log_level: logLevel,
					stage: stage,
					template: template,
				}),
			},
			this.backend.auth,
		);

		if (!boardUpdate?.id) {
			throw new Error("Failed to update board");
		}
	}

	async closeBoard(boardId: string) {
		await invoke("close_board", {
			boardId: boardId,
		});
	}

	async executeCommand(
		appId: string,
		boardId: string,
		command: IGenericCommand,
	): Promise<IGenericCommand> {
		const returnValue = await invoke<IGenericCommand>("execute_command", {
			appId: appId,
			boardId: boardId,
			command: command,
		});

		const isOffline = await this.backend.isOffline(appId);
		if (isOffline) {
			return returnValue;
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			await this.backend.pushOfflineSyncCommand(appId, boardId, [command]);
			return returnValue;
		}

		try {
			await fetcher(
				this.backend.profile,
				`apps/${appId}/board/${boardId}`,
				{
					method: "POST",
					body: JSON.stringify({
						commands: [command],
					}),
				},
				this.backend.auth,
			);
		} catch (error) {
			console.error("Failed to push command to server:", error);
			await this.backend.pushOfflineSyncCommand(appId, boardId, [command]);
		}

		return returnValue;
	}

	async executeCommands(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	): Promise<IGenericCommand[]> {
		const returnValue = await invoke<IGenericCommand[]>("execute_commands", {
			appId: appId,
			boardId: boardId,
			commands: commands,
		});

		const isOffline = await this.backend.isOffline(appId);
		if (isOffline) {
			return returnValue;
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			await this.backend.pushOfflineSyncCommand(appId, boardId, commands);
			return returnValue;
		}

		try {
			const pushTask = await fetcher(
				this.backend.profile,
				`apps/${appId}/board/${boardId}`,
				{
					method: "POST",
					body: JSON.stringify({
						commands: commands,
					}),
				},
				this.backend.auth,
			);
		} catch (error) {
			console.error("Failed to push commands to server:", error);
			await this.backend.pushOfflineSyncCommand(appId, boardId, commands);
		}

		return returnValue;
	}
}
