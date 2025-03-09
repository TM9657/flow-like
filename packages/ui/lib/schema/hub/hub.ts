export interface IHub {
	dependencies: string[];
	description: string;
	domain: string;
	icon: string;
	name: string;
	thumbnail: string;
	[property: string]: any;
}
