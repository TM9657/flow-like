import { deCommon } from "./common-de";
import { enCommon } from "./common-en";
import { esCommon } from "./common-es";
import { frCommon } from "./common-fr";
import { itCommon } from "./common-it";
import { jaCommon } from "./common-ja";
import { koCommon } from "./common-ko";
import { nlCommon } from "./common-nl";
import { ptCommon } from "./common-pt";
import { svCommon } from "./common-sv";
import { zhCommon } from "./common-zh";

export const translationsCommon: Record<string, Record<string, string>> = {
	en: enCommon,
	de: deCommon,
	fr: frCommon,
	es: esCommon,
	zh: zhCommon,
	ja: jaCommon,
	sv: svCommon,
	pt: ptCommon,
	nl: nlCommon,
	ko: koCommon,
	it: itCommon,
};

export type LangCommon = keyof typeof translationsCommon;

export function tCommon(lang: string, key: string): string {
	return translationsCommon[lang]?.[key] ?? translationsCommon.en[key] ?? key;
}

// Helper to detect current language from URL path
export function detectLangFromPath(path: string): string {
	const supportedLangs = Object.keys(translationsCommon);
	for (const lang of supportedLangs) {
		if (path.startsWith(`/${lang}/`) || path === `/${lang}`) {
			return lang;
		}
	}
	return "en";
}
