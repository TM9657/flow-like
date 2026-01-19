import type { IApp, IMetadata } from "../../../lib";

export interface TemplateInfo {
	appId: string;
	templateId: string;
	metadata?: IMetadata;
}

export interface AppWithTemplates {
	app: IApp;
	appMetadata?: IMetadata;
	templates: TemplateInfo[];
}

export interface FlowTemplateSelectorProps {
	onSelectTemplate: (appId: string, templateId: string) => Promise<void>;
	onDismiss?: () => void;
}
