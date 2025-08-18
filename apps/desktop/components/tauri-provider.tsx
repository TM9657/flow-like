"use client";
import { invoke } from "@tauri-apps/api/core";

import { createId } from "@paralleldrive/cuid2";
import {
	type IAppState,
	IAppVisibility,
	type IBackendState,
	type IBit,
	type IBitState,
	type IBoardState,
	type IEventState,
	type IGenericCommand,
	type IHelperState,
	type IProfile,
	type IRoleState,
	type IStorageState,
	type ITeamState,
	type ITemplateState,
	type IUserState,
	LoadingScreen,
	type QueryClient,
	offlineSyncDB,
	useBackend,
	useBackendStore,
	useDownloadManager,
	useInvoke,
	useQueryClient,
} from "@tm9657/flow-like-ui";
import type { ICommandSync } from "@tm9657/flow-like-ui/lib";
import type { IAIState } from "@tm9657/flow-like-ui/state/backend-state/ai-state";
import { useCallback, useEffect, useRef, useTransition } from "react";
import type { AuthContextProps } from "react-oidc-context";
import { appsDB } from "../lib/apps-db";
import { AiState } from "./tauri-provider/ai-state";
import { AppState } from "./tauri-provider/app-state";
import { BitState } from "./tauri-provider/bit-state";
import { BoardState } from "./tauri-provider/board-state";
import { EventState } from "./tauri-provider/event-state";
import { HelperState } from "./tauri-provider/helper-state";
import { RoleState } from "./tauri-provider/role-state";
import { StorageState } from "./tauri-provider/storage-state";
import { TeamState } from "./tauri-provider/team-state";
import { TemplateState } from "./tauri-provider/template-state";
import { UserState } from "./tauri-provider/user-state";

// One-time resume guards for the whole app session
declare global {
	// eslint-disable-next-line no-var
	var __FL_DL_RESUME_PROMISE__: Promise<void> | undefined;
	// eslint-disable-next-line no-var
	var __FL_DL_RESUMED__: boolean | undefined;
}

export class TauriBackend implements IBackendState {
	appState: IAppState;
	bitState: IBitState;
	boardState: IBoardState;
	eventState: IEventState;
	helperState: IHelperState;
	roleState: IRoleState;
	storageState: IStorageState;
	teamState: ITeamState;
	templateState: ITemplateState;
	userState: IUserState;
	aiState: IAIState;

	constructor(
		public readonly backgroundTaskHandler: (task: Promise<any>) => void,
		public queryClient?: QueryClient,
		public auth?: AuthContextProps,
		public profile?: IProfile,
	) {
		this.appState = new AppState(this);
		this.bitState = new BitState(this);
		this.boardState = new BoardState(this);
		this.eventState = new EventState(this);
		this.helperState = new HelperState(this);
		this.roleState = new RoleState(this);
		this.storageState = new StorageState(this);
		this.teamState = new TeamState(this);
		this.templateState = new TemplateState(this);
		this.userState = new UserState(this);
		this.aiState = new AiState(this);
	}

	pushProfile(profile: IProfile) {
		this.profile = profile;
	}

	pushAuthContext(auth: AuthContextProps) {
		this.auth = auth;
	}

	pushQueryClient(queryClient: QueryClient) {
		this.queryClient = queryClient;
	}

	async isOffline(appId: string): Promise<boolean> {
		const status = await appsDB.visibility.get(appId);
		if (typeof status !== "undefined") {
			return status.visibility === IAppVisibility.Offline;
		}
		return true;
	}

	async pushOfflineSyncCommand(
		appId: string,
		boardId: string,
		commands: IGenericCommand[],
	) {
		console.log("Pushing offline sync command", { appId, boardId, commands });
		await offlineSyncDB.commands.put({
			commandId: createId(),
			appId: appId,
			boardId: boardId,
			commands: commands,
			createdAt: new Date(),
		});
	}

	async getOfflineSyncCommands(
		appId: string,
		boardId: string,
	): Promise<ICommandSync[]> {
		const commands = await offlineSyncDB.commands
			.where({
				appId: appId,
				boardId: boardId,
			})
			.toArray();

		return commands.toSorted(
			(a, b) => a.createdAt.getTime() - b.createdAt.getTime(),
		);
	}

	async clearOfflineSyncCommands(
		commandId: string,
		appId: string,
		boardId: string,
	): Promise<void> {
		await offlineSyncDB.commands.delete(commandId);
	}

	async uploadSignedUrl(
		signedUrl: string,
		file: File,
		completedFiles: number,
		totalFiles: number,
		onProgress?: (progress: number) => void,
	): Promise<void> {
		const formData = new FormData();
		formData.append("file", file);

		await new Promise<void>((resolve, reject) => {
			const xhr = new XMLHttpRequest();

			xhr.upload.addEventListener("progress", (event) => {
				if (event.lengthComputable) {
					const fileProgress = event.loaded / event.total;
					const totalProgress =
						((completedFiles + fileProgress) / totalFiles) * 100;
					onProgress?.(totalProgress);
				}
			});

			xhr.addEventListener("load", () => {
				if (xhr.status >= 200 && xhr.status < 300) {
					resolve();
				} else {
					reject(new Error(`Upload failed with status: ${xhr.status}`));
				}
			});

			xhr.addEventListener("error", () => {
				reject(new Error("Upload failed"));
			});

			xhr.open("PUT", signedUrl);
			xhr.setRequestHeader(
				"Content-Type",
				file.type || "application/octet-stream",
			);
			xhr.send(file);
		});

		onProgress?.((completedFiles / totalFiles) * 100);
	}
}

export function TauriProvider({
	children,
}: Readonly<{ children: React.ReactNode }>) {
	const queryClient = useQueryClient();
	const { backend, setBackend } = useBackendStore();
	const { setDownloadBackend, download } = useDownloadManager();
	const [isPending, startTransition] = useTransition();

	// Safety to avoid state updates after unmount during resume
	const mountedRef = useRef(true);
	useEffect(() => {
		mountedRef.current = true;
		return () => {
			mountedRef.current = false;
		};
	}, []);

	// Resume downloads exactly once per app lifecycle
	const resumeDownloads = useCallback(async () => {
		if (globalThis.__FL_DL_RESUME_PROMISE__) {
			await globalThis.__FL_DL_RESUME_PROMISE__;
			return;
		}

		globalThis.__FL_DL_RESUME_PROMISE__ = (async () => {
			try {
				// Small defer to let backend wire up
				await new Promise((r) => setTimeout(r, 100));

				// First-time resume: ask backend for items to resume
				if (!globalThis.__FL_DL_RESUMED__) {
					console.time("Resuming Downloads (init_downloads)");
					const downloads = await invoke<{ [key: string]: IBit }>(
						"init_downloads",
					);
					console.timeEnd("Resuming Downloads (init_downloads)");
					const items = Object.keys(downloads).map((bitId) => downloads[bitId]);

					if (items.length) {
						console.time("Resuming download requests");
						await Promise.allSettled(items.map((item) => download(item)));
						console.timeEnd("Resuming download requests");
					}
					globalThis.__FL_DL_RESUMED__ = true;
					return;
				}

				// Subsequent mounts: only hydrate UI state, do not re-trigger init
				console.time("Hydrating Downloads (get_downloads)");
				const snapshot = await invoke<{ [key: string]: IBit }>("get_downloads");
				console.timeEnd("Hydrating Downloads (get_downloads)");
				const items = Object.keys(snapshot).map((bitId) => snapshot[bitId]);

				// Calling download(item) is safe because the manager de-duplicates in-flight calls.
				if (items.length) {
					await Promise.allSettled(items.map((item) => download(item)));
				}
			} catch (e) {
				console.error("resumeDownloads failed:", e);
			}
		})();

		await globalThis.__FL_DL_RESUME_PROMISE__;
	}, [download]);

	// Kick off resume when a backend is set
	useEffect(() => {
		if (!backend) return;
		startTransition(() => {
			void resumeDownloads();
		});
	}, [backend, resumeDownloads]);

	// Keep backend references in sync
	useEffect(() => {
		if (backend && backend instanceof TauriBackend && queryClient) {
			backend.pushQueryClient(queryClient);
		}
	}, [backend, queryClient]);

	useEffect(() => {
		console.time("TauriProvider Initialization");
		const backend = new TauriBackend((promise) => {
			promise
				.then((result) => {
					console.log("Background task completed:", result);
				})
				.catch((error) => {
					console.error("Background task failed:", error);
				});
		}, queryClient);
		console.timeEnd("TauriProvider Initialization");

		console.time("Setting Backend");
		setBackend(backend);
		console.timeEnd("Setting Backend");

		console.time("Setting Download Backend");
		setDownloadBackend(backend);
		console.timeEnd("Setting Download Backend");
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	if (!backend) {
		return <LoadingScreen progress={50} />;
	}

	return (
		<>
			{backend && <ProfileSyncer />}
			{children}
		</>
	);
}

function ProfileSyncer() {
	const backend = useBackend();
	const profile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
		true,
	);

	useEffect(() => {
		if (profile.data && backend instanceof TauriBackend) {
			backend.pushProfile(profile.data);
		}
	}, [profile.data, backend]);

	return null;
}
