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
	createPAT(name: string, validUntil?: Date, permissions?: number): Promise<{pat: string, permission: number}>;
	getPATs(): Promise<{id: string, name: string, created_at: string, valid_until: string | null, permission: number}[]>;
	deletePAT(id: string): Promise<void>;
}
