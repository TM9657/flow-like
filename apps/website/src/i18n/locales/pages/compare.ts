import { deCompare } from "./compare-de";
import { enCompare } from "./compare-en";
import { esCompare } from "./compare-es";
import { frCompare } from "./compare-fr";

// For languages without specific translations, fall back to English
export const translationsCompare: Record<string, Record<string, string>> = {
	en: enCompare,
	de: deCompare,
	fr: frCompare,
	es: esCompare,
	zh: enCompare,
	ja: enCompare,
	sv: enCompare,
	pt: enCompare,
	nl: enCompare,
	ko: enCompare,
	it: enCompare,
};

export type LangCompare = keyof typeof translationsCompare;

export function tCompare(lang: string, key: string): string {
	return translationsCompare[lang]?.[key] ?? translationsCompare.en[key] ?? key;
}
