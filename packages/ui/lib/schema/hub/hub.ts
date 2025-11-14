export interface IHub {
	app?: null | string;
	authentication?: null | IAuthenticationObject;
	cdn?: null | string;
	contact: IContact;
	default_user_plan?: null | string;
	description: string;
	domain: string;
	environment: IEnvironment;
	features: IFeatures;
	hubs: string[];
	icon?: null | string;
	legal_notice: string;
	lookup?: ILookup;
	max_users_prototype?: number | null;
	name: string;
	privacy_policy: string;
	provider?: null | string;
	region?: null | string;
	signaling?: string[] | null;
	terms_of_service: string;
	thumbnail?: null | string;
	tiers: { [key: string]: ITierValue };
	[property: string]: any;
}

export interface IAuthenticationObject {
	oauth2?: null | IOauth2Object;
	openid?: null | IOpenidObject;
	variant: string;
	[property: string]: any;
}

export interface IOauth2Object {
	authorization_endpoint: string;
	client_id: string;
	token_endpoint: string;
	[property: string]: any;
}

export interface IOpenidObject {
	authority?: null | string;
	client_id?: null | string;
	cognito?: null | ICognitoObject;
	discovery_url?: null | string;
	jwks_url: string;
	post_logout_redirect_uri?: null | string;
	proxy?: null | IProxyObject;
	redirect_uri?: null | string;
	response_type?: null | string;
	scope?: null | string;
	user_info_url?: null | string;
	[property: string]: any;
}

export interface ICognitoObject {
	user_pool_id: string;
	[property: string]: any;
}

export interface IProxyObject {
	authorize?: null | string;
	enabled: boolean;
	revoke?: null | string;
	token?: null | string;
	userinfo?: null | string;
	[property: string]: any;
}

export interface IContact {
	email: string;
	name: string;
	url: string;
	[property: string]: any;
}

export enum IEnvironment {
	Development = "Development",
	Production = "Production",
	Staging = "Staging",
}

export interface IFeatures {
	admin_interface: boolean;
	ai_act: boolean;
	flow_hosting: boolean;
	governance: boolean;
	model_hosting: boolean;
	premium: boolean;
	unauthorized_read: boolean;
	[property: string]: any;
}

export interface ILookup {
	additional_information: boolean;
	avatar: boolean;
	created_at: boolean;
	description: boolean;
	email: boolean;
	name: boolean;
	preferred_username: boolean;
	username: boolean;
	[property: string]: any;
}

export interface ITierValue {
	execution_tier: string;
	llm_tiers: string[];
	max_llm_calls?: number | null;
	max_llm_cost: number;
	max_non_visible_projects: number;
	max_remote_executions: number;
	max_total_size: number;
	[property: string]: any;
}
