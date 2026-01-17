import { create } from "zustand";

import type { IAIState } from "./backend-state/ai-state";
import type { IApiState } from "./backend-state/api-state";
import type { IAppState } from "./backend-state/app-state";
import type { IBitState } from "./backend-state/bit-state";
import type { IBoardState } from "./backend-state/board-state";
import type { IDatabaseState } from "./backend-state/db-state";
import type { IEventState } from "./backend-state/event-state";
import type { IHelperState } from "./backend-state/helper-state";
import type { IPageState } from "./backend-state/page-state";
import type { IRegistryState } from "./backend-state/registry-state";
import type { IRoleState } from "./backend-state/role-state";
import type { IAppRouteState } from "./backend-state/route-state";
import type { ISinkState, IEventRegistration } from "./backend-state/sink-state";
import type { IStorageState } from "./backend-state/storage-state";
import type { ITeamState } from "./backend-state/team-state";
import type { ITemplateState } from "./backend-state/template-state";
import type { IUserState } from "./backend-state/user-state";
import type { IWidgetState } from "./backend-state/widget-state";
import type { IProfile } from "../types";

export * from "./backend-state/api-state";
export * from "./backend-state/empty-states/index";
export * from "./backend-state/registry-state";
export * from "./backend-state/idb-route-state";
export type {
	IAIState,
	IApiState,
	IAppState,
	IAppRouteState,
	IBitState,
	IBoardState,
	IEventState,
	IHelperState,
	IPageState,
	IRegistryState,
	IRoleState,
	ISinkState,
	IEventRegistration,
	IStorageState,
	ITeamState,
	ITemplateState,
	IUserState,
	IWidgetState,
};

export type { SinkType } from "./backend-state/sink-state";

export type {
	IPage,
	IWidgetRef,
	PageContent,
	PageLayoutType,
	PageMeta,
	PageListItem,
	CanvasSettings,
	WidgetInstance,
} from "./backend-state/page-state";

export type { IRouteMapping } from "./backend-state/route-state";

export type {
	CustomizationOption,
	CustomizationType,
	IWidget,
	ValidationRule,
	Version,
	VersionType,
} from "./backend-state/widget-state";

export type {
	IBackendRole,
	IInvite,
	IInviteLink,
	IJoinRequest,
	IMember,
	IStorageItemActionResult,
	INotification,
	INotificationsOverview,
	INotificationEvent,
	NotificationType,
} from "./backend-state/types";
export * from "./backend-state/db-state";
export type {
	IUserWidgetInfo,
	IUserTemplateInfo,
} from "./backend-state/user-state";

export interface ICapabilities {
	needsSignIn: boolean;
	canHostLlamaCPP: boolean;
	canHostEmbeddings: boolean;
	canExecuteLocally: boolean;
}

export interface IBackendState {
	appState: IAppState;
	apiState: IApiState;
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
	widgetState: IWidgetState;
	pageState: IPageState;
	routeState: IAppRouteState;
	registryState: IRegistryState;
	/** Sink state for managing active event sinks (desktop only) */
	sinkState?: ISinkState;

	/** Optional runtime profile (desktop/mobile providers populate this). */
	profile?: IProfile;

	capabilities(): ICapabilities;
	isOffline(appId: string): Promise<boolean>;
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
