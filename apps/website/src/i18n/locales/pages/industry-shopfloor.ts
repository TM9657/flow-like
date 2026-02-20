import { enShopfloor } from "./industry-shopfloor-en";
import { deShopfloor } from "./industry-shopfloor-de";
import { frShopfloor } from "./industry-shopfloor-fr";
import { esShopfloor } from "./industry-shopfloor-es";
import { zhShopfloor } from "./industry-shopfloor-zh";
import { jaShopfloor } from "./industry-shopfloor-ja";
import { koShopfloor } from "./industry-shopfloor-ko";
import { ptShopfloor } from "./industry-shopfloor-pt";
import { itShopfloor } from "./industry-shopfloor-it";
import { nlShopfloor } from "./industry-shopfloor-nl";
import { svShopfloor } from "./industry-shopfloor-sv";

export const translationsShopfloor: Record<string, Record<string, string>> = {
	en: enShopfloor,
	de: deShopfloor,
	fr: frShopfloor,
	es: esShopfloor,
	zh: zhShopfloor,
	ja: jaShopfloor,
	ko: koShopfloor,
	pt: ptShopfloor,
	it: itShopfloor,
	nl: nlShopfloor,
	sv: svShopfloor,
};

export function tShopfloor(lang: string, key: string): string {
	return translationsShopfloor[lang]?.[key] ?? translationsShopfloor.en[key] ?? key;
}
