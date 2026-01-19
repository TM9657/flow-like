import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

// https://stackoverflow.com/questions/10420352/converting-file-size-in-bytes-to-human-readable-string
export function humanFileSize(bytes: number, si = false, dp = 1) {
	const thresh = si ? 1000 : 1024;

	if (Math.abs(bytes) < thresh) {
		return `${bytes} B`;
	}

	const units = si
		? ["kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"]
		: ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
	let u = -1;
	let value = bytes;
	const r = 10 ** dp;

	do {
		value /= thresh;
		++u;
	} while (
		Math.round(Math.abs(value) * r) / r >= thresh &&
		u < units.length - 1
	);

	return `${value.toFixed(dp)} ${units[u]}`;
}

const SAFE_URL_PROTOCOLS = ["http:", "https:", "blob:", "data:"];

/** Sanitize image URL to prevent XSS - only allow safe protocols */
export function sanitizeImageUrl(
	url: string | undefined,
	fallback: string,
): string {
	if (!url) return fallback;
	try {
		const parsed = new URL(url, window.location.origin);
		if (SAFE_URL_PROTOCOLS.includes(parsed.protocol)) {
			return url;
		}
		return fallback;
	} catch {
		return fallback;
	}
}
