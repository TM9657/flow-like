import { invoke } from "@tauri-apps/api/core";
import type {
	IMetadata,
	IWidget,
	IWidgetState,
	Version,
	VersionType,
} from "@tm9657/flow-like-ui";
import type { TauriBackend } from "../tauri-provider";

export class WidgetState implements IWidgetState {
	constructor(private readonly backend: TauriBackend) {}

	async getWidgets(
		appId: string,
		language?: string,
	): Promise<[string, string, IMetadata | undefined][]> {
		const widgets = await invoke<IWidget[]>("get_widgets", { appId });
		const result: [string, string, IMetadata | undefined][] = [];

		for (const widget of widgets) {
			let metadata: IMetadata | undefined;
			try {
				metadata = await this.getWidgetMeta(appId, widget.id, language);
			} catch {
				metadata = undefined;
			}
			result.push([appId, widget.id, metadata]);
		}

		return result;
	}

	async getWidget(
		appId: string,
		widgetId: string,
		version?: Version,
	): Promise<IWidget> {
		return invoke<IWidget>("get_widget", { appId, widgetId, version });
	}

	async createWidget(
		appId: string,
		widgetId: string,
		name: string,
		description?: string,
	): Promise<IWidget> {
		return invoke<IWidget>("create_widget", {
			appId,
			widgetId,
			name,
			description,
		});
	}

	async updateWidget(appId: string, widget: IWidget): Promise<void> {
		return invoke("update_widget", { appId, widget });
	}

	async deleteWidget(appId: string, widgetId: string): Promise<void> {
		return invoke("delete_widget", { appId, widgetId });
	}

	async createWidgetVersion(
		appId: string,
		widgetId: string,
		versionType: VersionType,
	): Promise<Version> {
		return invoke<Version>("create_widget_version", {
			appId,
			widgetId,
			versionType,
		});
	}

	async getWidgetVersions(appId: string, widgetId: string): Promise<Version[]> {
		return invoke<Version[]>("get_widget_versions", { appId, widgetId });
	}

	async getOpenWidgets(): Promise<[string, string, string][]> {
		return invoke<[string, string, string][]>("get_open_widgets");
	}

	async closeWidget(widgetId: string): Promise<void> {
		return invoke("close_widget", { widgetId });
	}

	async getWidgetMeta(
		appId: string,
		widgetId: string,
		language?: string,
	): Promise<IMetadata> {
		return invoke<IMetadata>("get_widget_meta", { appId, widgetId, language });
	}

	async pushWidgetMeta(
		appId: string,
		widgetId: string,
		metadata: IMetadata,
		language?: string,
	): Promise<void> {
		return invoke("push_widget_meta", { appId, widgetId, metadata, language });
	}
}
