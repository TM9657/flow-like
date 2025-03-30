"use client"

import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { toast } from "sonner";

export default function ToastProvider() {
    useEffect(() => {
		const subscriptions: (Promise<UnlistenFn> | undefined)[] = [];
		const unlistenFn = listen(
			"toast",
			(event: {
				payload: {
					message: string;
					level: "success" | "error" | "info" | "warning";
				}[];
			}) => {
				for (const message of event.payload) {
					if (message.level === "success")
						return toast.success(message.message);
					if (message.level === "error") return toast.error(message.message);
					if (message.level === "warning")
						return toast.warning(message.message);
					toast.info(message.message);
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
	}, []);

    return null
}