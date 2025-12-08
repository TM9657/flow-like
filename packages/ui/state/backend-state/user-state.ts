import type { IProfile, IProfileApp } from "../../lib";
import type { ISettingsProfile } from "../../types";
import type { INotificationsOverview, IUserLookup } from "./types";

export interface IUserUpdate {
	name?: string;
	description?: string;
	avatar_extension?: string;
	accepted_terms_version?: string;
	tutorial_completed?: boolean;
}

export interface IUserInfo {
	id: string;
	stripeId?: string;
	email?: string;
	username?: string;
	preferred_username?: string;
	name?: string;
	description?: string;
	avatar?: string;

	permission?: number;
	accepted_terms_version?: string;
	tutorial_completed?: boolean;

	status?: string;
	tier?: string;

	total_size?: number;

	created_at?: string;
	updated_at?: string;
}

export interface IPriceInfo {
	amount: number;
	currency: string;
	interval?: string;
}

export interface ITierInfo {
	name: string;
	product_id?: string;
	max_non_visible_projects: number;
	max_remote_executions: number;
	execution_tier: string;
	max_total_size: number;
	max_llm_cost: number;
	max_llm_calls?: number;
	llm_tiers: string[];
	price?: IPriceInfo;
}

export interface IPricingResponse {
	current_tier: string;
	tiers: Record<string, ITierInfo>;
}

export interface ISubscribeRequest {
	tier: string;
	success_url: string;
	cancel_url: string;
}

export interface ISubscribeResponse {
	checkout_url: string;
	session_id: string;
}

export interface IBillingSession {
	session_id: string;
	url: string;
}

export interface IUserState {
	lookupUser(userId: string): Promise<IUserLookup>;
	searchUsers(query: string): Promise<IUserLookup[]>;
	getNotifications(): Promise<INotificationsOverview>;
	getProfile(): Promise<IProfile>;
	getSettingsProfile(): Promise<ISettingsProfile>;
	updateUser(data: IUserUpdate, avatar?: File): Promise<void>;
	updateProfileApp(
		profile: ISettingsProfile,
		app: IProfileApp,
		operation: "Upsert" | "Remove",
	): Promise<void>;
	getInfo(): Promise<IUserInfo>;
	createPAT(
		name: string,
		validUntil?: Date,
		permissions?: number,
	): Promise<{ pat: string; permission: number }>;
	getPATs(): Promise<
		{
			id: string;
			name: string;
			created_at: string;
			valid_until: string | null;
			permission: number;
		}[]
	>;
	deletePAT(id: string): Promise<void>;
	getPricing(): Promise<IPricingResponse>;
	createSubscription(request: ISubscribeRequest): Promise<ISubscribeResponse>;
	getBillingSession(): Promise<IBillingSession>;
}
