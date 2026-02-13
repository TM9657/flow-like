import { invoke } from "@tauri-apps/api/core";
import type { IPage, IPageState, PageListItem } from "@tm9657/flow-like-ui";
import type { TauriBackend } from "../tauri-provider";

export class PageState implements IPageState {
	constructor(private readonly backend: TauriBackend) {}

	async getPages(appId: string, boardId?: string): Promise<PageListItem[]> {
		return invoke<PageListItem[]>("get_pages", {
			appId,
			boardId,
		});
	}

	async getPage(
		appId: string,
		pageId: string,
		boardId?: string,
	): Promise<IPage> {
		return invoke<IPage>("get_page", { appId, pageId, boardId });
	}

	async createPage(
		appId: string,
		pageId: string,
		name: string,
		route: string,
		boardId: string,
		title?: string,
	): Promise<IPage> {
		return invoke<IPage>("create_page", {
			appId,
			pageId,
			name,
			route,
			boardId,
			title,
		});
	}

	async updatePage(appId: string, page: IPage): Promise<void> {
		return invoke("update_page", { appId, page });
	}

	async deletePage(
		appId: string,
		pageId: string,
		boardId: string,
	): Promise<void> {
		return invoke("delete_page", { appId, pageId, boardId });
	}

	async getOpenPages(): Promise<[string, string, string][]> {
		return invoke<[string, string, string][]>("get_open_pages");
	}

	async closePage(pageId: string): Promise<void> {
		return invoke("close_page", { pageId });
	}
}
