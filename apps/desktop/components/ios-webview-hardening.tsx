"use client";

import { useEffect } from "react";

const IOS_VIEWPORT_CONTENT =
	"width=device-width, initial-scale=1, maximum-scale=1, viewport-fit=cover, interactive-widget=resizes-content";
const MAX_SAFE_TOP_PX = 96;
const MAX_SAFE_BOTTOM_PX = 64;

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

function clamp(value: number, min: number, max: number): number {
	return Math.min(Math.max(value, min), max);
}

function syncIOSViewportMetrics() {
	const vv = window.visualViewport;
	const viewportHeight = Math.round(vv?.height ?? window.innerHeight);
	const safeTop = clamp(Math.round(vv?.offsetTop ?? 0), 0, MAX_SAFE_TOP_PX);
	const safeBottom = clamp(
		Math.round(
			vv
				? window.innerHeight - vv.height - (vv.offsetTop ?? 0)
				: 0,
		),
		0,
		MAX_SAFE_BOTTOM_PX,
	);

	document.documentElement.style.setProperty(
		"--fl-ios-vvh",
		`${viewportHeight}px`,
	);
	document.documentElement.style.setProperty("--fl-ios-safe-top", `${safeTop}px`);
	document.documentElement.style.setProperty(
		"--fl-ios-safe-bottom",
		`${safeBottom}px`,
	);
}

export function IOSWebviewHardening() {
	useEffect(() => {
		if (!isTauriRuntime() || !isIOSDevice()) return;

		document.documentElement.setAttribute("data-ios-app", "true");
		document.body.setAttribute("data-ios-app", "true");
		upsertViewportMeta(IOS_VIEWPORT_CONTENT);

		syncIOSViewportMetrics();

		window.visualViewport?.addEventListener(
			"resize",
			syncIOSViewportMetrics,
		);
		window.visualViewport?.addEventListener(
			"scroll",
			syncIOSViewportMetrics,
		);
		window.addEventListener("orientationchange", syncIOSViewportMetrics);
		window.addEventListener("resize", syncIOSViewportMetrics);

		return () => {
			window.visualViewport?.removeEventListener(
				"resize",
				syncIOSViewportMetrics,
			);
			window.visualViewport?.removeEventListener(
				"scroll",
				syncIOSViewportMetrics,
			);
			window.removeEventListener("orientationchange", syncIOSViewportMetrics);
			window.removeEventListener("resize", syncIOSViewportMetrics);
			document.documentElement.style.removeProperty("--fl-ios-vvh");
			document.documentElement.style.removeProperty("--fl-ios-safe-top");
			document.documentElement.style.removeProperty("--fl-ios-safe-bottom");
		};
	}, []);

	return null;
}
