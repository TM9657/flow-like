import { deDataDeletion } from "./data-deletion-de";
import { enDataDeletion } from "./data-deletion-en";
import { esDataDeletion } from "./data-deletion-es";
import { frDataDeletion } from "./data-deletion-fr";
import { itDataDeletion } from "./data-deletion-it";
import { jaDataDeletion } from "./data-deletion-ja";
import { koDataDeletion } from "./data-deletion-ko";
import { nlDataDeletion } from "./data-deletion-nl";
import { ptDataDeletion } from "./data-deletion-pt";
import { svDataDeletion } from "./data-deletion-sv";
import { zhDataDeletion } from "./data-deletion-zh";

export const translationsDataDeletion: Record<
	string,
	Record<string, string>
> = {
	en: enDataDeletion,
	de: deDataDeletion,
	fr: frDataDeletion,
	es: esDataDeletion,
	zh: zhDataDeletion,
	ja: jaDataDeletion,
	sv: svDataDeletion,
	pt: ptDataDeletion,
	nl: nlDataDeletion,
	ko: koDataDeletion,
	it: itDataDeletion,
};

export type LangDataDeletion = keyof typeof translationsDataDeletion;

export function tDataDeletion(lang: string, key: string): string {
	return (
		translationsDataDeletion[lang]?.[key] ??
		translationsDataDeletion.en[key] ??
		key
	);
}
