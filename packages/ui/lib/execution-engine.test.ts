import { describe, expect, mock, test } from "bun:test";
import { createFrontendStateStore } from "../components/a2ui/frontend-state";
import type { IBackendState } from "../state/backend-state";
import {
	type ExecuteEventFn,
	ExecutionEngineProvider,
} from "./execution-engine";
import type { IIntercomEvent } from "./schema/events/intercom-event";

describe("ExecutionEngineProvider live events", () => {
	test("applies state once while detached and does not replay it on subscription or completion", async () => {
		const engine = new ExecutionEngineProvider();
		const state = createFrontendStateStore(undefined);
		let emit!: (events: IIntercomEvent[]) => void;
		let finish!: () => void;
		const executeEvent: ExecuteEventFn = (
			_appId,
			_eventId,
			_payload,
			_streamState,
			_onExecutionStart,
			callback,
		) => {
			if (!callback) throw new Error("Missing event callback");
			emit = callback;
			return new Promise((resolve) => {
				finish = () => resolve(undefined);
			});
		};
		engine.setBackend({
			eventState: { executeEvent },
		} as unknown as IBackendState);
		const onLiveEvents = mock((events: IIntercomEvent[]) => {
			for (const event of events) state.handleMessage(event.payload);
		});
		const running = engine.executeEvent("chat-session", {
			appId: "app",
			eventId: "chat",
			payload: { id: "chat" },
			onLiveEvents,
		});
		const globalUpdate = {
			event_id: "global-update",
			event_type: "a2ui",
			payload: { type: "setGlobalState", key: "count", value: 1 },
		} as IIntercomEvent;
		const pageUpdate = {
			event_id: "page-update",
			event_type: "a2ui",
			payload: {
				type: "setPageState",
				page_id: "chat",
				key: "tab",
				value: "all",
			},
		} as IIntercomEvent;
		emit([globalUpdate, pageUpdate]);
		expect(state.getSnapshot().globalState.count).toBe(1);
		expect(state.getSnapshot().pageStates.chat.tab).toBe("all");

		// A widget can write newer state before the chat subscribes again.
		state.setGlobalState("count", 2);
		const onReplay = mock((_events: IIntercomEvent[]) => {});
		engine.subscribeToEventStream("chat-session", "chat-view", onReplay);
		expect(onReplay).toHaveBeenCalledWith([globalUpdate, pageUpdate]);
		expect(state.getSnapshot().globalState.count).toBe(2);
		emit([globalUpdate, pageUpdate]);
		expect(onLiveEvents).toHaveBeenCalledTimes(1);
		expect(state.getSnapshot().globalState.count).toBe(2);

		engine.unsubscribeFromEventStream("chat-session", "chat-view");
		const nextUpdate = {
			...globalUpdate,
			event_id: "next-update",
			payload: { type: "setGlobalState", key: "count", value: 3 },
		};
		emit([nextUpdate]);
		expect(state.getSnapshot().globalState.count).toBe(3);
		finish();
		await running;
		state.setGlobalState("count", 4);
		engine.subscribeToEventStream("chat-session", "finished-view", onReplay);
		expect(onLiveEvents).toHaveBeenCalledTimes(2);
		expect(state.getSnapshot().globalState.count).toBe(4);
	});
});
