import type { IApiState } from "@tm9657/flow-like-ui";
import type { IProfile } from "@tm9657/flow-like-ui/types";
import { type WebBackendRef, getApiBaseUrl } from "./api-utils";

export class WebApiState implements IApiState {
	constructor(private readonly backend: WebBackendRef) {}

	private constructUrl(profile: IProfile, path: string): string {
		let baseUrl = profile.hub ?? getApiBaseUrl();
		if (!baseUrl.endsWith("/")) {
			baseUrl += "/";
		}
		if (baseUrl.startsWith("http://") || baseUrl.startsWith("https://")) {
			return `${baseUrl}api/v1/${path}`;
		}
		const protocol = profile.secure === false ? "http" : "https";
		return `${protocol}://${baseUrl}api/v1/${path}`;
	}

	private getHeaders(): HeadersInit {
		const headers: HeadersInit = {
			"Content-Type": "application/json",
		};
		if (this.backend.auth?.user?.access_token) {
			headers["Authorization"] =
				`Bearer ${this.backend.auth.user.access_token}`;
		}
		return headers;
	}

	async fetch<T>(
		profile: IProfile,
		path: string,
		options?: RequestInit,
	): Promise<T> {
		const url = this.constructUrl(profile, path);
		const response = await fetch(url, {
			...options,
			headers: {
				...this.getHeaders(),
				...options?.headers,
			},
		});

		if (!response.ok) {
			throw new Error(`API error: ${response.status}`);
		}

		return response.json();
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
		const url = this.constructUrl(profile, path);
		const response = await fetch(url, {
			...options,
			headers: {
				...this.getHeaders(),
				...options?.headers,
			},
		});

		if (!response.ok) {
			throw new Error(`Stream error: ${response.status}`);
		}

		if (!response.body || !onMessage) return;

		const reader = response.body.getReader();
		const decoder = new TextDecoder();
		let buffer = "";

		while (true) {
			const { done, value } = await reader.read();
			if (done) break;

			buffer += decoder.decode(value, { stream: true });
			const lines = buffer.split("\n");
			buffer = lines.pop() ?? "";

			for (const line of lines) {
				if (line.startsWith("data: ")) {
					try {
						const data = JSON.parse(line.slice(6)) as T;
						onMessage(data);
					} catch {
						// Ignore parse errors
					}
				}
			}
		}
	}
}
