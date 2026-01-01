import { de24h } from "./24h-de";
import { en24h } from "./24h-en";
import { es24h } from "./24h-es";
import { fr24h } from "./24h-fr";
import { it24h } from "./24h-it";
import { ja24h } from "./24h-ja";
import { ko24h } from "./24h-ko";
import { nl24h } from "./24h-nl";
import { pt24h } from "./24h-pt";
import { sv24h } from "./24h-sv";
import { zh24h } from "./24h-zh";

export const translations24h: Record<string, Record<string, string>> = {
	en: en24h,
	de: de24h,
	fr: fr24h,
	es: es24h,
	zh: zh24h,
	ja: ja24h,
	sv: sv24h,
	pt: pt24h,
	nl: nl24h,
	ko: ko24h,
	it: it24h,
};

export type Lang24h = keyof typeof translations24h;

export function t24h(lang: string, key: string): string {
	return translations24h[lang]?.[key] ?? translations24h.en[key] ?? key;
}
