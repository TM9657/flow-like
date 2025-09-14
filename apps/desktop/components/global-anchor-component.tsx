"use client";

import { createId } from "@paralleldrive/cuid2";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { openUrl as shellOpen } from '@tauri-apps/plugin-opener';
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@tm9657/flow-like-ui";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

const isIosLike = () => {
	if (typeof navigator === "undefined") return false;
	// iPhone, iPad, iPod; also iPadOS reports MacIntel + touch
	return /iPad|iPhone|iPod/.test(navigator.userAgent) ||
		(navigator.platform === "MacIntel" && (navigator as any).maxTouchPoints > 1);
};

const isTauri = () =>
	typeof window !== "undefined" && (
		"__TAURI__" in (window as any) ||
		"__TAURI_INTERNAL__" in (window as any) ||
		"__TAURI_IPC__" in (window as any)
	);

const isHttpish = (href: string) =>
	/^(https?:|mailto:|tel:)/i.test(href);

const sameOrigin = (href: string) => {
	try {
		const u = new URL(href, location.href);
		return u.origin === location.origin;
	} catch {
		return false;
	}
};

const wantsExternal = (a: HTMLAnchorElement) =>
	a.getAttribute("target") === "_blank" ||
	a.rel.split(/\s+/).includes("external") ||
	a.dataset.openExternal === "true";

// Best-effort external opener that works on iOS; avoids blocking the user gesture.
const openInBrowser = async (href: string) => {
	try {
		await shellOpen(href);
		return true;
	} catch {
		try {
			// May still open in the same webview, but better than doing nothing
			window.open(href, "_blank", "noopener,noreferrer");
			return true;
		} catch {
			location.href = href; // last-resort fallback
			return true;
		}
	}
};

const GlobalAnchorHandler = () => {
	const [contextMenuData, setContextMenuData] = useState<{
		x: number;
		y: number;
		href: string;
		show: boolean;
		title?: string;
	} | null>(null);

	const IOS = useMemo(isIosLike, []);
	const TAURI = useMemo(isTauri, []);

	const createNewWindow = useCallback((url: string, title?: string) => {
		// Desktop-only: iOS WKWebView doesn't support multiple windows like desktop
		if (!TAURI || IOS) {
			// Fallback to shell open if we can't spawn a new webview
			if (isHttpish(url)) void openInBrowser(url);
			return;
		}

		const windowLabel = `window-${createId()}`;
		try {
			const _view = new WebviewWindow(windowLabel, {
				url,
				title: title ?? "Flow-Like",
				focus: true,
				resizable: true,
				maximized: true,
			});
		} catch (error) {
			console.error("Failed to create new window:", error);
		}
	}, [IOS, TAURI]);

	useEffect(() => {
		const lastTouchHandledAt = { value: 0 };
		const findAnchorElement = (target: HTMLElement): HTMLAnchorElement | null => {
			let el: HTMLElement | null = target;
			while (el) {
				if (el.tagName === "A") return el as HTMLAnchorElement;
				el = el.parentElement;
			}
			return null;
		};

		// Unified external open handler
		const openExternallyIfNeeded = async (a: HTMLAnchorElement, e: MouseEvent) => {
			const href = a.href;
			if (!href) return false;

			const externalIntent = wantsExternal(a);
			const httpish = isHttpish(href);
			const same = sameOrigin(href);

			// Rules:
			// - iOS: route http(s)/mailto/tel via shell when either true external OR link expresses external intent (_blank/rel=external)
			// - Desktop: target=_blank on any platform: if http(s)/mailto/tel and not same-origin app route, open via shell
			// - otherwise let normal navigation happen
			if (!httpish) return false;

			if (IOS) {
				if (externalIntent || !same) {
					e.preventDefault();
					e.stopPropagation();
					void openInBrowser(href);
					return true;
				}
				return false;
			}

			if (TAURI && externalIntent && !same) {
				e.preventDefault();
				e.stopPropagation();
				void openInBrowser(href);
				return true;
			}
			return false;
		};

		const handleMouseDown = (event: MouseEvent) => {
			// Prevent the browser from handling middle-click before we decide
			if (event.button === 1) {
				const anchor = findAnchorElement(event.target as HTMLElement);
				if (anchor?.href) {
					event.preventDefault();
					event.stopPropagation();
					event.stopImmediatePropagation();
				}
			}
		};

		const handleAuxClick = (event: MouseEvent) => {
			if (event.button !== 1) return;
			const anchor = findAnchorElement(event.target as HTMLElement);
			if (!anchor?.href) return;

			// Middle click: prefer new app window on desktop; iOS -> shell open
			event.preventDefault();
			event.stopPropagation();
			event.stopImmediatePropagation();

			const linkTitle =
				anchor.textContent?.trim() ??
				anchor.getAttribute("title") ??
				undefined;

			// If on iOS, prefer opening true external links in Safari; otherwise let app handle
			if (IOS) {
				if (isHttpish(anchor.href) && !sameOrigin(anchor.href)) {
					void openInBrowser(anchor.href);
				}
			} else {
				createNewWindow(anchor.href, linkTitle);
			}
		};

			const handleTouchEnd = async (event: TouchEvent) => {
			if (!IOS) return;
			const anchor = findAnchorElement(event.target as HTMLElement);
			if (!anchor?.href) return;
			const href = anchor.href;
			if (!isHttpish(href)) return;
			const externalIntent = wantsExternal(anchor);
			const same = sameOrigin(href);
			if (externalIntent || !same) {
				event.preventDefault();
				event.stopPropagation();
				// Call opener without await to keep gesture context
				void openInBrowser(href);
				lastTouchHandledAt.value = Date.now();
			}
		};

			const handlePointerUp = async (event: PointerEvent) => {
				if (!IOS) return;
				// Only react to touch-like pointers
				if (event.pointerType !== "touch" && event.pointerType !== "pen") return;
				const anchor = findAnchorElement(event.target as HTMLElement);
				if (!anchor?.href) return;
				const href = anchor.href;
				if (!isHttpish(href)) return;
				const externalIntent = wantsExternal(anchor);
				const same = sameOrigin(href);
				if (externalIntent || !same) {
					event.preventDefault();
					event.stopPropagation();
					void openInBrowser(href);
					lastTouchHandledAt.value = Date.now();
				}
			};

		const handleClick = async (event: MouseEvent) => {
			// If a touch handler just ran, ignore the synthetic click
			if (IOS && Date.now() - lastTouchHandledAt.value < 500) {
				event.preventDefault();
				event.stopPropagation();
				event.stopImmediatePropagation?.();
				setContextMenuData(null);
				return;
			}
			const anchor = findAnchorElement(event.target as HTMLElement);
			if (!anchor?.href) {
				setContextMenuData(null);
				return;
			}

			// Cmd/Ctrl click should pass through (desktop user expectations)
			if ((event as MouseEvent).metaKey || (event as MouseEvent).ctrlKey) {
				setContextMenuData(null);
				return;
			}

			// If this is an external intent or we're on iOS, open via shell and stop.
			const handled = await openExternallyIfNeeded(anchor, event);
			if (handled) {
				// Ensure no further handlers run
				event.stopImmediatePropagation?.();
				setContextMenuData(null);
				return;
			}

			// Otherwise, if target=_blank and we are on desktop Tauri, spawn a new window
			if (wantsExternal(anchor) && TAURI && !IOS) {
				event.preventDefault();
				event.stopPropagation();
				const linkTitle =
					anchor.textContent?.trim() ??
					anchor.getAttribute("title") ??
					undefined;
				createNewWindow(anchor.href, linkTitle);
			}

			// Close context menu on any (handled or not) click
			setContextMenuData(null);
		};

		const handleContextMenu = (event: MouseEvent) => {
			const anchor = findAnchorElement(event.target as HTMLElement);
			if (!anchor?.href) return;

			event.preventDefault();

			setContextMenuData({
				x: event.clientX,
				y: event.clientY,
				href: anchor.href,
				title:
					anchor.textContent?.trim() ??
					anchor.getAttribute("title") ??
					undefined,
				show: true,
			});
		};

		document.addEventListener("mousedown", handleMouseDown, true);
		document.addEventListener("auxclick", handleAuxClick, true);
		document.addEventListener("touchend", handleTouchEnd, true);
		document.addEventListener("pointerup", handlePointerUp as any, true);
		document.addEventListener("click", handleClick, true);
		document.addEventListener("contextmenu", handleContextMenu, true);

		return () => {
			document.removeEventListener("mousedown", handleMouseDown, true);
			document.removeEventListener("auxclick", handleAuxClick, true);
			document.removeEventListener("touchend", handleTouchEnd, true);
			document.removeEventListener("pointerup", handlePointerUp as any, true);
			document.removeEventListener("click", handleClick, true);
			document.removeEventListener("contextmenu", handleContextMenu, true);
		};
	}, [IOS, TAURI, createNewWindow]);

	return (
		<>
			{contextMenuData && (
				<div
					style={{
						position: "fixed",
						left: contextMenuData.x,
						top: contextMenuData.y,
						zIndex: 50,
						pointerEvents: "auto",
					}}
				>
					<DropdownMenu
						open={contextMenuData.show}
						onOpenChange={(open) => {
							if (!open) setContextMenuData(null);
						}}
					>
						<DropdownMenuTrigger asChild>
							<div className="w-1 h-1 opacity-0" />
						</DropdownMenuTrigger>

						<DropdownMenuContent side="bottom" align="start">
							<DropdownMenuItem asChild>
								<button
									className="w-full"
									onMouseDown={async (e) => {
										e.preventDefault();
										e.stopPropagation();
										const href = contextMenuData.href;
										const title = contextMenuData.title;

										// iOS: always open in browser; Desktop: spawn new window for same-origin/_blank, shell for true external
										if (isIosLike()) {
											if (isHttpish(href)) {
												await openInBrowser(href);
											}
										} else if (isHttpish(href) && !sameOrigin(href)) {
											await openInBrowser(href);
										} else {
											createNewWindow(href, title);
										}
										setContextMenuData(null);
									}}
									style={{ cursor: "pointer" }}
								>
									{IOS ? "Open in browser" : "Open in new window"}
								</button>
							</DropdownMenuItem>

							<DropdownMenuItem asChild>
								<button
									className="w-full"
									onMouseDown={(e) => {
										e.preventDefault();
										e.stopPropagation();
										navigator.clipboard.writeText(contextMenuData.href);
										setContextMenuData(null);
									}}
									style={{ cursor: "pointer" }}
								>
									Copy Link
								</button>
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			)}
		</>
	);
};

export default GlobalAnchorHandler;
