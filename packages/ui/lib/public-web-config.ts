/** Public container settings shared by web and UI API consumers. Never secrets. */
export interface PublicWebConfig {
	apiUrl?: string;
	redirectUrl?: string;
	logoutUrl?: string;
}

interface ConfigSource {
	runtimeEnabled: boolean;
	browserOrigin?: string;
	runtimeConfig?: unknown;
	buildConfig: PublicWebConfig;
}

declare global {
	interface Window {
		__FLOW_LIKE_PUBLIC_CONFIG__?: unknown;
	}
}

function validatePublicUrl(value: unknown, name: string): string {
	if (
		typeof value !== "string" ||
		!value ||
		/[\s\\]/u.test(value) ||
		Array.from(value).some((character) => {
			const code = character.charCodeAt(0);
			return code < 32 || code === 127;
		})
	) {
		throw new Error(`Invalid public web configuration: ${name}`);
	}
	let url: URL;
	try {
		url = new URL(value);
	} catch {
		throw new Error(`Invalid public web configuration: ${name}`);
	}
	if (
		!/^https?:\/\//.test(value) ||
		!url.hostname ||
		url.username ||
		url.password ||
		value.includes("?") ||
		value.includes("#") ||
		url.port === "0"
	) {
		throw new Error(`Invalid public web configuration: ${name}`);
	}
	return value;
}

/** Pure resolver, shared by browser accessors and tests. */
export function resolvePublicWebConfig(source: ConfigSource): PublicWebConfig {
	if (!source.runtimeEnabled) return source.buildConfig;
	// Static export has no deployment settings. Never serialize a hosted API
	// fallback into prerendered container pages or read window on the server.
	if (source.browserOrigin === undefined) return {};
	const config = source.runtimeConfig;
	if (
		!config ||
		typeof config !== "object" ||
		!("version" in config) ||
		config.version !== 1 ||
		!("apiUrl" in config)
	) {
		throw new Error(
			"Public web runtime configuration is missing or unsupported",
		);
	}
	return {
		apiUrl: validatePublicUrl(config.apiUrl, "apiUrl").replace(/\/+$/, ""),
		redirectUrl:
			"redirectUrl" in config
				? validatePublicUrl(config.redirectUrl, "redirectUrl")
				: new URL("/callback", source.browserOrigin).href,
		logoutUrl:
			"logoutUrl" in config
				? validatePublicUrl(config.logoutUrl, "logoutUrl")
				: new URL("/", source.browserOrigin).href,
	};
}

export function getPublicWebConfig(): PublicWebConfig {
	return resolvePublicWebConfig({
		runtimeEnabled: process.env.NEXT_PUBLIC_FLOW_LIKE_RUNTIME_CONFIG === "1",
		browserOrigin:
			typeof window === "undefined" ? undefined : window.location.origin,
		runtimeConfig:
			typeof window === "undefined"
				? undefined
				: window.__FLOW_LIKE_PUBLIC_CONFIG__,
		buildConfig: {
			apiUrl: process.env.NEXT_PUBLIC_API_URL,
			redirectUrl: process.env.NEXT_PUBLIC_REDIRECT_URL,
			logoutUrl: process.env.NEXT_PUBLIC_REDIRECT_LOGOUT_URL,
		},
	});
}

export function getPublicApiUrl(): string {
	const apiUrl = getPublicWebConfig().apiUrl;
	if (apiUrl) return apiUrl;
	if (process.env.NEXT_PUBLIC_FLOW_LIKE_RUNTIME_CONFIG === "1") {
		throw new Error(
			"Public web runtime configuration is only available in the browser",
		);
	}
	return "https://api.flow-like.com";
}
