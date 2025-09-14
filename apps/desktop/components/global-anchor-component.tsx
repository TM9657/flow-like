"use client";

import { createId } from "@paralleldrive/cuid2";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open as shellOpen } from "@tauri-apps/plugin-shell";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@tm9657/flow-like-ui";
import { useCallback, useEffect, useMemo, useState } from "react";

const isIosLike = () => {
  if (typeof navigator === "undefined") return false;
  // iPhone, iPad, iPod; also iPadOS reports MacIntel + touch
  return /iPad|iPhone|iPod/.test(navigator.userAgent) ||
    (navigator.platform === "MacIntel" && (navigator as any).maxTouchPoints > 1);
};

const isTauri = () =>
  typeof window !== "undefined" && Object.prototype.hasOwnProperty.call(window, "__TAURI__");

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
      if (isHttpish(url)) void shellOpen(url);
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
      // - iOS: always route http(s)/mailto/tel via shell to Safari/Phone/Mail
      // - target=_blank on any platform: if http(s)/mailto/tel and not same-origin app route, open via shell
      // - otherwise let normal navigation happen
      if (TAURI && httpish && (IOS || (externalIntent && !same))) {
        e.preventDefault();
        e.stopPropagation();
        await shellOpen(href);
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

      // If we already handled via shell (iOS/external), skip new window
      // For iOS this will open in Safari; desktop continues to spawn a Webview window
      if (IOS) {
        void shellOpen(anchor.href);
      } else {
        createNewWindow(anchor.href, linkTitle);
      }
    };

    const handleClick = async (event: MouseEvent) => {
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
    document.addEventListener("click", handleClick, true);
    document.addEventListener("contextmenu", handleContextMenu, true);

    return () => {
      document.removeEventListener("mousedown", handleMouseDown, true);
      document.removeEventListener("auxclick", handleAuxClick, true);
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

                    // iOS: open in Safari; Desktop: spawn new window for same-origin/_blank, shell for true external
                    if (isIosLike()) {
                      await shellOpen(href);
                    } else if (isHttpish(href) && !sameOrigin(href)) {
                      await shellOpen(href);
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
