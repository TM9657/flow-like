import { deBi } from "./bi-de";
import { enBi } from "./bi-en";
import { esBi } from "./bi-es";
import { frBi } from "./bi-fr";
import { itBi } from "./bi-it";
import { jaBi } from "./bi-ja";
import { koBi } from "./bi-ko";
import { nlBi } from "./bi-nl";
import { ptBi } from "./bi-pt";
import { svBi } from "./bi-sv";
import { zhBi } from "./bi-zh";

export const translationsBi: Record<string, Record<string, string>> = {
	en: enBi,
	de: deBi,
	fr: frBi,
	es: esBi,
	zh: zhBi,
	ja: jaBi,
	sv: svBi,
	pt: ptBi,
	nl: nlBi,
	ko: koBi,
	it: itBi,
};

export type LangBi = keyof typeof translationsBi;

export function tBi(lang: string, key: string): string {
	return translationsBi[lang]?.[key] ?? translationsBi.en[key] ?? key;
}
