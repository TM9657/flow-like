"use client";

import { useEffect } from "react";

const IOS_VIEWPORT_CONTENT =
	"width=device-width, initial-scale=1, maximum-scale=1, viewport-fit=cover, interactive-widget=resizes-content";

function isTauriRuntime(): boolean {
	if (typeof window === "undefined") return false;
	const w = window as Window & {
		__TAURI__?: unknown;
		__TAURI_IPC__?: unknown;
		__TAURI_INTERNALS__?: unknown;
	};
	return Boolean(w.__TAURI__ || w.__TAURI_IPC__ || w.__TAURI_INTERNALS__);
}

function isIOSDevice(): boolean {
	if (typeof navigator === "undefined") return false;
	return (
		/iPad|iPhone|iPod/.test(navigator.userAgent) ||
		(navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1)
	);
}

function upsertViewportMeta(content: string) {
	let meta = document.querySelector(
		'meta[name="viewport"]',
	) as HTMLMetaElement | null;

	if (!meta) {
		meta = document.createElement("meta");
		meta.name = "viewport";
		document.head.appendChild(meta);
	}

	meta.setAttribute("content", content);
}

function syncIOSVisualViewportHeight() {
	const vv = window.visualViewport;
	const px = Math.round(vv?.height ?? window.innerHeight);
	document.documentElement.style.setProperty("--fl-ios-vvh", `${px}px`);
}

export function IOSWebviewHardening() {
	useEffect(() => {
		if (!isTauriRuntime() || !isIOSDevice()) return;

		document.documentElement.setAttribute("data-ios-app", "true");
		document.body.setAttribute("data-ios-app", "true");
		upsertViewportMeta(IOS_VIEWPORT_CONTENT);

		syncIOSVisualViewportHeight();

		window.visualViewport?.addEventListener(
			"resize",
			syncIOSVisualViewportHeight,
		);
		window.visualViewport?.addEventListener(
			"scroll",
			syncIOSVisualViewportHeight,
		);
		window.addEventListener("orientationchange", syncIOSVisualViewportHeight);
		window.addEventListener("resize", syncIOSVisualViewportHeight);

		return () => {
			window.visualViewport?.removeEventListener(
				"resize",
				syncIOSVisualViewportHeight,
			);
			window.visualViewport?.removeEventListener(
				"scroll",
				syncIOSVisualViewportHeight,
			);
			window.removeEventListener("orientationchange", syncIOSVisualViewportHeight);
			window.removeEventListener("resize", syncIOSVisualViewportHeight);
			document.documentElement.style.removeProperty("--fl-ios-vvh");
		};
	}, []);

	return null;
}
