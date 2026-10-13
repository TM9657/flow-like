"use client";

import { createContext, useContext } from "react";

export type RendererPresentation = "default" | "chat" | "chat-compact";

export const RendererPresentationContext =
	createContext<RendererPresentation>("default");

export const IsStreamingMarkdownContext = createContext(false);

export function useRendererPresentation() {
	return useContext(RendererPresentationContext);
}

export function useIsStreamingMarkdown() {
	return useContext(IsStreamingMarkdownContext);
}
