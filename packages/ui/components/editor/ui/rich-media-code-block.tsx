"use client";

import { useTranslation } from "@flow-like/locales";
import { useMemo, useRef, useState } from "react";
import { useEditorAssetUrl } from "../hooks/use-editor-asset-url";
import { useRendererPresentation } from "../renderer-presentation";
import {
	type RichMediaData,
	type RichMediaImage,
	type RichMediaLanguage,
	parseRichMediaData,
	parseStreamingRichMediaData,
} from "./rich-media-data";

function EditorialImage({ image }: { image: RichMediaImage }) {
	const src = useEditorAssetUrl(image.src);
	const [failedSource, setFailedSource] = useState<string>();
	const picture =
		src && failedSource !== src ? (
			<img
				className="aspect-[16/10] w-full rounded-lg object-cover"
				src={src}
				alt={image.alt}
				loading="lazy"
				decoding="async"
				onError={() => setFailedSource(src)}
			/>
		) : (
			<div
				className="grid aspect-[16/10] w-full place-items-center rounded-lg bg-muted p-4 text-sm text-muted-foreground"
				data-slot="rich-media-image-fallback"
				role="img"
				aria-label={image.alt}
			>
				{image.alt}
			</div>
		);
	return (
		<figure data-slot="rich-media-image" className="m-0 min-w-0">
			{image.href ? (
				<a href={image.href} target="_blank" rel="noopener noreferrer">
					{picture}
				</a>
			) : (
				picture
			)}
			{(image.caption || image.credit) && (
				<figcaption>
					{image.caption && <span>{image.caption}</span>}
					{image.credit && (
						<span data-slot="rich-media-credit">{image.credit}</span>
					)}
				</figcaption>
			)}
		</figure>
	);
}

function PreviewSkeleton({
	kind,
	label,
}: { kind: RichMediaLanguage; label: string }) {
	return (
		<div data-slot="rich-media-skeleton" data-kind={kind}>
			<output className="sr-only">{label}</output>
			<div data-slot="rich-media-skeleton-shapes" aria-hidden="true">
				<div />
				<div />
				<div />
			</div>
		</div>
	);
}

export function RichMediaCodeBlock({
	content,
	language,
	isStreaming = false,
}: { content: string; language: RichMediaLanguage; isStreaming?: boolean }) {
	const { t } = useTranslation("common");
	const presentation = useRendererPresentation();
	const [showSource, setShowSource] = useState(false);
	const lastPreview = useRef<{
		content: string;
		language: RichMediaLanguage;
		data: RichMediaData | null;
	} | null>(null);
	const { data, complete } = useMemo(() => {
		let data: RichMediaData | null;
		let complete = false;
		try {
			data = parseRichMediaData(content, language);
			complete = true;
		} catch {
			data = isStreaming
				? parseStreamingRichMediaData(content, language)
				: null;
		}
		const previous = lastPreview.current;
		// A closing fence arrives one backtick at a time. Keep the last readable
		// preview while an append is temporarily invalid; edits and settling reset it.
		if (
			!data &&
			isStreaming &&
			previous?.language === language &&
			content.startsWith(previous.content)
		)
			data = previous.data;
		lastPreview.current = { content, language, data };
		return { data, complete };
	}, [content, language, isStreaming]);
	const title = data?.title;
	const pending = isStreaming && !complete;
	return (
		<section
			data-fl-rich-media={language}
			aria-busy={pending || undefined}
			data-fl-rich-media-appearance={
				presentation === "default" ? "default" : "chat"
			}
			aria-label={
				title ??
				(language === "gallery"
					? t("imageGallery", "Image gallery")
					: language === "recommendations"
						? t("rankedRecommendations", "Ranked recommendations")
						: t("comparison", "Comparison"))
			}
			className="my-3 min-w-0 font-sans"
		>
			{(data || isStreaming) && (
				<div
					data-slot="rich-media-heading"
					className="mb-3 flex items-start justify-between gap-4"
				>
					{title ? (
						<div data-slot="rich-media-title" className="font-semibold">
							{title}
						</div>
					) : (
						<span />
					)}
					<button
						type="button"
						data-slot="rich-media-source-toggle"
						className="shrink-0 rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring"
						onClick={() => setShowSource((value) => !value)}
						aria-pressed={showSource}
					>
						{showSource
							? t("showPreview", "Show preview")
							: t("viewSource", "View source")}
					</button>
				</div>
			)}
			{showSource || (!data && !isStreaming) ? (
				<pre
					data-slot="rich-media-source"
					className="overflow-x-auto rounded-lg bg-muted/40 p-4 text-sm"
				>
					<code>{content}</code>
				</pre>
			) : !data ? (
				<PreviewSkeleton
					kind={language}
					label={t("buildingPreview", "Building preview")}
				/>
			) : data.kind === "gallery" ? (
				<div
					data-slot="gallery-grid"
					className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3"
				>
					{data.items.map((item) => (
						<EditorialImage key={item.id} image={item} />
					))}
				</div>
			) : data.kind === "recommendations" ? (
				<ol
					data-slot="recommendation-list"
					className="m-0 list-none p-0"
					// biome-ignore lint/a11y/useSemanticElements lint/a11y/noRedundantRoles: Explicit list semantics survive list-style:none in Safari.
					role="list"
				>
					{data.items.map((item) => (
						<li
							key={item.id}
							data-slot="recommendation-item"
							className="border-b border-border/50 py-4 last:border-0"
						>
							<div data-slot="recommendation-body">
								{item.image && (
									<EditorialImage
										image={{
											...item.image,
											href: item.image.href ?? item.href,
										}}
									/>
								)}
								<div data-slot="recommendation-details">
									<div
										data-slot="recommendation-heading"
										className="mb-2 flex items-start gap-2"
									>
										<span data-slot="recommendation-rank" aria-hidden="true">
											{item.rank}.
										</span>
										<div className="min-w-0 flex-1">
											<div
												data-slot="recommendation-title"
												className="font-semibold"
											>
												{item.href ? (
													<a
														href={item.href}
														target="_blank"
														rel="noopener noreferrer"
													>
														{item.title}
													</a>
												) : (
													item.title
												)}
											</div>
											{item.subtitle && (
												<div
													data-slot="recommendation-subtitle"
													className="text-sm text-muted-foreground"
												>
													{item.subtitle}
												</div>
											)}
										</div>
										{item.badge && (
											<span
												data-slot="recommendation-badge"
												className="rounded-full bg-muted px-2 py-1 text-xs"
											>
												{item.badge}
											</span>
										)}
									</div>

									{item.specs && item.specs.length > 0 && (
										<dl
											data-slot="recommendation-specs"
											className="flex flex-wrap gap-x-4 gap-y-2"
										>
											{item.specs.map((spec, specIndex) => (
												<div key={`${item.id}-${specIndex}`}>
													<dt className="text-xs text-muted-foreground">
														{spec.label}
													</dt>
													<dd className="m-0 text-sm font-medium">
														{spec.value}
													</dd>
												</div>
											))}
										</dl>
									)}
									{item.description && (
										<p data-slot="recommendation-description">
											{item.description}
										</p>
									)}
								</div>
							</div>
						</li>
					))}
				</ol>
			) : (
				<div data-slot="comparison-content">
					{data.description && (
						<p
							data-slot="comparison-description"
							className="mb-4 text-sm text-muted-foreground"
						>
							{data.description}
						</p>
					)}
					<div data-slot="comparison-list" className="space-y-4">
						{data.items.map((item) => (
							<div key={item.id} data-slot="comparison-item">
								<div
									data-slot="comparison-label"
									className="mb-1.5 flex justify-between gap-4 text-sm"
								>
									<span>{item.label}</span>
									<span className="shrink-0 font-medium tabular-nums">
										{item.valueLabel ??
											`${item.value}${data.unit ? ` ${data.unit}` : ""}`}
									</span>
								</div>
								<div
									role="meter"
									aria-label={item.label}
									aria-valuenow={item.value}
									aria-valuemin={0}
									aria-valuemax={data.max}
									aria-valuetext={
										item.valueLabel ??
										`${item.value}${data.unit ? ` ${data.unit}` : ""}`
									}
									data-slot="comparison-track"
									className="h-2 overflow-hidden rounded-full bg-muted"
								>
									<div
										data-slot="comparison-fill"
										className="h-full rounded-full bg-primary"
										style={{ width: `${(item.value / data.max) * 100}%` }}
									/>
								</div>
								{item.detail && (
									<p
										data-slot="comparison-detail"
										className="mt-1 text-xs text-muted-foreground"
									>
										{item.detail}
									</p>
								)}
							</div>
						))}
					</div>
				</div>
			)}
			{pending && data && !showSource && (
				<output
					data-slot="rich-media-continuation"
					className="mt-3 block h-1 w-12 rounded-full bg-muted motion-safe:animate-pulse"
				>
					<span className="sr-only">
						{t("buildingPreview", "Building preview")}
					</span>
				</output>
			)}
		</section>
	);
}
