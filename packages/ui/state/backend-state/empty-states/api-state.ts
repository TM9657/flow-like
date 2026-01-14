import type { IProfile } from "../../../types";
import type { IApiState } from "../api-state";

function constructUrl(profile: IProfile, path: string): string {
	let baseUrl = profile.hub ?? "api.flow-like.com";
	if (typeof process !== "undefined" && process.env?.NEXT_PUBLIC_API_URL) {
		baseUrl = process.env.NEXT_PUBLIC_API_URL;
	}
	if (!baseUrl.endsWith("/")) {
		baseUrl += "/";
	}

	if (baseUrl.startsWith("http://") || baseUrl.startsWith("https://")) {
		return `${baseUrl}api/v1/${path}`;
	}

	const protocol = profile.secure === false ? "http" : "https";
	return `${protocol}://${baseUrl}api/v1/${path}`;
}

/**
 * Empty API state implementation using native fetch.
 * Can be used for web apps or as a base for custom implementations.
 */
export class EmptyApiState implements IApiState {
	private getAuthHeader: () => string | null;

	constructor(getAuthHeader: () => string | null = () => null) {
		this.getAuthHeader = getAuthHeader;
	}

	private getHeaders(extraHeaders?: HeadersInit): Headers {
		const headers = new Headers({
			"Content-Type": "application/json",
			...extraHeaders,
		});

		const authHeader = this.getAuthHeader();
		if (authHeader) {
			headers.set("Authorization", `Bearer ${authHeader}`);
		}

		return headers;
	}

	async fetch<T>(
		profile: IProfile,
		path: string,
		options?: RequestInit,
	): Promise<T> {
		const url = constructUrl(profile, path);
		const response = await fetch(url, {
			...options,
			headers: this.getHeaders(options?.headers as HeadersInit),
		});

		if (!response.ok) {
			throw new Error(`Error fetching data: ${response.statusText}`);
		}

		return response.json() as Promise<T>;
	}

	async get<T>(profile: IProfile, path: string): Promise<T> {
		return this.fetch<T>(profile, path, { method: "GET" });
	}

	async post<T>(profile: IProfile, path: string, data?: unknown): Promise<T> {
		return this.fetch<T>(profile, path, {
			method: "POST",
			body: data ? JSON.stringify(data) : undefined,
		});
	}

	async put<T>(profile: IProfile, path: string, data?: unknown): Promise<T> {
		return this.fetch<T>(profile, path, {
			method: "PUT",
			body: data ? JSON.stringify(data) : undefined,
		});
	}

	async patch<T>(profile: IProfile, path: string, data?: unknown): Promise<T> {
		return this.fetch<T>(profile, path, {
			method: "PATCH",
			body: data ? JSON.stringify(data) : undefined,
		});
	}

	async del<T>(profile: IProfile, path: string, data?: unknown): Promise<T> {
		return this.fetch<T>(profile, path, {
			method: "DELETE",
			body: data ? JSON.stringify(data) : undefined,
		});
	}

	async stream<T>(
		profile: IProfile,
		path: string,
		options?: RequestInit,
		onMessage?: (data: T) => void,
	): Promise<void> {
		const url = constructUrl(profile, path);
		const response = await fetch(url, {
			...options,
			headers: this.getHeaders({
				Accept: "text/event-stream",
				...options?.headers,
			}),
		});

		if (!response.ok) {
			throw new Error(`HTTP error: ${response.status} ${response.statusText}`);
		}

		if (!response.body) {
			throw new Error("Response body is null - streaming not supported");
		}

		const reader = response.body.getReader();
		const decoder = new TextDecoder();
		let buffer = "";

		try {
			while (true) {
				const { done, value } = await reader.read();
				if (done) break;

				buffer += decoder.decode(value, { stream: true });
				const lines = buffer.split("\n\n");
				buffer = lines.pop() ?? "";

				for (const line of lines) {
					if (!line.trim()) continue;
					const dataLine = line
						.split("\n")
						.find((l) => l.startsWith("data:"));
					if (dataLine) {
						const data = dataLine.slice(5).trim();
						try {
							const parsed = JSON.parse(data) as T;
							onMessage?.(parsed);
						} catch {
							// Non-JSON data, skip
						}
					}
				}
			}
		} finally {
			reader.releaseLock();
		}
	}
}
