import type { IMetadata } from "../../../lib";
import { nowSystemTime } from "../../../lib/time/now";
import type { IBackendState } from "../../../state/backend-state";
import type { CreatedArtifactJournal } from "../../flowpilot/board-edit-guard";
import { argBool, argString } from "./tool-arguments";

export interface CreateAppToolOptions {
	authenticated: boolean;
	conversationId?: string;
	requestId: string;
	journal: CreatedArtifactJournal;
	assertActive(): void;
	rememberTarget(appId: string): void;
	referenceApp(appId: string): void;
	invalidate(): void;
	handleUpgrade(error: unknown): boolean;
}

async function registerProfileApp(
	backend: Pick<IBackendState, "userState">,
	appId: string,
	assertActive: () => void,
): Promise<string | undefined> {
	try {
		const profile = await backend.userState.getSettingsProfile();
		if (!profile)
			return "No current profile is available to register the created app.";
		assertActive();
		await backend.userState.updateProfileApp(
			profile,
			{ app_id: appId, favorite: false, pinned: false },
			"Upsert",
		);
		return undefined;
	} catch (error) {
		return error instanceof Error ? error.message : String(error);
	}
}

/** Provision an app while keeping profile association and retry identity observable. */
export async function createAppTool(
	backend: Pick<IBackendState, "appState" | "userState">,
	args: Record<string, unknown>,
	options: CreateAppToolOptions,
): Promise<Record<string, unknown>> {
	const name = argString(args, "name").trim();
	if (!name)
		return {
			status: "error",
			message: `create_app requires a \`name\`. Derive a short name from the request (e.g. "Weather App") and call create_app once with it. Do not call it again with empty arguments.`,
		};
	const description = argString(args, "description");
	const idempotencyKey =
		argString(args, "idempotency_key") || argString(args, "idempotencyKey");
	const creationConversationId = options.conversationId;
	const creationIdentity = creationConversationId
		? {
				conversationId: creationConversationId,
				toolName: "create_app",
				instruction: `${name}\n${description}`,
				...(idempotencyKey ? { idempotencyKey } : {}),
			}
		: undefined;
	const journaled = creationIdentity
		? options.journal.find(creationIdentity)
		: undefined;
	if (journaled?.artifacts.appId) {
		const existingAppId = journaled.artifacts.appId;
		try {
			options.assertActive();
			await backend.appState.getAppAuthoritative(existingAppId);
		} catch (error) {
			return {
				status: "unknown",
				app_id: existingAppId,
				code: "journaled_app_unverified",
				message: `The prior app cannot be verified: ${error instanceof Error ? error.message : String(error)}. Inspect that app before retrying creation.`,
			};
		}
		// A prior response can be partial after app creation. Retry the profile association,
		// not the app creation, before reporting success for a replay.
		const profileError = await registerProfileApp(
			backend,
			existingAppId,
			options.assertActive,
		);
		options.invalidate();
		options.rememberTarget(existingAppId);
		options.referenceApp(existingAppId);
		return {
			status: profileError ? "partial" : "ok",
			app_id: existingAppId,
			name,
			already_created: true,
			provisioning: { app_created: true, profile_registered: !profileError },
			...(profileError
				? { code: "app_profile_incomplete", message: profileError }
				: {}),
			note: "An app for this exact request was already created earlier in this conversation; its app_id is returned instead of creating a duplicate. Continue building on this app_id. Only if the user truly wants a second, separate app, call create_app again with a distinct `idempotency_key`.",
		};
	}
	const meta: IMetadata = {
		name,
		description,
		tags: [],
		use_case: "",
		created_at: nowSystemTime(),
		updated_at: nowSystemTime(),
		preview_media: [],
	};
	// Default to a cloud app when signed in (mirrors the library's create dialog),
	// let the model force local via online:false, but never attempt online without
	// auth: createApp's remote PUT would fail without a token.
	const authenticated = options.authenticated;
	const online = (argBool(args, "online") ?? authenticated) && authenticated;
	let app: Awaited<ReturnType<typeof backend.appState.createApp>>;
	try {
		options.assertActive();
		app = await backend.appState.createApp(meta, [], online);
	} catch (error) {
		console.error("[global-tool-bridge] create_app: creation failed", error);
		if (options.handleUpgrade(error)) {
			return {
				status: "error",
				message: `The user's plan does not allow creating another online project; an upgrade dialog was shown to them. Either wait for the user to upgrade, or offer to create the app locally instead (online:false).`,
			};
		}
		return {
			status: "error",
			message: `create_app failed: ${error instanceof Error ? error.message : String(error)}`,
		};
	}
	// Persist the known ID before secondary setup or UI callbacks can fail. This cannot
	// close a lost createApp response; server-side idempotency is still required for that.
	if (creationIdentity) {
		options.journal.record(
			creationIdentity,
			{ appId: app.id },
			options.requestId,
		);
	}
	// Associate the app with the current profile so it surfaces in list_apps
	// (which is profile-scoped) and the user's library, matching the other
	// create-app entry points.
	const profileError = await registerProfileApp(
		backend,
		app.id,
		options.assertActive,
	);
	options.invalidate();
	options.rememberTarget(app.id);
	options.referenceApp(app.id);
	return {
		status: profileError ? "partial" : "ok",
		app_id: app.id,
		name,
		online,
		provisioning: { app_created: true, profile_registered: !profileError },
		...(profileError
			? { code: "app_profile_incomplete", message: profileError }
			: {}),
	};
}
