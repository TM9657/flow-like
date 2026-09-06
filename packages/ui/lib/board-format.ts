/** Board data versions evolve independently of app releases and compiled artifacts. */
export const LEGACY_BOARD_FORMAT_VERSION = 1;
export const CURRENT_BOARD_FORMAT_VERSION = 2;
export const GEOMETRY_BOARD_FORMAT_VERSION = 2;
export const BOARD_FORMAT_HEADER = "x-flow-like-board-format";

export interface BoardFormatCapabilities {
	board_format_version: number;
}

/** Missing or malformed advertisements keep clients on the legacy format. */
export function boardFormatVersion(value: unknown): number {
	return typeof value === "number" &&
		Number.isInteger(value) &&
		value >= LEGACY_BOARD_FORMAT_VERSION &&
		value <= 0xffff_ffff
		? value
		: LEGACY_BOARD_FORMAT_VERSION;
}
