import type {
	IEvent,
	IIntercomEvent,
	ILogMetadata,
	IOAuthProvider,
	IOAuthToken,
	IRunPayload,
	IVersionType,
} from "../../lib";

export interface IOAuthCheckResult {
	tokens?: Record<string, IOAuthToken>;
	missingProviders: IOAuthProvider[];
}

export interface IEventState {
	getEvent(
		appId: string,
		eventId: string,
		version?: [number, number, number],
	): Promise<IEvent>;
	getEvents(appId: string): Promise<IEvent[]>;
	getEventVersions(
		appId: string,
		eventId: string,
	): Promise<[number, number, number][]>;
	upsertEvent(
		appId: string,
		event: IEvent,
		versionType?: IVersionType,
		personalAccessToken?: string,
		oauthTokens?: Record<string, IOAuthToken>,
	): Promise<IEvent>;
	/** Check OAuth requirements for an event's board. Returns missing providers. */
	checkEventOAuth?(
		appId: string,
		event: IEvent,
	): Promise<IOAuthCheckResult>;
	deleteEvent(appId: string, eventId: string): Promise<void>;
	validateEvent(
		appId: string,
		eventId: string,
		version?: [number, number, number],
	): Promise<void>;
	upsertEventFeedback(
		appId: string,
		eventId: string,
		feedbackId: string,
		feedback: {
			rating: number;
			history?: any[];
			globalState?: Record<string, any>;
			localState?: Record<string, any>;
			comment?: string;
		},
	): Promise<string>;
	executeEvent(
		appId: string,
		eventId: string,
		payload: IRunPayload,
		streamState?: boolean,
		onEventId?: (id: string) => void,
		cb?: (event: IIntercomEvent[]) => void,
	): Promise<ILogMetadata | undefined>;

	cancelExecution(runId: string): Promise<void>;

	isEventSinkActive(eventId: string): Promise<boolean>;
}
