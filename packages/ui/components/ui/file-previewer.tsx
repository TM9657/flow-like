"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import { MonacoFileEditor } from "./monaco-file-editor";
import { TextEditor } from "./text-editor";

function rawFileName(url: string) {
	if (url.startsWith("data:")) {
		const mediaType = url.split(";")[0].split(":")[1];
		if (mediaType) {
			const extension = mediaType.split("/")[1];
			if (extension) {
				return `file.${extension}`;
			}
		}
		return "file";
	}

	return url.split("?")[0].split("/").pop() ?? "";
}

export function isPdf(file: string) {
	return /\.(pdf)$/i.test(rawFileName(file));
}

export function isImage(file: string) {
	return /\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(rawFileName(file));
}

export function isVideo(file: string) {
	return /\.(mp4|mkv|webm|ogg|avi|mov)$/i.test(rawFileName(file));
}

export function isAudio(file: string) {
	return /\.(mp3|wav|ogg|flac|aac)$/i.test(rawFileName(file));
}

export function isCode(file: string) {
	return /\.(json|xml|css|js|jsx|ts|tsx|py|java|c|cpp|h|hpp|cs|go|rb|php|swift|kt|rs|html|yml|yaml|toml|sql|sh|bash|scss|sass|less|vue|svelte)$/i.test(
		rawFileName(file),
	);
}

export function getCodeLanguage(file: string) {
	return (
		/\.(json|xml|css|js|jsx|ts|tsx|py|java|c|cpp|h|hpp|cs|go|rb|php|swift|kt|rs|html|yml|yaml|toml|sql|sh|bash|scss|sass|less|vue|svelte)$/i
			.exec(rawFileName(file))?.[0]
			?.replaceAll(".", "") ?? "text"
	);
}

export function isText(file: string) {
	if (isCode(file)) return true;
	return /\.(txt|csv|html|md|mdx|ini|conf|cfg|log|env)$/i.test(
		rawFileName(file),
	);
}

export function canPreview(file: string) {
	return (
		isPdf(file) ||
		isImage(file) ||
		isVideo(file) ||
		isAudio(file) ||
		isText(file)
	);
}

export function FilePreviewer({
	url,
	page,
	editable = false,
	onSave,
}: Readonly<{
	url: string;
	page?: number;
	editable?: boolean;
	onSave?: (content: string) => Promise<void>;
}>) {
	const [content, setContent] = useState<string>("");
	const [pdfKey, setPdfKey] = useState(0);
	const containerRef = useRef<HTMLDivElement>(null);

	const previewContent = useCallback(async () => {
		const response = await fetch(url);
		if (!response.ok) {
			throw new Error("Failed to fetch file");
		}
		setContent(await response.text());
	}, [url]);

	useEffect(() => {
		if (isText(url)) {
			previewContent();
		}
	}, [url]);

	useEffect(() => {
		if (isPdf(url) && containerRef.current) {
			const observer = new ResizeObserver(() => {
				// Force rerender of the PDF iframe when size changes
				setPdfKey((prev) => prev + 1);
			});

			observer.observe(containerRef.current);

			return () => {
				observer.disconnect();
			};
		}
	}, [url]);

	if (!canPreview(url)) {
		return (
			<div className="text-red-500">File type not supported for preview</div>
		);
	}

	if (isPdf(url)) {
		const pageUrl = page
			? `#page=${page}&#toolbar=1&#view=FitH`
			: "#toolbar=1&#view=FitH";
		return (
			<div ref={containerRef} className="w-full h-full flex flex-col">
				<iframe
					key={pdfKey}
					src={`${url}${pageUrl}`}
					className="w-full h-full border-0 max-h-full max-w-full"
					title={`PDF Preview: ${rawFileName(url)}`}
				>
					<p>
						Your browser cannot display the PDF.{" "}
						<a href={url} target="_blank" rel="noopener noreferrer">
							Download the PDF
						</a>{" "}
						instead.
					</p>
				</iframe>
			</div>
		);
	}

	if (isImage(url)) {
		return (
			<img
				src={url}
				alt={rawFileName(url)}
				className="w-full h-full object-contain"
			/>
		);
	}

	if (isVideo(url)) {
		return (
			<video src={url} controls className="w-full h-full object-contain">
				<track
					kind="captions"
					label="English captions"
					srcLang="en"
					src=""
					default={false}
				/>
				Your browser does not support the video tag.
			</video>
		);
	}

	if (isAudio(url)) {
		return (
			<audio src={url} controls className="w-full h-full object-contain">
				<track
					kind="captions"
					label="English captions"
					srcLang="en"
					src=""
					default={false}
				/>
				Your browser does not support the audio tag.
			</audio>
		);
	}

	if (isCode(url)) {
		if (editable && onSave) {
			return (
				<MonacoFileEditor
					fileName={rawFileName(url)}
					initialContent={content}
					editable={true}
					onSave={onSave}
				/>
			);
		}
		return (
			<TextEditor
				initialContent={`\n\`\`\`${getCodeLanguage(url)}\n${content}\n\`\`\`\n`}
				isMarkdown={true}
				editable={false}
			/>
		);
	}

	if (isText(url)) {
		if (editable && onSave) {
			return (
				<MonacoFileEditor
					fileName={rawFileName(url)}
					initialContent={content}
					editable={true}
					onSave={onSave}
				/>
			);
		}
		return (
			<TextEditor initialContent={content} isMarkdown={true} editable={false} />
		);
	}

	return (
		<div className="text-red-500">File type not supported for preview</div>
	);
}
