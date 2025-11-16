export interface IUpsertComment {
	comment: IComment;
	current_layer?: null | string;
	old_comment?: null | IComment;
	[property: string]: any;
}

export interface IComment {
	author?: null | string;
	color?: null | string;
	comment_type: ICommentType;
	content: string;
	coordinates: number[];
	hash?: number | null;
	height?: number | null;
	id: string;
	is_locked?: boolean | null;
	layer?: null | string;
	timestamp: ISystemTime;
	width?: number | null;
	z_index?: number | null;
	[property: string]: any;
}

export enum ICommentType {
	Image = "Image",
	Text = "Text",
	Video = "Video",
}

export interface ISystemTime {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
}
