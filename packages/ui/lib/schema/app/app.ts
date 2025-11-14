export interface IApp {
	authors: string[];
	avg_rating?: number | null;
	bits: string[];
	boards: string[];
	changelog?: null | string;
	created_at: ICreatedAt;
	download_count: number;
	events: string[];
	execution_mode: IExecutionMode;
	frontend?: null | IFrontendObject;
	id: string;
	interactions_count: number;
	price?: number | null;
	primary_category?: IPrimaryCategoryEnum | null;
	rating_count: number;
	rating_sum: number;
	relevance_score?: number | null;
	secondary_category?: IPrimaryCategoryEnum | null;
	status: IStatus;
	templates: string[];
	updated_at: ICreatedAt;
	version?: null | string;
	visibility: IVisibility;
	[property: string]: any;
}

export interface ICreatedAt {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
}

export enum IExecutionMode {
	Any = "Any",
	Local = "Local",
	Remote = "Remote",
}

export interface IFrontendObject {
	landing_page?: null | string;
	[property: string]: any;
}

export enum IPrimaryCategoryEnum {
	Anime = "Anime",
	Business = "Business",
	Communication = "Communication",
	Education = "Education",
	Entertainment = "Entertainment",
	Finance = "Finance",
	FoodAndDrink = "FoodAndDrink",
	Games = "Games",
	Health = "Health",
	Lifestyle = "Lifestyle",
	Music = "Music",
	News = "News",
	Other = "Other",
	Photography = "Photography",
	Productivity = "Productivity",
	Shopping = "Shopping",
	Social = "Social",
	Sports = "Sports",
	Travel = "Travel",
	Utilities = "Utilities",
	Weather = "Weather",
}

export enum IStatus {
	Active = "Active",
	Archived = "Archived",
	Inactive = "Inactive",
}

export enum IVisibility {
	Offline = "Offline",
	Private = "Private",
	Prototype = "Prototype",
	Public = "Public",
	PublicRequestAccess = "PublicRequestAccess",
}
