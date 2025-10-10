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
	IUserInfo,
	IUserUpdate,
} from "@tm9657/flow-like-ui/state/backend-state/user-state";

export class EmptyUserState implements IUserState {
	lookupUser(userId: string): Promise<IUserLookup> {
		throw new Error("Method not implemented.");
	}
	searchUsers(query: string): Promise<IUserLookup[]> {
		throw new Error("Method not implemented.");
	}
	getNotifications(): Promise<INotificationsOverview> {
		throw new Error("Method not implemented.");
	}
	getProfile(): Promise<IProfile> {
		throw new Error("Method not implemented.");
	}
	getSettingsProfile(): Promise<ISettingsProfile> {
		throw new Error("Method not implemented.");
	}
	updateUser(data: IUserUpdate, avatar?: File): Promise<void> {
		throw new Error("Method not implemented.");
	}
	getInfo(): Promise<IUserInfo> {
		throw new Error("Method not implemented.");
	}
	updateProfileApp(
		profile: ISettingsProfile,
		app: IProfileApp,
		operation: "Upsert" | "Remove",
	): Promise<void> {
		throw new Error("Method not implemented.");
	}

	createPAT(name: string, validUntil?: Date, permissions?: number): Promise<{pat: string, permission: number}> {
		throw new Error("Method not implemented.");
	}

	getPATs(): Promise<{id: string, name: string, created_at: string, valid_until: string | null, permission: number}[]> {
		throw new Error("Method not implemented.");
	}

	deletePAT(id: string): Promise<void> {
		throw new Error("Method not implemented.");
	}
}
