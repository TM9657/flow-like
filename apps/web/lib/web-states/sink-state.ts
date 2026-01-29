import type { IEventRegistration, ISinkState } from "@tm9657/flow-like-ui";
import { apiGet, apiDelete, type WebBackendRef } from "./api-utils";

export class WebSinkState implements ISinkState {
	constructor(private readonly backend: WebBackendRef) {}

	async listEventSinks(): Promise<IEventRegistration[]> {
		return apiGet<IEventRegistration[]>("sinks", this.backend.auth);
	}

	async removeEventSink(eventId: string): Promise<void> {
		await apiDelete<void>(`sinks/${eventId}`, this.backend.auth);
	}

	async isEventSinkActive(eventId: string): Promise<boolean> {
		try {
			const result = await apiGet<{ active: boolean }>(
				`sinks/${eventId}/status`,
				this.backend.auth,
			);
			return result?.active ?? false;
		} catch {
			return false;
		}
	}
}
