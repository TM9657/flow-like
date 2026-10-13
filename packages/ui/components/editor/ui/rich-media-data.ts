import { isSafeEditorUrl } from "../plugins/safe-url-kit";
import { parsePartialJson } from "./partial-json";

export const RICH_MEDIA_LANGUAGES = [
	"gallery",
	"recommendations",
	"comparison",
] as const;
export type RichMediaLanguage = (typeof RICH_MEDIA_LANGUAGES)[number];

export interface RichMediaImage {
	src?: string;
	alt: string;
	caption?: string;
	credit?: string;
	href?: string;
}

interface Identified {
	id: string;
}
export interface GalleryData {
	kind: "gallery";
	title?: string;
	items: (RichMediaImage & Identified)[];
}
export interface RecommendationItem extends Identified {
	rank: number;
	title: string;
	subtitle?: string;
	badge?: string;
	description?: string;
	image?: RichMediaImage;
	specs?: { label: string; value: string }[];
	href?: string;
}
export interface RecommendationsData {
	kind: "recommendations";
	title?: string;
	items: RecommendationItem[];
}
export interface ComparisonData {
	kind: "comparison";
	title?: string;
	description?: string;
	max: number;
	unit?: string;
	items: (Identified & {
		label: string;
		value: number;
		valueLabel?: string;
		detail?: string;
	})[];
}
export type RichMediaData = GalleryData | RecommendationsData | ComparisonData;

export function isRichMediaLanguage(
	language: string | undefined,
): language is RichMediaLanguage {
	return RICH_MEDIA_LANGUAGES.some((item) => item === language);
}

function record(value: unknown): Record<string, unknown> {
	if (!value || typeof value !== "object" || Array.isArray(value))
		throw new Error("Expected an object.");
	return value as Record<string, unknown>;
}

function text(value: unknown, required = false): string | undefined {
	if (value === undefined && !required) return undefined;
	if (typeof value !== "string" || (required && !value.trim()))
		throw new Error("Expected text.");
	return value;
}

function requiredText(value: unknown): string {
	return text(value, true) as string;
}

function items(value: unknown): unknown[] {
	if (!Array.isArray(value) || value.length === 0)
		throw new Error("Expected at least one item.");
	return value;
}

/** Match editor URL safety, then restrict navigation and image protocols. */
function safeUrl(value: unknown, image = false): string | undefined {
	const url = text(value);
	if (!url || !isSafeEditorUrl(url)) return undefined;
	try {
		const parsed = new URL(url, "https://editor.invalid/");
		if (["https:", "http:"].includes(parsed.protocol)) return url;
		if (image && ["blob:", "storage:"].includes(parsed.protocol)) return url;
		if (
			image &&
			/^data:image\/(?:png|jpeg|jpg|webp|gif);base64,[a-z\d+/=]+$/i.test(url)
		)
			return url;
	} catch {
		/* Invalid URLs leave the text and image description readable. */
	}
	return undefined;
}

function parseImage(value: unknown): RichMediaImage {
	const image = record(value);
	requiredText(image.src);
	return {
		src: safeUrl(image.src, true),
		alt: requiredText(image.alt),
		caption: text(image.caption),
		credit: text(image.credit),
		href: safeUrl(image.href),
	};
}

/** Fences remain plain JSON; strings never become HTML or executable JSX. */
export function parseRichMediaData(
	content: string,
	language: RichMediaLanguage,
): RichMediaData {
	return parseRichMediaValue(JSON.parse(content), language);
}

/** Only streaming callers may render completed fields from an open JSON object. */
export function parseStreamingRichMediaData(
	content: string,
	language: RichMediaLanguage,
): RichMediaData | null {
	const partial = parsePartialJson(content, { allowPartialArrayItems: true });
	if (!partial || typeof partial !== "object" || Array.isArray(partial))
		return null;
	const source = partial as Record<string, unknown>;
	if (!Array.isArray(source.items)) return null;
	const sourceIndexes: number[] = [];
	const validItems = source.items.flatMap((entry, index) => {
		let item = entry;
		if (
			language === "recommendations" &&
			entry &&
			typeof entry === "object" &&
			!Array.isArray(entry)
		) {
			const candidate = { ...entry } as Record<string, unknown>;
			// Optional structures arrive field by field. Keep an already readable
			// recommendation while its image or next specification is still open.
			try {
				parseImage(candidate.image);
			} catch {
				candidate.image = undefined;
			}
			if (Array.isArray(candidate.specs)) {
				candidate.specs = candidate.specs.filter((entry) => {
					try {
						const spec = record(entry);
						requiredText(spec.label);
						requiredText(spec.value);
						return true;
					} catch {
						return false;
					}
				});
			} else candidate.specs = undefined;
			item = candidate;
		}
		try {
			parseRichMediaValue({ ...source, items: [item] }, language);
			sourceIndexes.push(index);
			return [item];
		} catch {
			return [];
		}
	});
	if (!validItems.length) return null;
	try {
		const data = parseRichMediaValue(
			{ ...source, items: validItems },
			language,
		);
		const prefix = {
			gallery: "image",
			recommendations: "recommendation",
			comparison: "comparison",
		}[language];
		data.items.forEach((item, index) => {
			item.id = `${prefix}-${sourceIndexes[index]}`;
		});
		if (data.kind === "recommendations")
			data.items.forEach((item, index) => {
				item.rank = sourceIndexes[index] + 1;
			});
		return data;
	} catch {
		return null;
	}
}

function parseRichMediaValue(
	value: unknown,
	language: RichMediaLanguage,
): RichMediaData {
	const source = record(value);
	const title = text(source.title);
	const entries = items(source.items);
	if (language === "gallery") {
		return {
			kind: language,
			title,
			items: entries.map((item, index) => ({
				...parseImage(item),
				id: `image-${index}`,
			})),
		};
	}
	if (language === "recommendations") {
		return {
			kind: language,
			title,
			items: entries.map((entry, index) => {
				const item = record(entry);
				if (item.specs !== undefined && !Array.isArray(item.specs))
					throw new Error("Expected a list of specifications.");
				return {
					id: `recommendation-${index}`,
					rank: index + 1,
					title: requiredText(item.title),
					subtitle: text(item.subtitle),
					badge: text(item.badge),
					description: text(item.description),
					image: item.image === undefined ? undefined : parseImage(item.image),
					specs: (item.specs as unknown[] | undefined)?.map((value) => {
						const spec = record(value);
						return {
							label: requiredText(spec.label),
							value: requiredText(spec.value),
						};
					}),
					href: safeUrl(item.href),
				};
			}),
		};
	}
	if (
		typeof source.max !== "number" ||
		!Number.isFinite(source.max) ||
		source.max <= 0
	)
		throw new Error("Expected a positive comparison maximum.");
	const max = source.max;
	return {
		kind: language,
		title,
		max,
		description: text(source.description),
		unit: text(source.unit),
		items: entries.map((entry, index) => {
			const item = record(entry);
			if (
				typeof item.value !== "number" ||
				!Number.isFinite(item.value) ||
				item.value < 0 ||
				item.value > max
			)
				throw new Error("Comparison values must be within the declared scale.");
			return {
				id: `comparison-${index}`,
				label: requiredText(item.label),
				value: item.value,
				valueLabel: text(item.valueLabel),
				detail: text(item.detail),
			};
		}),
	};
}
