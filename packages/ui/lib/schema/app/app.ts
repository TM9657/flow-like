export interface IApp {
	authors: string[];
	bits: string[];
	boards: string[];
	created_at: ISystemTime;
	id: string;
	meta: { [key: string]: IBitMeta };
	updated_at: ISystemTime;
	[property: string]: any;
}

export interface ISystemTime {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
}

export interface IBitMeta {
	description: string;
	long_description: string;
	name: string;
	tags: string[];
	use_case: string;
	[property: string]: any;
}
