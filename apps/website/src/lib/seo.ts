export const SEO_LOCALES = [
	"en",
	"de",
	"es",
	"fr",
	"zh",
	"ja",
	"ko",
	"pt",
	"it",
	"nl",
	"sv",
] as const;

const LOCALE_SET = new Set<string>(SEO_LOCALES);

export type HreflangLink = {
	hreflang: string;
	href: string;
};

function normalizePath(pathname: string): string {
	if (!pathname || pathname === "/") return "/";
	const withLeadingSlash = pathname.startsWith("/") ? pathname : `/${pathname}`;
	return withLeadingSlash.endsWith("/") && withLeadingSlash.length > 1
		? withLeadingSlash.slice(0, -1)
		: withLeadingSlash;
}

function localeAgnosticPath(pathname: string): string {
	const normalized = normalizePath(pathname);
	const segments = normalized.split("/").filter(Boolean);
	if (segments.length > 0 && LOCALE_SET.has(segments[0])) {
		const rest = segments.slice(1).join("/");
		return rest ? `/${rest}` : "/";
	}
	return normalized;
}

export function buildAlternateLinks(
	site: string,
	pathname: string,
): HreflangLink[] {
	const siteBase = site.endsWith("/") ? site.slice(0, -1) : site;
	const path = localeAgnosticPath(pathname);
	const toHref = (locale: string): string =>
		locale === "en"
			? `${siteBase}${path}`
			: `${siteBase}/${locale}${path === "/" ? "" : path}`;

	return [
		...SEO_LOCALES.map((locale) => ({
			hreflang: locale,
			href: toHref(locale),
		})),
		{ hreflang: "x-default", href: toHref("en") },
	];
}

export function getOgLocale(lang: string): string {
	const map: Record<string, string> = {
		en: "en_US",
		de: "de_DE",
		es: "es_ES",
		fr: "fr_FR",
		zh: "zh_CN",
		ja: "ja_JP",
		ko: "ko_KR",
		pt: "pt_PT",
		it: "it_IT",
		nl: "nl_NL",
		sv: "sv_SE",
	};
	return map[lang] ?? "en_US";
}
