"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, FilePreviewComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

function rawFileName(url: string): string {
	if (url.startsWith("data:")) {
		const mediaType = url.split(";")[0].split(":")[1];
		if (mediaType) {
			const extension = mediaType.split("/")[1];
			if (extension) return `file.${extension}`;
		}
		return "file";
	}
	return url.split("?")[0].split("/").pop() ?? "";
}

function getFileType(
	url: string,
): "pdf" | "image" | "video" | "audio" | "code" | "text" | "unknown" {
	const name = rawFileName(url).toLowerCase();
	if (/\.(pdf)$/i.test(name)) return "pdf";
	if (/\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(name)) return "image";
	if (/\.(mp4|mkv|webm|ogg|avi|mov)$/i.test(name)) return "video";
	if (/\.(mp3|wav|ogg|flac|aac)$/i.test(name)) return "audio";
	if (
		/\.(json|xml|css|js|jsx|ts|tsx|py|java|c|cpp|h|hpp|cs|go|rb|php|swift|kt|rs|html|yml|yaml|toml|sql|sh|bash|scss|sass|less|vue|svelte)$/i.test(
			name,
		)
	)
		return "code";
	if (/\.(txt|csv|md|mdx|ini|conf|cfg|log|env)$/i.test(name)) return "text";
	return "unknown";
}

function getCodeLanguage(file: string): string {
	const match =
		/\.(json|xml|css|js|jsx|ts|tsx|py|java|c|cpp|h|hpp|cs|go|rb|php|swift|kt|rs|html|yml|yaml|toml|sql|sh|bash|scss|sass|less|vue|svelte)$/i.exec(
			rawFileName(file),
		);
	return match?.[0]?.replace(".", "") ?? "text";
}

export function A2UIFilePreview({
	component,
	style,
}: ComponentProps<FilePreviewComponent>) {
	const src = useResolved<string>(component.src);
	const showControls = useResolved<boolean>(component.showControls) ?? true;
	const fit = useResolved<string>(component.fit) ?? "contain";
	const fallbackText =
		useResolved<string>(component.fallbackText) ?? "Cannot preview this file";

	const [content, setContent] = useState<string>("");
	const [error, setError] = useState(false);
	const [pdfKey, setPdfKey] = useState(0);
	const containerRef = useRef<HTMLDivElement>(null);

	const fileType = src ? getFileType(src) : "unknown";

	const loadTextContent = useCallback(async () => {
		if (!src) return;
		try {
			const response = await fetch(src);
			if (!response.ok) throw new Error("Failed to fetch");
			setContent(await response.text());
		} catch {
			setError(true);
		}
	}, [src]);

	useEffect(() => {
		if (fileType === "code" || fileType === "text") {
			loadTextContent();
		}
	}, [fileType, loadTextContent]);

	useEffect(() => {
		if (fileType === "pdf" && containerRef.current) {
			const observer = new ResizeObserver(() => {
				setPdfKey((prev) => prev + 1);
			});
			observer.observe(containerRef.current);
			return () => observer.disconnect();
		}
	}, [fileType]);

	const fitClass =
		{
			contain: "object-contain",
			cover: "object-cover",
			fill: "object-fill",
			none: "object-none",
			scaleDown: "object-scale-down",
		}[fit] ?? "object-contain";

	if (!src || error) {
		return (
			<div
				className={cn(
					"flex items-center justify-center text-muted-foreground p-4",
					resolveStyle(style),
				)}
				style={resolveInlineStyle(style)}
			>
				{fallbackText}
			</div>
		);
	}

	if (fileType === "pdf") {
		return (
			<div
				ref={containerRef}
				className={cn("w-full h-full flex flex-col", resolveStyle(style))}
				style={resolveInlineStyle(style)}
			>
				<iframe
					key={pdfKey}
					src={`${src}#toolbar=1&#view=FitH`}
					className="w-full h-full border-0"
					title={`PDF Preview: ${rawFileName(src)}`}
				>
					<p>
						Your browser cannot display the PDF.{" "}
						<a href={src} target="_blank" rel="noopener noreferrer">
							Download
						</a>
					</p>
				</iframe>
			</div>
		);
	}

	if (fileType === "image") {
		return (
			<img
				src={src}
				alt={rawFileName(src)}
				className={cn("w-full h-full", fitClass, resolveStyle(style))}
				style={resolveInlineStyle(style)}
				onError={() => setError(true)}
			/>
		);
	}

	if (fileType === "video") {
		return (
			<video
				src={src}
				controls={showControls}
				className={cn("w-full h-full", fitClass, resolveStyle(style))}
				style={resolveInlineStyle(style)}
			>
				<track kind="captions" srcLang="en" label="English captions" />
				Your browser does not support the video tag.
			</video>
		);
	}

	if (fileType === "audio") {
		return (
			<div
				className={cn(
					"flex items-center justify-center p-4",
					resolveStyle(style),
				)}
				style={resolveInlineStyle(style)}
			>
				<audio src={src} controls={showControls} className="w-full max-w-md">
					Your browser does not support the audio tag.
				</audio>
			</div>
		);
	}

	if (fileType === "code" || fileType === "text") {
		const lang = fileType === "code" ? getCodeLanguage(src) : "";
		return (
			<div
				className={cn(
					"w-full h-full overflow-auto bg-muted/30 rounded",
					resolveStyle(style),
				)}
				style={resolveInlineStyle(style)}
			>
				<pre className="p-4 text-sm font-mono whitespace-pre-wrap break-all">
					{lang && (
						<div className="text-xs text-muted-foreground mb-2 uppercase">
							{lang}
						</div>
					)}
					<code>{content}</code>
				</pre>
			</div>
		);
	}

	return (
		<div
			className={cn(
				"flex items-center justify-center text-muted-foreground p-4",
				resolveStyle(style),
			)}
			style={resolveInlineStyle(style)}
		>
			{fallbackText}
		</div>
	);
}
