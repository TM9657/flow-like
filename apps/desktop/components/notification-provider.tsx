"use client";

import { useQueryClient } from "@tanstack/react-query";
import { type Event, type UnlistenFn, listen } from "@tauri-apps/api/event";
import { useBackend } from "@tm9657/flow-like-ui";
import type { IIntercomEvent, INotificationEvent } from "@tm9657/flow-like-ui";
import { useEffect, useRef } from "react";
import { useAuth } from "react-oidc-context";
import { toast } from "sonner";
import { fetcher } from "../lib/api";
import { addLocalNotification } from "../lib/notifications-db";

type NotificationPermission = "granted" | "denied" | "default";
type NotificationApi = {
	isPermissionGranted: () => Promise<boolean>;
	requestPermission: () => Promise<NotificationPermission>;
	sendNotification: (options: { title: string; body?: string }) => void;
};

async function loadNotificationPlugin(): Promise<NotificationApi | null> {
	try {
		const mod = await import("@tauri-apps/plugin-notification");
		return {
			isPermissionGranted: mod.isPermissionGranted,
			requestPermission: mod.requestPermission,
			sendNotification: mod.sendNotification,
		};
	} catch {
		return null;
	}
}

interface NotificationProviderProps {
	appId?: string;
}

export default function NotificationProvider({
	appId,
}: NotificationProviderProps = {}) {
	const auth = useAuth();
	const backend = useBackend();
	const queryClient = useQueryClient();
	// Use a constant for offline/unauthenticated users
	const userId = auth.user?.profile?.sub ?? "offline-user";
	const notificationApi = useRef<NotificationApi | null>(null);
	const permissionGranted = useRef<boolean>(false);

	useEffect(() => {
		const initNotifications = async () => {
			try {
				const api = await loadNotificationPlugin();
				if (api) {
					notificationApi.current = api;
					let granted = await api.isPermissionGranted();
					if (!granted) {
						const permission = await api.requestPermission();
						granted = permission === "granted";
					}
					permissionGranted.current = granted;
				}
			} catch (e) {
				// Notification plugin not available (e.g., in dev mode or unsupported platform)
				console.log(
					"[NotificationProvider] Desktop notifications not available:",
					e,
				);
			}
		};

		initNotifications();
	}, []);

	useEffect(() => {
		const subscriptions: (Promise<UnlistenFn> | undefined)[] = [];

		const unlistenFn = listen(
			"flow_notification",
			async (events: Event<IIntercomEvent[]>) => {
				for (const event of events.payload) {
					const notification = event.payload as INotificationEvent;

					// Store in local database for persistence
					try {
						await addLocalNotification({
							userId,
							appId,
							title: notification.title,
							description: notification.description,
							icon: notification.icon,
							link: notification.link,
							notificationType: "WORKFLOW",
							sourceRunId: notification.source_run_id,
							sourceNodeId: notification.source_node_id,
						});

						// Refetch notification queries so UI updates immediately
						// Using refetchQueries instead of invalidateQueries to force immediate refetch
						await queryClient.refetchQueries({
							predicate: (query) => {
								const key = query.queryKey[0];
								return (
									key === "getNotifications" || key === "listNotifications"
								);
							},
						});
					} catch (e) {
						console.error(
							"[NotificationProvider] Failed to store local notification:",
							e,
						);
					}

					// Persist notification via backend API (requires event_id)
					if (
						appId &&
						backend?.profile &&
						auth.user &&
						notification.event_id &&
						notification.event_id.trim().length > 0
					) {
						try {
							await fetcher<{ id: string; success: boolean }>(
								backend.profile,
								`apps/${appId}/notifications/create`,
								{
									method: "POST",
									body: JSON.stringify({
										event_id: notification.event_id,
										target_user_sub: notification.target_user_sub,
										title: notification.title,
										description: notification.description,
										icon: notification.icon,
										link: notification.link,
										run_id: notification.source_run_id,
										node_id: notification.source_node_id,
									}),
								},
								auth,
							);
						} catch (e) {
							console.warn(
								"[NotificationProvider] Failed to persist remote notification:",
								e,
							);
						}
					}

					// Show desktop notification if enabled
					if (
						notificationApi.current &&
						permissionGranted.current &&
						notification.show_desktop
					) {
						notificationApi.current.sendNotification({
							title: notification.title,
							body: notification.description ?? undefined,
						});
					} else {
						toast.info(notification.title, {
							description: notification.description,
						});
					}
				}
			},
		);

		subscriptions.push(unlistenFn);

		return () => {
			(async () => {
				for await (const subscription of subscriptions) {
					if (subscription) subscription();
				}
			})();
		};
	}, [userId, appId, queryClient]);

	return null;
}
