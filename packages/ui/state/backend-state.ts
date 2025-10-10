import { create } from "zustand";

import type { IAIState } from "./backend-state/ai-state";
import type { IAppState } from "./backend-state/app-state";
import type { IBitState } from "./backend-state/bit-state";
import type { IBoardState } from "./backend-state/board-state";
import type { IDatabaseState } from "./backend-state/db-state";
import type { IEventState } from "./backend-state/event-state";
import type { IHelperState } from "./backend-state/helper-state";
import type { IRoleState } from "./backend-state/role-state";
import type { IStorageState } from "./backend-state/storage-state";
import type { ITeamState } from "./backend-state/team-state";
import type { ITemplateState } from "./backend-state/template-state";
import type { IUserState } from "./backend-state/user-state";

export * from "./backend-state/empty-states/index";

export type {
	IAIState,
	IAppState,
	IBitState,
	IBoardState,
	IEventState,
	IHelperState,
	IRoleState,
	IStorageState,
	ITeamState,
	ITemplateState,
	IUserState,
};

export type {
	IBackendRole,
	IInvite,
	IInviteLink,
	IJoinRequest,
	IMember,
	IStorageItemActionResult,
} from "./backend-state/types";
export * from "./backend-state/db-state";

export interface ICapabilities {
	needsSignIn: boolean;
	canHostLlamaCPP: boolean;
	canHostEmbeddings: boolean;
	canExecuteLocally: boolean;
}

export interface IBackendState {
	appState: IAppState;
	bitState: IBitState;
	boardState: IBoardState;
	userState: IUserState;
	teamState: ITeamState;
	roleState: IRoleState;
	storageState: IStorageState;
	templateState: ITemplateState;
	helperState: IHelperState;
	eventState: IEventState;
	aiState: IAIState;
	dbState: IDatabaseState;

	capabilities(): ICapabilities;
}

interface BackendStoreState {
	backend: IBackendState | null;
	setBackend: (backend: IBackendState) => void;
}

export const useBackendStore = create<BackendStoreState>((set) => ({
	backend: null,
	setBackend: (backend: IBackendState) => set({ backend }),
}));

export function useBackend(): IBackendState {
	const backend = useBackendStore((state) => state.backend);
	if (!backend) throw new Error("Backend not initialized");
	return backend;
}
