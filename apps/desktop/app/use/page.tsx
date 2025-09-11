"use client";

import {
	Container,
	Header,
	type IToolBarActions,
	LoadingScreen,
	NoDefaultInterface,
	useBackend,
	useInvoke,
	useSetQueryParams,
} from "@tm9657/flow-like-ui";
import type {
	ISidebarActions,
	IUseInterfaceProps,
} from "@tm9657/flow-like-ui/components/interfaces/interfaces";
import { parseUint8ArrayToJson } from "@tm9657/flow-like-ui/lib/uint8";
import { useRouter, useSearchParams } from "next/navigation";
import {
	type JSX,
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useRef,
} from "react";
import { EVENT_CONFIG } from "../../lib/event-config";
import NotFound from "../library/config/not-found";

export default function Page() {
	const backend = useBackend();
	const searchParams = useSearchParams();
	const appId = searchParams.get("id");
	const headerRef = useRef<IToolBarActions>(null!);
	const sidebarRef = useRef<ISidebarActions>(null!);
	const setQueryParams = useSetQueryParams();
	const router = useRouter();

	const usableEvents = useMemo(() => {
		const events = new Map<
			string,
			(props: IUseInterfaceProps) => JSX.Element | ReactNode | null
		>();
		Object.values(EVENT_CONFIG).forEach((config) => {
			const usable = Object.entries(config.useInterfaces);
			for (const [eventType, useInterface] of usable) {
				if (config.eventTypes.includes(eventType)) {
					events.set(eventType, useInterface);
				}
			}
		});
		return events;
	}, [EVENT_CONFIG]);

	const metadata = useInvoke(
		backend.appState.getAppMeta,
		backend.appState,
		[appId ?? ""],
		typeof appId === "string",
	);

	const eventId = searchParams.get("eventId");
	const events = useInvoke(
		backend.eventState.getEvents,
		backend.eventState,
		[appId ?? ""],
		(appId ?? "") !== "",
	);
	const sortedEvents = useMemo(() => {
		if (!events.data) return [];
		return events.data
			.filter((a) => a.active)
			.toSorted((a, b) => a.priority - b.priority);
	}, [events.data]);

	const currentEvent = useMemo(() => {
		if (!eventId) return undefined;
		return sortedEvents.find((e) => e.id === eventId);
	}, [eventId, sortedEvents]);

	const switchEvent = useCallback(
		(newEventId: string) => {
			if (!appId) return;
			if (!newEventId) return;
			if (eventId === newEventId) return;
			if (newEventId === "") return;

			// Clear toolbar elements when switching events
			headerRef.current?.pushToolbarElements([]);
			setQueryParams("eventId", newEventId);
		},
		[appId, router, eventId, setQueryParams],
	);

	const config = useMemo(() => {
		if (!currentEvent) return {};
		try {
			return parseUint8ArrayToJson(currentEvent.config);
		} catch (e) {
			console.error("Error parsing parameters:", e);
			return {};
		}
	}, [currentEvent]);

	useEffect(() => {
		if (!appId) return;
		if (sortedEvents.length === 0 && events.data) {
			console.log("No events found, redirecting to event config");
			router.replace(`/store?id=${appId}`);
			return;
		}

		if (sortedEvents.length === 0) return;

		let rerouteEvent = sortedEvents.find((e) => usableEvents.has(e.event_type));

		if (!rerouteEvent && usableEvents.size > 0 && events.data) {
			console.log("No usable events found, redirecting to event config");
			router.replace(`/store?id=${appId}`);
			return;
		}

		const lastEventId = localStorage.getItem(`lastUsedEvent-${appId}`);
		const lastEvent = sortedEvents.find((e) => e.id === lastEventId);

		if (lastEvent && usableEvents.has(lastEvent.event_type)) {
			rerouteEvent = lastEvent;
		}

		if (!currentEvent) {
			if (rerouteEvent) {
				switchEvent(rerouteEvent.id);
				return;
			}
			return;
		}

		if (eventId && !usableEvents.has(currentEvent.event_type)) {
			switchEvent(rerouteEvent?.id ?? "");
			return;
		}

		localStorage.setItem(`lastUsedEvent-${appId}`, eventId ?? "");
	}, [
		appId,
		eventId,
		sortedEvents,
		currentEvent,
		switchEvent,
		usableEvents,
		events.data,
	]);

	const inner = useMemo(() => {
		if (!appId) return <NotFound />;
		if (!currentEvent) return <LoadingScreen />;
		if (!usableEvents) return <LoadingScreen />;

		if (usableEvents.has(currentEvent.event_type)) {
			const innerItem = usableEvents.get(currentEvent.event_type);
			if (innerItem)
				return (
					<div
						key={currentEvent.id}
						className="flex flex-col flex-grow h-full w-full max-h-full overflow-hidden"
					>
						{innerItem({
							appId,
							event: currentEvent,
							config,
							toolbarRef: headerRef,
							sidebarRef: sidebarRef,
						})}
					</div>
				);
		}

		return <NoDefaultInterface appId={appId} eventId={eventId ?? undefined} />;
	}, [
		appId,
		currentEvent,
		config,
		eventId,
		headerRef,
		sidebarRef,
		usableEvents,
	]);

	if (!appId) {
		return <NotFound />;
	}

	return (
		<main className="flex flex-col h-full min-h-dvh max-h-dvh overflow-hidden">
			<Container ref={sidebarRef}>
				<div className="flex flex-col flex-grow h-full w-full max-h-full overflow-hidden">
					<Header
						ref={headerRef}
						usableEvents={new Set(usableEvents.keys())}
						currentEvent={currentEvent}
						sortedEvents={sortedEvents}
						metadata={metadata.data}
						appId={appId}
						switchEvent={switchEvent}
					/>
					{inner}
				</div>
			</Container>
		</main>
	);
}
