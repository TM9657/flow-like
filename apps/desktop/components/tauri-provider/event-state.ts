import { Channel, invoke } from "@tauri-apps/api/core";
import {
	type IBoard,
	type IEvent,
	type IEventState,
	type IIntercomEvent,
	type ILogMetadata,
	type IOAuthProvider,
	type IRunPayload,
	type IVersionType,
	checkOAuthTokens,
	injectDataFunction,
	isEqual,
} from "@tm9657/flow-like-ui";
import { fetcher } from "../../lib/api";
import { oauthConsentStore, oauthTokenStore } from "../../lib/oauth-db";
import type { TauriBackend } from "../tauri-provider";

export class EventState implements IEventState {
	constructor(private readonly backend: TauriBackend) {}

	async getEvent(
		appId: string,
		eventId: string,
		version?: [number, number, number],
	): Promise<IEvent> {
		const event = await invoke<IEvent>("get_event", {
			appId: appId,
			eventId: eventId,
			version: version,
		});

		const isOffline = await this.backend.isOffline(appId);
		if (
			isOffline ||
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			return event;
		}

		const promise = injectDataFunction(
			async () => {
				let url = `apps/${appId}/events/${eventId}`;
				if (version) {
					url += `?version=${version.join("_")}`;
				}
				const remoteData = await fetcher<IEvent>(
					this.backend.profile!,
					url,
					{
						method: "GET",
					},
					this.backend.auth,
				);

				if (!remoteData) {
					throw new Error("Failed to fetch event data");
				}

				if (!isEqual(remoteData, event) && typeof version === "undefined") {
					await invoke("upsert_event", {
						appId: appId,
						event: remoteData,
						enforceId: true,
						offline: isOffline,
					});
				}

				return remoteData;
			},
			this,
			this.backend.queryClient,
			this.getEvent,
			[appId, eventId, version],
			[],
			event,
		);

		this.backend.backgroundTaskHandler(promise);
		return event;
	}
	async getEvents(appId: string): Promise<IEvent[]> {
		const events = await invoke<IEvent[]>("get_events", {
			appId: appId,
		});
		const isOffline = await this.backend.isOffline(appId);
		if (
			isOffline ||
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			return events;
		}

		const promise = injectDataFunction(
			async () => {
				const remoteData = await fetcher<IEvent[]>(
					this.backend.profile!,
					`apps/${appId}/events`,
					{
						method: "GET",
					},
					this.backend.auth,
				);

				for (const event of remoteData) {
					await invoke("upsert_event", {
						appId: appId,
						event: event,
						enforceId: true,
						offline: isOffline,
					});
				}

				return remoteData;
			},
			this,
			this.backend.queryClient,
			this.getEvents,
			[appId],
			[],
			events,
		);

		this.backend.backgroundTaskHandler(promise);
		return events;
	}
	async getEventVersions(
		appId: string,
		eventId: string,
	): Promise<[number, number, number][]> {
		const versions = await invoke<[number, number, number][]>(
			"get_event_versions",
			{
				appId: appId,
				eventId: eventId,
			},
		);

		const isOffline = await this.backend.isOffline(appId);
		if (
			isOffline ||
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			return versions;
		}

		const promise = injectDataFunction(
			async () => {
				const remoteData = await fetcher<[number, number, number][]>(
					this.backend.profile!,
					`apps/${appId}/events/${eventId}/versions`,
					{
						method: "GET",
					},
					this.backend.auth,
				);

				return remoteData;
			},
			this,
			this.backend.queryClient,
			this.getEventVersions,
			[appId, eventId],
			[],
			versions,
		);

		this.backend.backgroundTaskHandler(promise);
		return versions;
	}
	async upsertEvent(
		appId: string,
		event: IEvent,
		versionType?: IVersionType,
		personalAccessToken?: string,
	): Promise<IEvent> {
		const isOffline = await this.backend.isOffline(appId);
		if (isOffline) {
			return await invoke("upsert_event", {
				appId: appId,
				event: event,
				versionType: versionType,
				offline: isOffline,
				pat: personalAccessToken,
			});
		}
		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			throw new Error(
				"Profile, auth or query client not set. Cannot upsert event.",
			);
		}
		const response = await fetcher<IEvent>(
			this.backend.profile,
			`apps/${appId}/events/${event.id}`,
			{
				method: "PUT",
				body: JSON.stringify({
					event: event,
					version_type: versionType,
				}),
			},
			this.backend.auth,
		);
		await invoke("upsert_event", {
			appId: appId,
			event: response,
			versionType: versionType,
			enforceId: true,
			offline: isOffline,
			pat: personalAccessToken,
		});
		return response;
	}
	async deleteEvent(appId: string, eventId: string): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);
		if (isOffline) {
			await invoke("delete_event", {
				appId: appId,
				eventId: eventId,
			});
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			throw new Error(
				"Profile, auth or query client not set. Cannot delete event.",
			);
		}

		await fetcher(
			this.backend.profile,
			`apps/${appId}/events/${eventId}`,
			{
				method: "DELETE",
			},
			this.backend.auth,
		);
		await invoke("delete_event", {
			appId: appId,
			eventId: eventId,
		});
	}
	async validateEvent(
		appId: string,
		eventId: string,
		version?: [number, number, number],
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);
		if (isOffline) {
			return await invoke("validate_event", {
				appId: appId,
				eventId: eventId,
				version: version,
			});
		}

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			throw new Error(
				"Profile, auth or query client not set. Cannot validate event.",
			);
		}

		return await fetcher(
			this.backend.profile,
			`apps/${appId}/events/${eventId}/validate`,
			{
				method: "POST",
				body: JSON.stringify({
					version: version,
				}),
			},
			this.backend.auth,
		);
	}
	async upsertEventFeedback(
		appId: string,
		eventId: string,
		feedbackId: string,
		feedback: {
			rating: number;
			history?: any[];
			globalState?: Record<string, any>;
			localState?: Record<string, any>;
			comment?: string;
		},
	): Promise<string> {
		const isOffline = await this.backend.isOffline(appId);
		if (isOffline) return "";

		if (
			!this.backend.profile ||
			!this.backend.auth ||
			!this.backend.queryClient
		) {
			throw new Error(
				"Profile, auth or query client not set. Cannot upsert event feedback.",
			);
		}

		const response = await fetcher<{ feedback_id: string }>(
			this.backend.profile,
			`apps/${appId}/events/${eventId}/feedback`,
			{
				method: "PUT",
				body: JSON.stringify({
					rating: feedback.rating,
					context: {
						history: feedback.history,
						global_state: feedback.globalState,
						local_state: feedback.localState,
					},
					comment: feedback.comment,
					feedback_id: feedbackId,
				}),
			},
			this.backend.auth,
		);

		return response.feedback_id;
	}

	async executeEvent(
		appId: string,
		eventId: string,
		payload: IRunPayload,
		streamState?: boolean,
		onEventId?: (id: string) => void,
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

		// Collect OAuth tokens from event's board using shared helper
		let oauthTokens:
			| Record<
					string,
					{
						access_token: string;
						refresh_token?: string;
						expires_at?: number;
						token_type?: string;
					}
			  >
			| undefined;
		const event = await this.getEvent(appId, eventId);
		const board: IBoard = await invoke("get_board", {
			appId: appId,
			boardId: event.board_id,
			version: event.board_version,
		});
		const oauthResult = await checkOAuthTokens(board, oauthTokenStore);

		// Check consent for providers that have tokens but might not have consent for this app
		const consentedIds = await oauthConsentStore.getConsentedProviderIds(appId);
		const providersNeedingConsent: IOAuthProvider[] = [];

		// Add providers that are missing tokens
		providersNeedingConsent.push(...oauthResult.missingProviders);

		// Also add providers that have tokens but no consent for this specific app
		for (const provider of oauthResult.requiredProviders) {
			const hasToken = oauthResult.tokens[provider.id] !== undefined;
			const hasConsent = consentedIds.has(provider.id);

			if (hasToken && !hasConsent) {
				console.log(
					`[OAuth] Provider ${provider.id} has token but no consent for app ${appId}`,
				);
				providersNeedingConsent.push(provider);
			}
		}

		if (providersNeedingConsent.length > 0) {
			const error = new Error(
				`Missing OAuth authorization for: ${providersNeedingConsent.map((p) => p.name).join(", ")}`,
			);
			(error as any).missingProviders = providersNeedingConsent;
			(error as any).isOAuthError = true;
			throw error;
		}

		if (Object.keys(oauthResult.tokens).length > 0) {
			oauthTokens = oauthResult.tokens;
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
					onEventId?.(runId);
					foundRunId = true;
				}
			}

			if (cb) cb(events);
		};

		const token = this.backend.auth?.user?.access_token;
		console.log("Using token:", token);

		const metadata: ILogMetadata | undefined = await invoke("execute_event", {
			appId: appId,
			eventId: eventId,
			payload: payload,
			events: channel,
			streamState: streamState,
			credentials,
			token,
			oauthTokens,
		});

		closed = true;

		return metadata;
	}

	async cancelExecution(runId: string): Promise<void> {
		await invoke("cancel_execution", {
			runId: runId,
		});
	}

	async isEventSinkActive(eventId: string): Promise<boolean> {
		return await invoke<boolean>("is_event_sink_active", {
			eventId: eventId,
		});
	}
}
