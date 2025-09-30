import type {
	IApp,
	IAppCategory,
	IAppState,
	IAppVisibility,
	IBoard,
	IMetadata,
} from "@tm9657/flow-like-ui";
import type { IAppSearchSort } from "@tm9657/flow-like-ui/lib/schema/app/app-search-query";
import type { IMediaItem } from "../app-state";

export class EmptyAppState implements IAppState {
	createApp(
		metadata: IMetadata,
		bits: string[],
		online: boolean,
		template?: IBoard,
	): Promise<IApp> {
		throw new Error("Method not implemented.");
	}
	deleteApp(appId: string): Promise<void> {
		throw new Error("Method not implemented.");
	}
	searchApps(
		id?: string,
		query?: string,
		language?: string,
		category?: IAppCategory,
		author?: string,
		sort?: IAppSearchSort,
		tag?: string,
		offset?: number,
		limit?: number,
	): Promise<[IApp, IMetadata | undefined][]> {
		throw new Error("Method not implemented.");
	}
	getApps(): Promise<[IApp, IMetadata | undefined][]> {
		throw new Error("Method not implemented.");
	}
	getApp(appId: string): Promise<IApp> {
		throw new Error("Method not implemented.");
	}
	updateApp(app: IApp): Promise<void> {
		throw new Error("Method not implemented.");
	}
	getAppMeta(appId: string, language?: string): Promise<IMetadata> {
		throw new Error("Method not implemented.");
	}
	pushAppMeta(
		appId: string,
		metadata: IMetadata,
		language?: string,
	): Promise<void> {
		throw new Error("Method not implemented.");
	}
	pushAppMedia(
		appId: string,
		item: IMediaItem,
		file: File,
		language?: string,
	): Promise<void> {
		throw new Error("Method not implemented.");
	}
	changeAppVisibility(
		appId: string,
		visibility: IAppVisibility,
	): Promise<void> {
		throw new Error("Method not implemented.");
	}

	requestJoinApp(appId: string, comment?: string): Promise<void> {
		throw new Error("Method not implemented.");
	}
}
