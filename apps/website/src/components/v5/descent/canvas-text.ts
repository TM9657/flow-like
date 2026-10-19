// Line breaking and fitting for every canvas string the scene draws.

/** Width of `text` at font size `size`, in the units the caller lays out in. */
export type MeasureAt = (text: string, size: number) => number;

export interface FitOptions {
	readonly size: number;
	/** Smallest size the text may shrink to before it is ellipsized. */
	readonly floor?: number;
	readonly locale?: string;
}

export interface Fitted {
	readonly text: string;
	readonly size: number;
}

export interface WrapOptions extends FitOptions {
	readonly maxLines: number;
}

export interface Wrapped {
	readonly lines: readonly string[];
	readonly size: number;
}

const CJK = /[⺀-⿟　-〿぀-ヿ㄀-ㄯㆠ-ㇿ㐀-䶿一-鿿豈-﫿︰-﹏＀-￯]/;
// Kinsoku: characters that may not start a line, and ones that may not end it.
const CLOSING =
	"、。，．,.!?！？:;：；)\\]}）」』】〉》〕ー…・々ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ%％";
const OPENING = "(（「『【〈《〔\\[{";
const NO_START = new RegExp(`^[\\s${CLOSING}]`);
const NO_END = new RegExp(`[${OPENING}]$`);
const DASHES = "\\-‐–—/";
const BREAK_AFTER = new RegExp(`[\\s${DASHES}]$`);
const ELLIPSIS = "…";

const segmenters = new Map<string, Intl.Segmenter | null>();

function segmenter(
	locale: string | undefined,
	granularity: "word" | "grapheme",
) {
	const key = `${locale ?? ""}|${granularity}`;
	let s = segmenters.get(key);
	if (s === undefined) {
		s =
			typeof Intl !== "undefined" && "Segmenter" in Intl
				? new Intl.Segmenter(locale, { granularity })
				: null;
		segmenters.set(key, s);
	}
	return s;
}

function graphemes(text: string, locale?: string): string[] {
	const s = segmenter(locale, "grapheme");
	if (!s) return Array.from(text);
	return Array.from(s.segment(text), (g) => g.segment);
}

function words(text: string, locale?: string): string[] {
	const s = segmenter(locale, "word");
	if (s) return Array.from(s.segment(text), (w) => w.segment);
	return text
		.split(/(\s+)/)
		.filter(Boolean)
		.flatMap((part) => (CJK.test(part) ? Array.from(part) : [part]));
}

/**
 * Splits text into runs that must stay on one line. A line may break after
 * white space or a dash, and between CJK words, never before closing
 * punctuation or after an opening bracket.
 */
export function breakRuns(text: string, locale?: string) {
	const runs: string[] = [];
	let run = "";
	for (const seg of words(text, locale)) {
		if (canBreak(run, seg)) {
			runs.push(run);
			run = seg;
		} else run += seg;
	}
	if (run) runs.push(run);
	return runs;
}

const isCJK = (ch: string | undefined) => ch !== undefined && CJK.test(ch);
const kinsoku = (run: string, next: string) =>
	NO_START.test(next) || NO_END.test(run);

function canBreak(run: string, next: string) {
	if (!run || kinsoku(run, next)) return false;
	return BREAK_AFTER.test(run) || isCJK(run.at(-1)) || isCJK(next[0]);
}

/** The longest prefix of `text` that fits `maxW` with an ellipsis, or the text itself if it fits. */
export function ellipsize(
	measure: (text: string) => number,
	text: string,
	maxW: number,
	locale?: string,
): string {
	if (measure(text) <= maxW) return text;
	const g = graphemes(text, locale);
	let lo = 0;
	let hi = g.length;
	while (lo < hi) {
		const mid = (lo + hi + 1) >> 1;
		if (measure(`${g.slice(0, mid).join("").trimEnd()}${ELLIPSIS}`) <= maxW)
			lo = mid;
		else hi = mid - 1;
	}
	return `${g.slice(0, lo).join("").trimEnd()}${ELLIPSIS}`;
}

/** Greedy line breaking over breakRuns; a run wider than a line is split between graphemes. */
export function wrapLines(
	measure: (text: string) => number,
	text: string,
	maxW: number,
	locale?: string,
) {
	const lines: string[] = [];
	let line = "";
	const pushRun = (run: string) => {
		for (const g of graphemes(run, locale)) {
			if (line && measure(`${line}${g}`.trimEnd()) > maxW) {
				lines.push(line.trimEnd());
				line = g.trimStart();
			} else line += g;
		}
	};
	for (const run of breakRuns(text.trim(), locale)) {
		const next = `${line}${run}`;
		if (measure(next.trimEnd()) <= maxW) {
			line = next;
		} else if (!line) {
			pushRun(run);
		} else {
			lines.push(line.trimEnd());
			line = "";
			if (measure(run.trimEnd()) <= maxW) line = run;
			else pushRun(run);
		}
	}
	if (line.trim()) lines.push(line.trimEnd());
	return lines;
}

function sizesFrom(size: number, floor: number) {
	const out: number[] = [];
	for (let s = size; s > floor + 1e-6; s -= 0.25) out.push(s);
	out.push(floor);
	return out;
}

/** Shrinks one line to fit `maxW`, down to `floor`, then ellipsizes at the floor. */
export function fitLine(
	measureAt: MeasureAt,
	text: string,
	maxW: number,
	{ size, floor = size, locale }: FitOptions,
): Fitted {
	const full = measureAt(text, size);
	if (full <= maxW) return { text, size };
	const low = Math.min(size, floor);
	const guess = Math.max(low, Math.min(size, (size * maxW) / full));
	for (const s of sizesFrom(guess, low)) {
		if (measureAt(text, s) <= maxW) return { text, size: s };
	}
	return {
		text: ellipsize((t) => measureAt(t, low), text, maxW, locale),
		size: low,
	};
}

/** Wraps into at most `maxLines`, shrinking down to `floor` first, then ellipsizing the last line. */
export function fitLines(
	measureAt: MeasureAt,
	text: string,
	maxW: number,
	{ size, floor = size, maxLines, locale }: WrapOptions,
): Wrapped {
	const low = Math.min(size, floor);
	for (const s of sizesFrom(size, low)) {
		const lines = wrapLines((t) => measureAt(t, s), text, maxW, locale);
		if (lines.length <= maxLines) return { lines, size: s };
	}
	const lines = wrapLines((t) => measureAt(t, low), text, maxW, locale);
	const kept = lines.slice(0, maxLines);
	const rest = lines.slice(maxLines - 1).join(CJK.test(text) ? "" : " ");
	kept[maxLines - 1] = ellipsize((t) => measureAt(t, low), rest, maxW, locale);
	return { lines: kept, size: low };
}
