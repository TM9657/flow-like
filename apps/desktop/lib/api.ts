import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import type { IProfile } from "@tm9657/flow-like-ui";
import type { AuthContextProps } from "react-oidc-context";
import { createEventSource, EventSourceMessage } from 'eventsource-client'

function constructUrl(profile: IProfile, path: string): string {
	let baseUrl = profile.hub ?? "api.flow-like.com";
	if (process.env.NEXT_PUBLIC_API_URL)
		baseUrl = process.env.NEXT_PUBLIC_API_URL;
	if (!baseUrl.endsWith("/")) {
		baseUrl += "/";
	}

	if (baseUrl.startsWith("http://") || baseUrl.startsWith("https://")) {
		// If the base URL is already a full URL, use it as is
		return `${baseUrl}api/v1/${path}`;
	}

	return `https://${baseUrl}api/v1/${path}`;
}

type SSEMessage = {
	event?: string;
	data: string;
	id?: string;
	raw: string;
};

function tryParseJSON<T>(text: string): T | null {
	try {
		return JSON.parse(text) as T;
	} catch {
		return null;
	}
}

export async function streamFetcher<T>(
	profile: IProfile,
	path: string,
	options?: RequestInit,
	auth?: AuthContextProps,
	onMessage?: (data: T) => void,
): Promise<void> {
	const authHeader = auth?.user?.id_token ? { Authorization: `Bearer ${auth.user.id_token}` } : {};
	const url = constructUrl(profile, path);
	let finished = false;

	await new Promise<void>((resolve, reject) => {
		const es = createEventSource({
			url: url,
			fetch: tauriFetch,
			// @ts-ignore
			headers: {
				Accept: "text/event-stream",
				// Only set Content-Type if we actually send a body
				...(options?.body ? { "Content-Type": "application/json" } : {}),
				...(options?.headers ?? {}),
				...(authHeader.Authorization ? { Authorization: authHeader.Authorization } : {}),
			},
			method: options?.method ?? "GET",
			body: options?.body ? options.body : undefined,
			signal: options?.signal,
			onMessage: (message: EventSourceMessage) => {
				const evt = message?.event ?? "message";
				const parsedData = tryParseJSON<T>(message.data);
				if (parsedData && onMessage) {
					onMessage(parsedData);
				} else {
					console.warn("Received non-JSON data:", message.data);
				}

				if (evt === "done") {
                    if (!finished) {
                        finished = true;
                        try { es.close(); } catch {}
                        resolve();
                    }
                }

                if (evt === "error") {
                    if (!finished) {
                        finished = true;
                        try { es.close(); } catch {}
                        reject(new Error("SSE stream error"));
                    }
                }
			},
			onConnect: () => {
				console.log("Connected to SSE stream:", url);
			},
			onDisconnect: () => {
				console.log("Disconnected from SSE stream:", url);
				es.close();
				resolve();
			},
		})
	})
}

export async function fetcher<T>(
	profile: IProfile,
	path: string,
	options?: RequestInit,
	auth?: AuthContextProps,
): Promise<T> {
	const headers: HeadersInit = {};
	if (auth?.user?.id_token) {
		headers["Authorization"] = `Bearer ${auth?.user?.id_token}`;
	}

	const url = constructUrl(profile, path);
	try {
		const response = await tauriFetch(url, {
			...options,
			headers: {
				"Content-Type": "application/json",
				...options?.headers,
				...headers,
			},
			keepalive: true,
			priority: "high",
		});

		if (!response.ok) {
			if (response.status === 401 && auth) {
				auth?.startSilentRenew();
			}
			console.error(`Error fetching ${path}:`, response);
			throw new Error(`Error fetching data: ${response.statusText}`);
		}

		const json = await response.json();
		console.groupCollapsed(`API Request: ${path}`);
		console.dir(json, { depth: null });
		console.groupEnd();
		return json as T;
	} catch (error) {
		console.groupCollapsed(`API Request: ${path}`);
		console.error(`Error fetching ${path}:`, error);
		console.groupEnd();
		throw new Error(`Error fetching data: ${error}`);
	}
}

export async function post<T>(
	profile: IProfile,
	path: string,
	data?: any,
	auth?: AuthContextProps,
): Promise<T> {
	return fetcher<T>(
		profile,
		path,
		{
			method: "POST",
			body: data ? JSON.stringify(data) : undefined,
		},
		auth,
	);
}

export async function get<T>(
	profile: IProfile,
	path: string,
	auth?: AuthContextProps,
): Promise<T> {
	return fetcher<T>(
		profile,
		path,
		{
			method: "GET",
		},
		auth,
	);
}

export async function put<T>(
	profile: IProfile,
	path: string,
	data?: any,
	auth?: AuthContextProps,
): Promise<T> {
	return fetcher<T>(
		profile,
		path,
		{
			method: "PUT",
			body: data ? JSON.stringify(data) : undefined,
		},
		auth,
	);
}

export async function del<T>(
	profile: IProfile,
	path: string,
	data?: any,
	auth?: AuthContextProps,
): Promise<T> {
	return fetcher<T>(
		profile,
		path,
		{
			method: "DELETE",
			body: data ? JSON.stringify(data) : undefined,
		},
		auth,
	);
}

export async function patch<T>(
	profile: IProfile,
	path: string,
	data?: any,
	auth?: AuthContextProps,
): Promise<T> {
	return fetcher<T>(
		profile,
		path,
		{
			method: "PATCH",
			body: data ? JSON.stringify(data) : undefined,
		},
		auth,
	);
}

