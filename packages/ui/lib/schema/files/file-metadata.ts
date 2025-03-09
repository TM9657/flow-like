export interface IFileMetadata {
	access_time?: null | ISystemTime;
	album?: null | string;
	artist?: null | string;
	author?: null | string;
	bitrate?: number | null;
	camera_make?: null | string;
	camera_model?: null | string;
	creation_time?: null | ISystemTime;
	creator?: null | string;
	duration?: number | null;
	file_extension: string;
	file_name: string;
	file_path: string;
	file_size: number;
	frame_rate?: number | null;
	genre?: null | string;
	keywords?: null | string;
	location?: number[] | null;
	mime_type: string;
	modification_time?: null | ISystemTime;
	orientation?: number | null;
	pages?: number | null;
	producer?: null | string;
	resolution?: number[] | null;
	sample_rate?: number | null;
	subject?: null | string;
	title?: null | string;
	track_number?: number | null;
	track_title?: null | string;
	year?: number | null;
	[property: string]: any;
}

export interface ISystemTime {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
}
