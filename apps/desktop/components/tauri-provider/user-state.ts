import { invoke } from "@tauri-apps/api/core";
import type {
	IProfile,
	IProfileApp,
	ISettingsProfile,
	IUserState,
} from "@tm9657/flow-like-ui";
import type {
	INotificationsOverview,
	IUserLookup,
} from "@tm9657/flow-like-ui/state/backend-state/types";
import type {
	IBillingSession,
	IPricingResponse,
	ISubscribeRequest,
	ISubscribeResponse,
	IUserInfo,
	IUserUpdate,
} from "@tm9657/flow-like-ui/state/backend-state/user-state";
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

	async updateUser(data: IUserUpdate, avatar?: File): Promise<void> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		if (avatar) {
			data.avatar_extension = avatar.name.split(".").pop() || "";
		}

		const response = await fetcher<{ signed_url?: string }>(
			this.backend.profile,
			`user/info`,
			{
				method: "PUT",
				body: JSON.stringify(data),
			},
			this.backend.auth,
		);

		if (response.signed_url && avatar) {
			await fetch(response.signed_url, {
				method: "PUT",
				body: avatar,
				headers: {
					"Content-Type": avatar.type,
				},
			});
		}
	}

	async getInfo(): Promise<IUserInfo> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<IUserInfo>(
			this.backend.profile,
			`user/info`,
			{
				method: "GET",
			},
			this.backend.auth,
		);

		return result;
	}

	async updateProfileApp(
		profile: ISettingsProfile,
		app: IProfileApp,
		operation: "Upsert" | "Remove",
	): Promise<void> {
		await invoke("profile_update_app", {
			profile,
			app,
			operation,
		});
	}

	async createPAT(
		name: string,
		validUntil?: Date,
		permissions?: number,
	): Promise<{ pat: string; permission: number }> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const unix = validUntil
			? Math.floor(validUntil.getTime() / 1000)
			: undefined;

		const result = await fetcher<{
			pat: string;
			permission: number;
		}>(
			this.backend.profile,
			`user/pat`,
			{
				method: "PUT",
				body: JSON.stringify({ name, valid_until: unix, permissions }),
			},
			this.backend.auth,
		);

		return result;
	}

	async getPATs(): Promise<
		{
			id: string;
			name: string;
			created_at: string;
			valid_until: string | null;
			permission: number;
		}[]
	> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<
			{
				id: string;
				name: string;
				created_at: string;
				valid_until: string | null;
				permission: number;
			}[]
		>(
			this.backend.profile,
			`user/pat`,
			{
				method: "GET",
			},
			this.backend.auth,
		);

		return result;
	}

	async deletePAT(id: string): Promise<void> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		await fetcher(
			this.backend.profile,
			`user/pat`,
			{
				method: "DELETE",
				body: JSON.stringify({ id }),
			},
			this.backend.auth,
		);

		return;
	}

	async getPricing(): Promise<IPricingResponse> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<IPricingResponse>(
			this.backend.profile,
			"user/pricing",
			{ method: "GET" },
			this.backend.auth,
		);

		return result;
	}

	async createSubscription(
		request: ISubscribeRequest,
	): Promise<ISubscribeResponse> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<ISubscribeResponse>(
			this.backend.profile,
			"user/subscribe",
			{
				method: "POST",
				body: JSON.stringify(request),
			},
			this.backend.auth,
		);

		return result;
	}

	async getBillingSession(): Promise<IBillingSession> {
		if (!this.backend.profile || !this.backend.auth) {
			throw new Error("Profile or auth context not available");
		}

		const result = await fetcher<IBillingSession>(
			this.backend.profile,
			"user/billing",
			{ method: "GET" },
			this.backend.auth,
		);

		return result;
	}
}
