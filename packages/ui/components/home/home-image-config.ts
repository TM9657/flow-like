import { safeHomeHref, textConfig } from "./home-content/config";

export type HomeImageReference =
	| { source: "url"; url: string }
	| { source: "storage"; appId: string; path: string };

export function homeImageSource(config: Record<string, unknown>) {
	return config.imageSource === "storage" ? "storage" : "url";
}

export function isHomeStorageImagePath(value: string): boolean {
	return (
		value.length > 0 &&
		value === value.trim() &&
		!value.startsWith("/") &&
		!value.endsWith("/") &&
		!value.includes("\\") &&
		!/^[a-z][a-z\d+.-]*:/i.test(value) &&
		!Array.from(value).some(
			(char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127,
		) &&
		value.split("/").every((part) => part && part !== "." && part !== "..")
	);
}

export function homeImageReference(
	config: Record<string, unknown>,
): HomeImageReference | undefined {
	if (homeImageSource(config) === "storage") {
		const appId = textConfig(config, "imageAppId").trim();
		const path = textConfig(config, "imagePath");
		return appId && isHomeStorageImagePath(path)
			? { source: "storage", appId, path }
			: undefined;
	}
	const url = safeHomeHref(textConfig(config, "imageUrl"));
	return url && /^(https?:|\/)/.test(url) ? { source: "url", url } : undefined;
}
