import { invoke } from "@tauri-apps/api/core";
import type {
	IProfile,
	ISettingsProfile,
	IUserState,
} from "@tm9657/flow-like-ui";
import type {
	INotificationsOverview,
	IUserLookup,
} from "@tm9657/flow-like-ui/state/backend-state/types";
import { fetcher } from "../../lib/api";
import type { TauriBackend } from "../tauri-provider";

export class UserState implements IUserState {
	constructor(private readonly backend: TauriBackend) {}
	async lookupUser(userId: string): Promise<IUserLookup> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<IUserLookup>(
			this.backend.profile,
			`user/lookup/${userId}`,
			{
				method: "GET",
			},
			this.backend.auth,
		);

		return result;
	}
	async searchUsers(query: string): Promise<IUserLookup[]> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<IUserLookup[]>(
			this.backend.profile,
			`user/search/${query}`,
			{
				method: "GET",
			},
			this.backend.auth,
		);

		return result;
	}
	async getNotifications(): Promise<INotificationsOverview> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<INotificationsOverview>(
			this.backend.profile,
			`user/notifications`,
			{
				method: "GET",
			},
			this.backend.auth,
		);

		return result;
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
}
