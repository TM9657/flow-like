import { enDevelopers } from "./developers-en";
import { deDevelopers } from "./developers-de";
import { frDevelopers } from "./developers-fr";
import { esDevelopers } from "./developers-es";
import { zhDevelopers } from "./developers-zh";
import { jaDevelopers } from "./developers-ja";
import { koDevelopers } from "./developers-ko";
import { ptDevelopers } from "./developers-pt";
import { itDevelopers } from "./developers-it";
import { nlDevelopers } from "./developers-nl";
import { svDevelopers } from "./developers-sv";

export const translationsDevelopers: Record<string, Record<string, string>> = {
	en: enDevelopers,
	de: deDevelopers,
	fr: frDevelopers,
	es: esDevelopers,
	zh: zhDevelopers,
	ja: jaDevelopers,
	ko: koDevelopers,
	pt: ptDevelopers,
	it: itDevelopers,
	nl: nlDevelopers,
	sv: svDevelopers,
};

export function tDevelopers(lang: string, key: string): string {
	return (
		translationsDevelopers[lang]?.[key] ??
		translationsDevelopers.en[key] ??
		key
	);
}
