export interface ILogMetadata {
	app_id: string;
	board_id: string;
	end: number;
	log_level: number;
	logs?: number | null;
	node_id: string;
	nodes?: Array<Array<number | string>> | null;
	payload: number[];
	run_id: string;
	start: number;
	version: string;
	[property: string]: any;
}
