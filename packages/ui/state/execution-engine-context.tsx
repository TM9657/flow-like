"use client";

import { createContext, useContext, useEffect, useRef } from "react";
import { RunningTasksIndicator } from "../components/execution-indicator";
import { ExecutionEngineProvider } from "../lib/execution-engine";
import type { IIntercomEvent } from "../lib/schema/events/intercom-event";
import { useBackend } from "./backend-state";
import { useExecutionServiceOptional } from "./execution-service-context";

const ExecutionEngineContext = createContext<ExecutionEngineProvider | null>(
	null,
);

export function ExecutionEngineProviderComponent({
	children,
}: { children: React.ReactNode }) {
	const backend = useBackend();
	const executionService = useExecutionServiceOptional();
	const engineRef = useRef<ExecutionEngineProvider | null>(null);

	if (!engineRef.current) {
		engineRef.current = new ExecutionEngineProvider();
	}

	useEffect(() => {
		if (engineRef.current && backend) {
			engineRef.current.setBackend(backend);
		}
	}, [backend]);

	useEffect(() => {
		if (engineRef.current && executionService) {
			engineRef.current.setExecuteEventFn(executionService.executeEvent);
		}
	}, [executionService]);

	return (
		<ExecutionEngineContext.Provider value={engineRef.current}>
			{children}
			<RunningTasksIndicator />
		</ExecutionEngineContext.Provider>
	);
}

export function useExecutionEngine(): ExecutionEngineProvider {
	const context = useContext(ExecutionEngineContext);
	if (!context) {
		throw new Error(
			"useExecutionEngine must be used within ExecutionEngineProviderComponent",
		);
	}
	return context;
}

export function useEventStream(
	streamId: string,
	subscriberId: string,
	onEvents: (events: IIntercomEvent[]) => void,
	onComplete?: (events: IIntercomEvent[]) => void,
) {
	const engine = useExecutionEngine();

	useEffect(() => {
		engine.subscribeToEventStream(streamId, subscriberId, onEvents, onComplete);

		return () => {
			engine.unsubscribeFromEventStream(streamId, subscriberId);
		};
	}, [engine, streamId, subscriberId, onEvents, onComplete]);
}
