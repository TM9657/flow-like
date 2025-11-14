export interface IProfile {
	apps?: IAppElement[] | null;
	bits: string[];
	created: string;
	description?: null | string;
	hub?: string;
	hubs?: string[];
	icon?: null | string;
	id?: string;
	interests?: string[];
	name: string;
	settings?: ISettings;
	tags?: string[];
	theme?: any;
	thumbnail?: null | string;
	updated: string;
	[property: string]: any;
}

export interface IAppElement {
	app_id: string;
	favorite: boolean;
	favorite_order?: number | null;
	pinned: boolean;
	pinned_order?: number | null;
	[property: string]: any;
}

export interface ISettings {
	connection_mode: IConnectionMode;
	[property: string]: any;
}

export enum IConnectionMode {
	Default = "default",
	Simplebezier = "simplebezier",
	Smoothstep = "smoothstep",
	Step = "step",
	Straight = "straight",
}
