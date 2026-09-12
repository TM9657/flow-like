import type { IComputeLeg } from "./schema/usage/tracking";

/** Unit formatting for the serverless sizing behind a runtime cost estimate. */
export function formatMemory(gigabytes: number): string {
	return gigabytes < 1
		? `${Math.round(gigabytes * 1024)} MB`
		: `${Number(gigabytes.toFixed(2))} GB`;
}

/** A leg's billed share of the run, as a whole percentage. */
export function formatDurationShare(share: number): string {
	return `${Math.round(share * 100)}%`;
}

/** `2 GB x86_64`, the part of a leg that carries no prose to translate. */
export function formatComputeLeg(leg: IComputeLeg): string {
	return `${formatMemory(leg.memoryGb)} ${leg.architecture}`;
}
