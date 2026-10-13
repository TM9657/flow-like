"use client";

import { useTranslation } from "@flow-like/locales";
import { safeCssLength } from "../safe-css-length";

import type { TCaptionProps, TImageElement, TResizableProps } from "platejs";
import type { SlateElementProps } from "platejs/static";

import { NodeApi } from "platejs";
import { SlateElement } from "platejs/static";

import { cn } from "../../../lib/utils";
import { useEditorAssetUrl } from "../hooks/use-editor-asset-url";
import { type MediaMetadata, mediaObjectPosition } from "../media-metadata";
import {
	useIsStreamingMarkdown,
	useRendererPresentation,
} from "../renderer-presentation";
import { MediaAttribution } from "./media-attribution";

export function ImageElementStatic(
	props: SlateElementProps<
		TImageElement & TCaptionProps & TResizableProps & MediaMetadata
	>,
) {
	const { align = "center", alt, caption, url, width } = props.element;
	const src = useEditorAssetUrl(url);
	const presentation = useRendererPresentation();
	const isStreaming = useIsStreamingMarkdown();
	const { t } = useTranslation("common");
	const pending = isStreaming && Boolean(props.element.streamingPending);
	const placeholder =
		pending && (props.element.streamingPending !== "ready" || !src);

	return (
		<SlateElement {...props} className="py-2.5">
			<figure
				aria-busy={pending || undefined}
				data-fl-editorial-image={
					presentation === "default" ? undefined : "chat"
				}
				className={cn(
					"group relative m-0 inline-block",
					placeholder && "w-full",
				)}
				style={{ width: safeCssLength(width) }}
			>
				<div
					className="relative max-w-full min-w-[92px]"
					style={{ textAlign: align }}
				>
					{placeholder ? (
						<div
							data-slot="editorial-image-pending"
							className="flex aspect-video w-full items-center justify-center rounded-lg bg-muted/30 text-sm text-muted-foreground motion-safe:animate-pulse"
						>
							<output>{t("preparingImage", "Preparing image…")}</output>
						</div>
					) : (
						<img
							className={cn(
								"w-full max-w-full cursor-default object-cover px-0",
								"rounded-sm",
							)}
							alt={alt ?? ""}
							loading="lazy"
							decoding="async"
							style={{ objectPosition: mediaObjectPosition(props.element) }}
							src={src}
						/>
					)}
					{caption && (
						<figcaption className="mx-auto mt-2 max-w-full">
							{caption.map((node) => NodeApi.string(node)).join("\n")}
						</figcaption>
					)}
				</div>
				<MediaAttribution element={props.element} />
			</figure>
			{props.children}
		</SlateElement>
	);
}
