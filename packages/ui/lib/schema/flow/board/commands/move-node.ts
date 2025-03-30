export interface IMoveNode {
	from_coordinates?: number[] | null;
	node_id: string;
	to_coordinates: number[];
	[property: string]: any;
}
