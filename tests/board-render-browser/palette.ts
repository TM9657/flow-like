import { oklchToHex } from "../../packages/ui/lib/appearance/appearance-theme";
import type { Rgba } from "./types";

// Keep theme resolution independent of Canvas parsing and pixel readback.
export function paletteColor(value: string): Rgba {
	const hex = oklchToHex(value) ?? value.trim();
	const match = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(hex);
	if (!match) throw new Error(`Unsupported board palette colour: ${value}`);
	const rgb = Number.parseInt(match[1], 16);
	return [
		(rgb >> 16) & 255,
		(rgb >> 8) & 255,
		rgb & 255,
		match[2] ? Number.parseInt(match[2], 16) : 255,
	];
}
