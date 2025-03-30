"use client";
import { invoke } from "@tauri-apps/api/core";
import { type Event, type UnlistenFn, listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
	type IApp,
	type IBackendState,
	type IBit,
	type IBitPack,
	type IBitTypes,
	type IBoard,
	type IDownloadProgress,
	type IExecutionStage,
	type IFileMetadata,
	type IGeneric,
	type ILogLevel,
	type INode,
	type IProfile,
	type IRun,
	type IRunUpdateEvent,
	type ISettingsProfile,
	useBackendStore,
	useDownloadManager,
} from "@tm9657/flow-like-ui";
import { useEffect, useState } from "react";

export class TauriBackend implements IBackendState {
	async getCatalog(): Promise<INode[]> {
		const nodes: INode[] = await invoke("get_catalog");
		return nodes;
	}

	async getBoard(appId: string, boardId: string): Promise<IBoard> {
		const board: IBoard = await invoke("get_board", {
			appId: appId,
			boardId: boardId,
		});
		return board;
	}

	async getOpenBoards(): Promise<[string, string][]> {
		const boards: [string, string][] = await invoke("get_open_boards");
		return boards;
	}

	async getBoardSettings(): Promise<"straight" | "step" | "simpleBezier"> {
		const profile: ISettingsProfile = await invoke("get_current_profile");
		return profile.flow_settings.connection_mode;
	}

	async createRun(
		appId: string,
		boardId: string,
		startIds: string[],
	): Promise<string> {
		const runId: string = await invoke("create_run", {
			appId: appId,
			boardId: boardId,
			startIds: startIds,
		});
		return runId;
	}

	async executeRun(
		appId: string,
		runId: string,
		cb?: (event: IRunUpdateEvent[]) => void,
	) {
		const listenEvent = await listen(
			`run:${runId}`,
			(event: Event<IRunUpdateEvent[]>) => {
				if (cb) cb(event.payload);
			},
		);

		await invoke("execute_run", {
			appId: appId,
			runId: runId,
		});

		listenEvent();
	}

	async getRun(appId: string, runId: string): Promise<IRun> {
		const run: IRun = await invoke("get_run", {
			appId: appId,
			runId: runId,
		});
		return run;
	}

	async finalizeRun(appId: string, runId: string) {
		await invoke("finalize_run", {
			appId: appId,
			runId: runId,
		});
	}

	async undoBoard(appId: string, boardId: string) {
		await invoke("undo_board", {
			appId: appId,
			boardId: boardId,
		});
	}

	async redoBoard(appId: string, boardId: string) {
		await invoke("redo_board", {
			appId: appId,
			boardId: boardId,
		});
	}

	async updateBoardMeta(
		appId: string,
		boardId: string,
		name: string,
		description: string,
		logLevel: ILogLevel,
		stage: IExecutionStage,
	) {
		await invoke("update_board_meta", {
			appId: appId,
			boardId: boardId,
			name: name,
			description: description,
			logLevel: logLevel,
			stage: stage,
		});
	}

	async closeBoard(boardId: string) {
		await invoke("close_board", {
			boardId: boardId,
		});
	}

	async getProfile(): Promise<IProfile> {
		const profile: ISettingsProfile = await invoke("get_current_profile");
		if (profile.hub_profile === undefined) {
			throw new Error("Profile not found");
		}
		return profile.hub_profile;
	}

	async getSettingsProfile(): Promise<ISettingsProfile> {
		const profile: ISettingsProfile = await invoke("get_current_profile");
		return profile;
	}

	async executeCommand(
		appId: string,
		boardId: string,
		command: IGeneric,
		append: boolean,
	) {
		await invoke("execute_command", {
			appId: appId,
			boardId: boardId,
			command: command,
			append: append,
		});
	}

	async getPathMeta(path: string): Promise<IFileMetadata[]> {
		return await invoke("get_path_meta", {
			path: path,
		});
	}

	async openFileOrFolderMenu(
		multiple: boolean,
		directory: boolean,
		recursive: boolean,
	): Promise<string[] | string | undefined> {
		return (
			(await open({
				multiple: multiple,
				directory: directory,
				recursive: recursive,
			})) ?? undefined
		);
	}

	async downloadBit(
		bit: IBit,
		pack: IBitPack,
		cb?: (progress: IDownloadProgress[]) => void,
	): Promise<IBit[]> {
		const unlistenFn: UnlistenFn[] = [];

		for (const deps of pack.bits) {
			unlistenFn.push(
				await listen(
					`download:${deps.hash}`,
					(event: Event<IDownloadProgress[]>) => {
						if (cb) cb(event.payload);
					},
				),
			);
		}

		const bits: IBit[] = await invoke("download_bit", {
			bit: bit,
		});

		for (const unlisten of unlistenFn) {
			unlisten();
		}

		return bits;
	}

	async deleteBit(bit: IBit): Promise<void> {
		throw new Error("Method not implemented.");
	}

	async addBit(bit: IBit, profile: ISettingsProfile): Promise<void> {
		await invoke("add_bit", {
			bit: bit,
			profile: profile,
		});
	}

	async removeBit(bit: IBit, profile: ISettingsProfile): Promise<void> {
		await invoke("remove_bit", {
			bit: bit,
			profile: profile,
		});
	}

	async getBitsByCategory(type: IBitTypes): Promise<IBit[]> {
		throw new Error("Method not implemented.");
	}

	async getBitSize(bit: IBit): Promise<number> {
		return await invoke("get_bit_size", {
			bit: bit,
		});
	}

	async getInstalledBit(bits: IBit[]): Promise<IBit[]> {
		return await invoke("get_installed_bit", {
			bits: bits,
		});
	}

	async getPackFromBit(bit: IBit): Promise<{ bits: IBit[] }> {
		return await invoke("get_pack_from_bit", {
			bit: bit,
		});
	}

	async getPackSize(bits: IBit[]): Promise<number> {
		const size: number = await invoke("get_pack_size", {
			bits: bits,
		});
		return size;
	}

	async isBitInstalled(bit: IBit): Promise<boolean> {
		return await invoke("is_bit_installed", {
			bit: bit,
		});
	}

	async getApp(appId: string): Promise<IApp> {
		return await invoke("get_app", {
			appId: appId,
		});
	}

	async getApps(): Promise<IApp[]> {
		return await invoke("get_apps");
	}

	async getBit(id: string, hub?: string): Promise<IBit> {
		return await invoke("get_bit_by_id", {
			bit: id,
			hub: hub,
		});
	}

	async getBoards(appId: string): Promise<IBoard[]> {
		const boards: IBoard[] = await invoke("get_app_boards", {
			appId: appId,
		});
		return boards;
	}
}

export function TauriProvider({
	children,
}: Readonly<{ children: React.ReactNode }>) {
	const [loaded, setLoaded] = useState(false);
	const { setBackend } = useBackendStore();
	const { setDownloadBackend, download } = useDownloadManager();

	async function resumeDownloads() {
		const downloads = await invoke<{ [key: string]: IBit }>("init_downloads");
		const items = Object.keys(downloads).map((bitId) => {
			const bit: IBit = downloads[bitId];
			return bit;
		});

		const download_requests = items.map((item) => {
			return download(item);
		});

		await Promise.allSettled([...download_requests]);
	}

	useEffect(() => {
		(async () => {
			const backend = new TauriBackend();
			setBackend(backend);
			setDownloadBackend(backend);
			await resumeDownloads();
			setLoaded(true);
		})();
	}, []);

	if (!loaded) {
		return <p>Loading...</p>;
	}

	return children;
}
