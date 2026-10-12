import { useCallback, useEffect, useRef } from "react";

const HISTORY_KEY = "__flowLikeDocx";

interface DocumentHistory {
	id: string;
	url: string;
	state: unknown;
	pending?: (closed: boolean) => void;
}

function taggedState(document: DocumentHistory, entry: "document" | "return") {
	return {
		...(document.state && typeof document.state === "object"
			? document.state
			: {}),
		[HISTORY_KEY]: { id: document.id, entry },
	};
}

function isEntry(document: DocumentHistory, entry: "document" | "return") {
	const marker = window.history.state?.[HISTORY_KEY];
	return (
		window.location.href === document.url &&
		marker?.id === document.id &&
		marker.entry === entry
	);
}

/** Keep browser Back within this document's storage page until dismissal. */
export function useStorageDocxHistory(
	canClose: () => boolean,
	onBrowserClose: () => void,
) {
	const document = useRef<DocumentHistory | null>(null);
	const latest = useRef({ canClose, onBrowserClose });
	latest.current = { canClose, onBrowserClose };

	const openHistory = useCallback(() => {
		if (document.current) return;
		const next: DocumentHistory = {
			id: crypto.randomUUID(),
			url: window.location.href,
			state: window.history.state,
		};
		// Both entries retain Next's routing state. The return tag identifies our
		// own Back transition without intercepting visits to other history entries.
		window.history.replaceState(taggedState(next, "return"), "", next.url);
		window.history.pushState(taggedState(next, "document"), "", next.url);
		document.current = next;
	}, []);

	const closeHistory = useCallback((): Promise<boolean> => {
		const current = document.current;
		if (!current) return Promise.resolve(true);
		if (!isEntry(current, "document") || current.pending)
			return Promise.resolve(false);
		return new Promise((resolve) => {
			current.pending = resolve;
			window.history.back();
		});
	}, []);

	useEffect(() => {
		const handleBack = (event: PopStateEvent) => {
			const current = document.current;
			if (!current) return;
			if (!isEntry(current, "return")) {
				if (!isEntry(current, "document")) {
					document.current = null;
					current.pending?.(false);
				}
				return;
			}
			event.stopImmediatePropagation();
			if (!current.pending && !latest.current.canClose()) {
				window.history.pushState(
					taggedState(current, "document"),
					"",
					current.url,
				);
				return;
			}
			window.history.replaceState(current.state, "", current.url);
			document.current = null;
			if (current.pending) current.pending(true);
			else latest.current.onBrowserClose();
		};
		window.addEventListener("popstate", handleBack, true);
		return () => {
			window.removeEventListener("popstate", handleBack, true);
			const current = document.current;
			document.current = null;
			if (!current) return;
			const traversing = Boolean(current.pending);
			current.pending?.(false);
			if (isEntry(current, "return")) {
				window.history.replaceState(current.state, "", current.url);
			} else if (isEntry(current, "document")) {
				// This component can unmount without a route change. Consume only
				// the entry it owns; a concurrent unrelated traversal stays untouched.
				window.addEventListener(
					"popstate",
					(event) => {
						if (!isEntry(current, "return")) return;
						event.stopImmediatePropagation();
						window.history.replaceState(current.state, "", current.url);
					},
					{ capture: true, once: true },
				);
				if (!traversing) window.history.back();
			}
		};
	}, []);

	return { openHistory, closeHistory };
}
