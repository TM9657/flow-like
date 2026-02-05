"use client";
import { invoke } from "@tauri-apps/api/core";
import { readFile } from "@tauri-apps/plugin-fs";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";

import { createId } from "@paralleldrive/cuid2";
import {
	type IApiKeyState,
	type IApiState,
	type IAppRouteState,
	type IAppState,
	IAppVisibility,
	type IBackendState,
	type IBit,
	type IBitState,
	type IBoardState,
	type ICapabilities,
	type IDatabaseState,
	type IEventState,
	type IGenericCommand,
	type IHelperState,
	type IPageState,
	type IProfile,
	type IRegistryState,
	type IRoleState,
	type ISalesState,
	type ISinkState,
	type IStorageState,
	type ITeamState,
	type ITemplateState,
	type IUserState,
	type IWidgetState,
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
import { ApiKeyState } from "./tauri-provider/api-key-state";
import { TauriApiState } from "./tauri-provider/api-state";
import { AppState } from "./tauri-provider/app-state";
import { BitState } from "./tauri-provider/bit-state";
import { BoardState } from "./tauri-provider/board-state";
import { DatabaseState } from "./tauri-provider/db-state";
import { EventState } from "./tauri-provider/event-state";
import { HelperState } from "./tauri-provider/helper-state";
import { PageState } from "./tauri-provider/page-state";
import { RegistryState } from "./tauri-provider/registry-state";
import { RoleState } from "./tauri-provider/role-state";
import { RouteState } from "./tauri-provider/route-state";
import { SalesState } from "./tauri-provider/sales-state";
import { SinkState } from "./tauri-provider/sink-state";
import { StorageState } from "./tauri-provider/storage-state";
import { TeamState } from "./tauri-provider/team-state";
import { TemplateState } from "./tauri-provider/template-state";
import { UserState } from "./tauri-provider/user-state";
import { WidgetState } from "./tauri-provider/widget-state";

// One-time resume guards for the whole app session
declare global {
	// eslint-disable-next-line no-var
	var __FL_DL_RESUME_PROMISE__: Promise<void> | undefined;
	// eslint-disable-next-line no-var
	var __FL_DL_RESUMED__: boolean | undefined;
}

export class TauriBackend implements IBackendState {
	appState: IAppState;
	apiState: IApiState;
	apiKeyState: IApiKeyState;
	bitState: IBitState;
	boardState: IBoardState;
	eventState: IEventState;
	helperState: IHelperState;
	roleState: IRoleState;
	routeState: IAppRouteState;
	storageState: IStorageState;
	teamState: ITeamState;
	templateState: ITemplateState;
	userState: IUserState;
	aiState: IAIState;
	dbState: IDatabaseState;
	widgetState: IWidgetState;
	pageState: IPageState;
	registryState: IRegistryState;
	sinkState: ISinkState;
	salesState: ISalesState;

	private _apiState: TauriApiState;

	constructor(
		public readonly backgroundTaskHandler: (task: Promise<any>) => void,
		public queryClient?: QueryClient,
		public auth?: AuthContextProps,
		public profile?: IProfile,
	) {
		this._apiState = new TauriApiState();
		this.apiState = this._apiState;
		this.apiKeyState = new ApiKeyState(this);
		this.appState = new AppState(this);
		this.bitState = new BitState(this);
		this.boardState = new BoardState(this);
		this.eventState = new EventState(this);
		this.helperState = new HelperState(this);
		this.roleState = new RoleState(this);
		this.routeState = new RouteState(this);
		this.storageState = new StorageState(this);
		this.teamState = new TeamState(this);
		this.templateState = new TemplateState(this);
		this.userState = new UserState(this);
		this.aiState = new AiState(this);
		this.dbState = new DatabaseState(this);
		this.widgetState = new WidgetState(this);
		this.pageState = new PageState(this);
		this.registryState = new RegistryState(this);
		this.sinkState = new SinkState();
		this.salesState = new SalesState(this);
	}

	capabilities(): ICapabilities {
		const isIos = /iPad|iPhone|iPod/.test(navigator.userAgent);

		return {
			needsSignIn: isIos,
			canHostLlamaCPP: !isIos,
			canHostEmbeddings: true,
			canExecuteLocally: true,
		};
	}

	pushProfile(profile: IProfile) {
		this.profile = profile;
	}

	pushAuthContext(auth: AuthContextProps) {
		this.auth = auth;
		this._apiState.setAuth(auth);
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
					reject(
						new Error(
							`Upload failed with status ${xhr.status}: ${xhr.statusText}`,
						),
					);
				}
			});

			xhr.addEventListener("error", () => {
				reject(new Error("Upload failed: Network error (possible CORS issue)"));
			});

			xhr.open("PUT", signedUrl);
			xhr.setRequestHeader(
				"Content-Type",
				file.type || "application/octet-stream",
			);

			// Azure Blob Storage requires x-ms-blob-type header
			if (signedUrl.includes(".blob.core.windows.net")) {
				xhr.setRequestHeader("x-ms-blob-type", "BlockBlob");
			}

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

	// Extract auth state for dependency tracking
	const isAuthenticated =
		backend instanceof TauriBackend && backend.auth?.isAuthenticated;
	const accessToken =
		backend instanceof TauriBackend
			? backend.auth?.user?.access_token
			: undefined;
	const hubUrl = profile.data?.hub;

	useEffect(() => {
		if (profile.data && backend instanceof TauriBackend) {
			backend.pushProfile(profile.data);
		}
	}, [profile.data, backend]);

	// Sync profiles to backend when authenticated
	useEffect(() => {
		if (!(backend instanceof TauriBackend)) return;
		if (!isAuthenticated || !hubUrl || !accessToken) return;

		const isLocalPath = (path?: string | null): boolean => {
			if (!path) return false;
			return !path.startsWith("http://") && !path.startsWith("https://");
		};

		const getExtension = (path: string): string => {
			const ext = path.split(".").pop()?.toLowerCase() ?? "png";
			// Normalize common extensions
			if (ext === "jpeg") return "jpg";
			return ext;
		};

		const getContentType = (ext: string): string => {
			switch (ext) {
				case "webp":
					return "image/webp";
				case "jpg":
				case "jpeg":
					return "image/jpeg";
				case "gif":
					return "image/gif";
				case "svg":
					return "image/svg+xml";
				default:
					return "image/png";
			}
		};

		const uploadToSignedUrl = async (
			localPath: string,
			signedUrl: string,
		): Promise<boolean> => {
			try {
				const fileData = await readFile(localPath);
				const ext = getExtension(localPath);
				const uploadResponse = await tauriFetch(signedUrl, {
					method: "PUT",
					headers: {
						"Content-Type": getContentType(ext),
					},
					body: fileData,
				});
				return uploadResponse.ok;
			} catch (error) {
				console.warn("Failed to upload to signed URL:", error);
				return false;
			}
		};

		const syncProfiles = async () => {
			try {
				console.log("Starting profile sync...");
				const localProfiles =
					await invoke<Record<string, { hub_profile: IProfile }>>(
						"get_profiles",
					);
				console.log("Local profiles:", Object.keys(localProfiles || {}).length);
				if (!localProfiles || Object.keys(localProfiles).length === 0) return;

				const visibilityRecords = await appsDB.visibility.toArray();
				const offlineAppIds = new Set(
					visibilityRecords
						.filter((v) => v.visibility === IAppVisibility.Offline)
						.map((v) => v.appId),
				);

				const baseUrl =
					hubUrl ?? process.env.NEXT_PUBLIC_API_URL ?? "api.flow-like.com";
				const protocol = profile.data?.secure === false ? "http" : "https";

				// Build profile data with upload extensions
				const profilesWithLocalImages: Map<
					string,
					{ icon?: string; thumbnail?: string }
				> = new Map();

				// Gather shortcuts for each profile
				const profileShortcuts: Map<string, any[]> = new Map();
				for (const p of Object.values(localProfiles)) {
					const profileId = p.hub_profile.id;
					if (profileId) {
						const shortcuts = await appsDB.shortcuts
							.where("profileId")
							.equals(profileId)
							.toArray();
						profileShortcuts.set(profileId, shortcuts);
					}
				}

				const profilesToSync = Object.values(localProfiles).map((p) => {
					const hubProfile = p.hub_profile;
					const filteredApps = hubProfile.apps?.filter(
						(app) => !offlineAppIds.has(app.app_id),
					);

					const hasLocalIcon = isLocalPath(hubProfile.icon);
					const hasLocalThumbnail = isLocalPath(hubProfile.thumbnail);

					if ((hasLocalIcon || hasLocalThumbnail) && hubProfile.id) {
						profilesWithLocalImages.set(hubProfile.id, {
							icon: hasLocalIcon ? hubProfile.icon! : undefined,
							thumbnail: hasLocalThumbnail ? hubProfile.thumbnail! : undefined,
						});
					}

					const shortcuts = hubProfile.id ? profileShortcuts.get(hubProfile.id) : undefined;

					return {
						id: hubProfile.id,
						name: hubProfile.name,
						description: hubProfile.description,
						icon_upload_ext: hasLocalIcon
							? getExtension(hubProfile.icon!)
							: undefined,
						thumbnail_upload_ext: hasLocalThumbnail
							? getExtension(hubProfile.thumbnail!)
							: undefined,
						interests: hubProfile.interests,
						tags: hubProfile.tags,
						theme: hubProfile.theme,
						bit_ids: hubProfile.bits,
						apps: filteredApps,
						shortcuts: shortcuts,
						hubs: hubProfile.hubs,
						settings: hubProfile.settings,
						createdAt: hubProfile.created,
						updatedAt: hubProfile.updated,
					};
				});

				console.log(
					"Profiles to sync:",
					profilesToSync.length,
					profilesToSync.map((p) => p.name),
				);
				if (profilesToSync.length === 0) return;

				const url = baseUrl.startsWith("http")
					? `${baseUrl}/api/v1/profile/sync`
					: `${protocol}://${baseUrl}/api/v1/profile/sync`;

				console.log("Syncing to URL:", url);

				const response = await tauriFetch(url, {
					method: "POST",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${accessToken}`,
					},
					body: JSON.stringify(profilesToSync),
				});

				if (!response.ok) {
					console.warn(
						"Profile sync failed:",
						response.status,
						await response.text(),
					);
					return;
				}

				type SyncResult = {
					synced: string[];
					created: Array<{
						local_id: string;
						server_id: string;
						icon_upload_url?: string;
						thumbnail_upload_url?: string;
					}>;
					updated: Array<{
						id: string;
						icon_upload_url?: string;
						thumbnail_upload_url?: string;
					}>;
					skipped: string[];
				};

				const result = (await response.json()) as SyncResult;
				console.log("Profile sync result:", result);

				// Upload images to the signed URLs returned by server
				for (const created of result.created) {
					const localImages = profilesWithLocalImages.get(created.local_id);
					if (localImages?.icon && created.icon_upload_url) {
						console.log("Uploading icon for new profile:", created.local_id);
						await uploadToSignedUrl(localImages.icon, created.icon_upload_url);
					}
					if (localImages?.thumbnail && created.thumbnail_upload_url) {
						console.log(
							"Uploading thumbnail for new profile:",
							created.local_id,
						);
						await uploadToSignedUrl(
							localImages.thumbnail,
							created.thumbnail_upload_url,
						);
					}
				}

				for (const updated of result.updated) {
					const localImages = profilesWithLocalImages.get(updated.id);
					if (localImages?.icon && updated.icon_upload_url) {
						console.log("Uploading icon for updated profile:", updated.id);
						await uploadToSignedUrl(localImages.icon, updated.icon_upload_url);
					}
					if (localImages?.thumbnail && updated.thumbnail_upload_url) {
						console.log("Uploading thumbnail for updated profile:", updated.id);
						await uploadToSignedUrl(
							localImages.thumbnail,
							updated.thumbnail_upload_url,
						);
					}
				}

				// Remap local profile IDs to server-assigned IDs
				for (const { local_id, server_id } of result.created) {
					await invoke("remap_profile_id", {
						localId: local_id,
						serverId: server_id,
					});

					// Also remap shortcuts profileId
					const shortcuts = await appsDB.shortcuts
						.where("profileId")
						.equals(local_id)
						.toArray();
					for (const shortcut of shortcuts) {
						await appsDB.shortcuts.update(shortcut.id, {
							profileId: server_id,
						});
					}
				}

				// After successful sync and uploads, fetch the updated profiles from backend
				// to get the final icon/thumbnail CUIDs and update local profiles with CDN URLs
				try {
					const profilesUrl = baseUrl.startsWith("http")
						? `${baseUrl}/api/v1/profile`
						: `${protocol}://${baseUrl}/api/v1/profile`;

					const profilesResponse = await tauriFetch(profilesUrl, {
						method: "GET",
						headers: {
							"Content-Type": "application/json",
							Authorization: `Bearer ${accessToken}`,
						},
					});

					if (profilesResponse.ok) {
						const onlineProfiles = (await profilesResponse.json()) as Array<{
							id: string;
							icon?: string;
							thumbnail?: string;
							shortcuts?: Array<{
								id: string;
								profileId: string;
								label: string;
								path: string;
								appId?: string;
								icon?: string;
								order: number;
								createdAt: string;
							}>;
						}>;

						// Get CDN base URL from platform config or use hub URL
						const cdnUrl =
							(await invoke<string | null>("get_platform_cdn_url").catch(
								() => null,
							)) ||
							(baseUrl.startsWith("http") ? baseUrl : `${protocol}://${baseUrl}`);

						// Build set of online profile IDs
						const onlineProfileIds = new Set(onlineProfiles.map((p) => p.id));

						// Delete local profiles that don't exist online anymore
						const currentLocalProfiles =
							await invoke<Record<string, { hub_profile: IProfile }>>(
								"get_profiles",
							);
						const currentProfileId = await invoke<string>("get_current_profile_id").catch(() => null);
						let deletedCurrentProfile = false;

						if (currentLocalProfiles) {
							for (const localProfileId of Object.keys(currentLocalProfiles)) {
								const localProfile = currentLocalProfiles[localProfileId];
								// Only delete if profile has online apps (is synced) and doesn't exist online
								const hasOnlineApps = localProfile.hub_profile.apps?.some(
									(app) => !offlineAppIds.has(app.app_id),
								);
								if (hasOnlineApps && !onlineProfileIds.has(localProfileId)) {
									console.log(
										"Deleting local profile that no longer exists online:",
										localProfileId,
									);

									// Check if we're deleting the current profile
									if (localProfileId === currentProfileId) {
										deletedCurrentProfile = true;
									}

									try {
										await invoke("delete_profile", {
											profileId: localProfileId,
										});
										// Also delete associated shortcuts
										await appsDB.shortcuts
											.where("profileId")
											.equals(localProfileId)
											.delete();
									} catch (error) {
										console.error(
											"Failed to delete local profile:",
											localProfileId,
											error,
										);
									}
								}
							}
						}

						// Handle edge cases after deletion
						const remainingProfiles = await invoke<Record<string, { hub_profile: IProfile }>>(
							"get_profiles",
						);

						if (!remainingProfiles || Object.keys(remainingProfiles).length === 0) {
							// No profiles left - redirect to onboarding
							console.log("No profiles remaining after sync, redirecting to onboarding");
							if (typeof window !== "undefined") {
								window.location.href = "/onboarding";
							}
							return;
						}

						if (deletedCurrentProfile) {
							// Current profile was deleted - switch to first available profile
							const firstProfileId = Object.keys(remainingProfiles)[0];
							console.log("Current profile was deleted, switching to:", firstProfileId);
							try {
								await invoke("set_current_profile", {
									profileId: firstProfileId,
								});
							} catch (error) {
								console.error("Failed to switch profile:", error);
							}
						}

						// Update local profiles with online icon/thumbnail URLs and sync shortcuts
						for (const onlineProfile of onlineProfiles) {
							// Sync shortcuts from backend to local IndexedDB
							if (onlineProfile.shortcuts) {
								const localShortcuts = await appsDB.shortcuts
									.where("profileId")
									.equals(onlineProfile.id)
									.toArray();

								const localShortcutMap = new Map(localShortcuts.map(s => [s.id, s]));
								const onlineShortcutMap = new Map(onlineProfile.shortcuts.map(s => [s.id, s]));

								// Add or update shortcuts from online
								for (const onlineShortcut of onlineProfile.shortcuts) {
									await appsDB.shortcuts.put(onlineShortcut);
								}

								// Remove shortcuts that don't exist online anymore
								for (const localShortcut of localShortcuts) {
									if (!onlineShortcutMap.has(localShortcut.id)) {
										await appsDB.shortcuts.delete(localShortcut.id);
									}
								}
							}

							if (onlineProfile.icon || onlineProfile.thumbnail) {
								const localProfile = localProfiles[onlineProfile.id];
								if (localProfile) {
									// Construct full CDN URL for icon (icon field contains the CUID)
									if (onlineProfile.icon) {
										localProfile.hub_profile.icon = `${cdnUrl}/media/profiles/${onlineProfile.id}/${onlineProfile.icon}.webp`;
									}

									// Construct full CDN URL for thumbnail
									if (onlineProfile.thumbnail) {
										localProfile.hub_profile.thumbnail = `${cdnUrl}/media/profiles/${onlineProfile.id}/${onlineProfile.thumbnail}.webp`;
									}

									// Update the local profile with new URLs
									await invoke("upsert_profile", {
										profile: localProfile,
									});
								}
							}
						}
						console.log("Local profiles updated with online CDN URLs");
					}
				} catch (error) {
					console.warn("Failed to fetch and update profile URLs:", error);
				}
			} catch (error) {
				console.warn("Failed to sync profiles to backend:", error);
			}
		};

		syncProfiles();
	}, [backend, isAuthenticated, accessToken, hubUrl, profile.data?.secure]);

	return null;
}
