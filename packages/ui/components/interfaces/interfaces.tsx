import type { JSX, ReactElement, RefObject } from "react";
import type { IEvent, IEventPayload, INode } from "../../lib";

export interface IToolBarActions {
	pushToolbarElements: (elements: ReactElement[]) => void;
}

export interface ISidebarActions {
	pushSidebar: (sidebar?: ReactElement) => void;
	toggleOpen: () => void;
	isMobile: () => boolean;
	isOpen: () => boolean;
}

export interface IUseInterfaceProps {
	appId: string;
	event: IEvent;
	config?: Partial<IEventPayload>;
	toolbarRef?: RefObject<IToolBarActions | null>;
	sidebarRef?: RefObject<ISidebarActions | null>;
}

export interface IConfigInterfaceProps {
	isEditing: boolean;
	appId: string;
	boardId: string;
	nodeId: string;
	node: INode;
	config: Partial<IEventPayload>;
	onConfigUpdate: (payload: IEventPayload) => void;
}

export type IEventMapping = Record<
	string,
	{
		configs: Record<string, Partial<IEventPayload>>;
		eventTypes: string[];
		defaultEventType: string;
		useInterfaces: Record<
			string,
			(props: IUseInterfaceProps) => JSX.Element | null
		>;
		configInterfaces: Record<
			string,
			(props: IConfigInterfaceProps) => JSX.Element | null
		>;
		withSink: string[]
	}
>;
