import type { IBackendState } from "../state/backend-state";
import type { IRunPayload } from "./schema";
import type { IIntercomEvent } from "./schema/events/intercom-event";

interface ISubscriber {
	onEvents: (events: IIntercomEvent[]) => void;
	onComplete?: (events: IIntercomEvent[]) => void;
}

interface IEventStream {
	subscribers: Map<string, ISubscriber>;
	accumulatedEvents: IIntercomEvent[];
	lastSentIndex: Map<string, number>;
	executionPromise?: Promise<any>;
	isComplete: boolean;
	path?: string;
	title?: string;
	interfaceType?: string;
}

interface IExecuteEventOptions {
	appId: string;
	eventId: string;
	payload: IRunPayload;
	streamState?: boolean;
	onExecutionStart?: (executionId: string) => void;
	path?: string;
	title?: string;
	interfaceType?: string;
}

export class ExecutionEngineProvider {
	private eventStreams: Map<string, IEventStream> = new Map();
	private backend: IBackendState | null = null;
	private globalListeners: Set<() => void> = new Set();

	constructor() {}

	setBackend(backend: IBackendState): void {
		this.backend = backend;
	}

	subscribeToGlobalUpdates(listener: () => void): () => void {
		this.globalListeners.add(listener);
		return () => {
			this.globalListeners.delete(listener);
		};
	}

	private notifyGlobalListeners() {
		this.globalListeners.forEach((listener) => listener());
	}

	subscribeToEventStream(
		streamId: string,
		subscriberId: string,
		onEvents: (events: IIntercomEvent[]) => void,
		onComplete?: (events: IIntercomEvent[]) => void,
	): void {
		if (!this.eventStreams.has(streamId)) {
			this.eventStreams.set(streamId, {
				subscribers: new Map(),
				accumulatedEvents: [],
				lastSentIndex: new Map(),
				isComplete: false,
			});
		}

		const stream = this.eventStreams.get(streamId)!;

		// Register subscriber
		stream.subscribers.set(subscriberId, { onEvents, onComplete });

		// Send accumulated events immediately
		if (stream.accumulatedEvents.length > 0) {
			onEvents(stream.accumulatedEvents);
			stream.lastSentIndex.set(subscriberId, stream.accumulatedEvents.length);
		} else {
			stream.lastSentIndex.set(subscriberId, 0);
		}

		// If already complete, notify immediately
		if (stream.isComplete && onComplete) {
			onComplete(stream.accumulatedEvents);
		}

		this.notifyGlobalListeners();
	}

	unsubscribeFromEventStream(streamId: string, subscriberId: string): void {
		const stream = this.eventStreams.get(streamId);
		if (stream) {
			stream.subscribers.delete(subscriberId);
			stream.lastSentIndex.delete(subscriberId);

			// Only delete if complete AND no subscribers
			if (stream.subscribers.size === 0 && stream.isComplete) {
				this.eventStreams.delete(streamId);
			}
			this.notifyGlobalListeners();
		}
	}

	async executeEvent(
		streamId: string,
		options: IExecuteEventOptions,
	): Promise<any> {
		if (!this.backend) {
			throw new Error("Backend not initialized in ExecutionEngineProvider");
		}

		let stream = this.eventStreams.get(streamId);
		if (!stream) {
			stream = {
				subscribers: new Map(),
				accumulatedEvents: [],
				lastSentIndex: new Map(),
				isComplete: false,
				path: options.path,
				title: options.title,
				interfaceType: options.interfaceType,
			};
			this.eventStreams.set(streamId, stream);
		} else {
			// If the stream was previously complete, reset it for the new execution
			if (stream.isComplete) {
				stream.isComplete = false;
				stream.accumulatedEvents = [];
				stream.lastSentIndex.clear();
				// We keep existing subscribers, but reset their sent index
				for (const subscriberId of stream.subscribers.keys()) {
					stream.lastSentIndex.set(subscriberId, 0);
				}
			}

			// Update metadata if provided
			if (options.path) stream.path = options.path;
			if (options.title) stream.title = options.title;
			if (options.interfaceType) stream.interfaceType = options.interfaceType;
		}

		this.notifyGlobalListeners();

		if (stream.executionPromise) {
			return stream.executionPromise;
		}

		const executionPromise = this.backend.eventState.executeEvent(
			options.appId,
			options.eventId,
			options.payload,
			options.streamState ?? false,
			(executionId: string) => {
				options.onExecutionStart?.(executionId);
			},
			(events: IIntercomEvent[]) => {
				// Handle new events
				if (events.length > 0) {
					stream!.accumulatedEvents.push(...events);

					// Publish to all subscribers
					for (const [
						subscriberId,
						subscriber,
					] of stream!.subscribers.entries()) {
						const lastSent = stream!.lastSentIndex.get(subscriberId) ?? 0;
						const newEvents = stream!.accumulatedEvents.slice(lastSent);

						if (newEvents.length > 0) {
							subscriber.onEvents(newEvents);
							stream!.lastSentIndex.set(
								subscriberId,
								stream!.accumulatedEvents.length,
							);
						}
					}

					this.notifyGlobalListeners();
				}
			},
		);

		stream.executionPromise = executionPromise;

		executionPromise
			.then(() => {
				stream!.isComplete = true;

				// Notify all subscribers of completion
				for (const subscriber of stream!.subscribers.values()) {
					if (subscriber.onComplete) {
						subscriber.onComplete(stream!.accumulatedEvents);
					}
				}

				this.notifyGlobalListeners();
			})
			.catch((error) => {
				console.error("Execution error:", error);
				stream!.isComplete = true;

				// Notify subscribers of completion
				for (const subscriber of stream!.subscribers.values()) {
					if (subscriber.onComplete) {
						subscriber.onComplete(stream!.accumulatedEvents);
					}
				}

				this.notifyGlobalListeners();
			});

		return executionPromise;
	}

	isStreamActive(streamId: string): boolean {
		const stream = this.eventStreams.get(streamId);
		return stream ? !stream.isComplete : false;
	}

	hasStream(streamId: string): boolean {
		return this.eventStreams.has(streamId);
	}

	isStreamComplete(streamId: string): boolean {
		const stream = this.eventStreams.get(streamId);
		return stream?.isComplete ?? false;
	}

	getAccumulatedEvents(streamId: string): IIntercomEvent[] {
		return this.eventStreams.get(streamId)?.accumulatedEvents ?? [];
	}

	getBackgroundStreams(): {
		streamId: string;
		path?: string;
		title?: string;
		preview?: string;
		interfaceType?: string;
	}[] {
		const backgroundStreams: {
			streamId: string;
			path?: string;
			title?: string;
			preview?: string;
			interfaceType?: string;
		}[] = [];

		for (const [streamId, stream] of this.eventStreams.entries()) {
			if (stream.subscribers.size === 0) {
				let preview = "";
				for (let i = stream.accumulatedEvents.length - 1; i >= 0; i--) {
					const ev = stream.accumulatedEvents[i];
					if (
						ev.event_type === "chat_stream_partial" &&
						ev.payload.chunk?.choices?.[0]?.delta?.content
					) {
						preview = ev.payload.chunk.choices[0].delta.content;
						break;
					}
					if (
						ev.event_type === "chat_stream" &&
						ev.payload.response?.choices?.[0]?.message?.content
					) {
						preview = ev.payload.response.choices[0].message.content;
						break;
					}
				}

				backgroundStreams.push({
					streamId,
					path: stream.path,
					title: stream.title,
					preview:
						preview.substring(0, 100) + (preview.length > 100 ? "..." : ""),
					interfaceType: stream.interfaceType,
				});
			}
		}

		return backgroundStreams;
	}
}
