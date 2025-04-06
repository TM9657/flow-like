"use client";

import { type Event, type UnlistenFn, listen } from "@tauri-apps/api/event";
import type { IIntercomEvent } from "@tm9657/flow-like-ui";
import { useEffect } from "react";
import { toast } from "sonner";

interface IToastEvent {
	message: string;
	level: "success" | "error" | "info" | "warning";
}

export default function ToastProvider() {
	useEffect(() => {
		const subscriptions: (Promise<UnlistenFn> | undefined)[] = [];
		const unlistenFn = listen("toast", (events: Event<IIntercomEvent[]>) => {
			const messages: IToastEvent[] = events.payload.map(
				(event) => event.payload,
			);
			console.dir(messages);
			for (const message of messages) {
				if (message.level === "success") return toast.success(message.message);
				if (message.level === "error") return toast.error(message.message);
				if (message.level === "warning") return toast.warning(message.message);
				toast.info(message.message);
			}
		});

		subscriptions.push(unlistenFn);

		return () => {
			(async () => {
				for await (const subscription of subscriptions) {
					if (subscription) subscription();
				}
			})();
		};
	}, []);

	return null;
}
