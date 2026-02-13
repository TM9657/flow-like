import type { IProfile } from "../../types";

export interface IApiState {
	/**
	 * Generic fetch function for API calls
	 */
	fetch<T>(profile: IProfile, path: string, options?: RequestInit): Promise<T>;

	/**
	 * GET request
	 */
	get<T>(profile: IProfile, path: string): Promise<T>;

	/**
	 * POST request
	 */
	post<T>(profile: IProfile, path: string, data?: unknown): Promise<T>;

	/**
	 * PUT request
	 */
	put<T>(profile: IProfile, path: string, data?: unknown): Promise<T>;

	/**
	 * PATCH request
	 */
	patch<T>(profile: IProfile, path: string, data?: unknown): Promise<T>;

	/**
	 * DELETE request
	 */
	del<T>(profile: IProfile, path: string, data?: unknown): Promise<T>;

	/**
	 * Stream fetcher for SSE endpoints
	 */
	stream<T>(
		profile: IProfile,
		path: string,
		options?: RequestInit,
		onMessage?: (data: T) => void,
	): Promise<void>;
}
