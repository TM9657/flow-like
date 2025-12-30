import { deDownload } from "./download-de";
import { enDownload } from "./download-en";
import { esDownload } from "./download-es";
import { frDownload } from "./download-fr";
import { itDownload } from "./download-it";
import { jaDownload } from "./download-ja";
import { koDownload } from "./download-ko";
import { nlDownload } from "./download-nl";
import { ptDownload } from "./download-pt";
import { svDownload } from "./download-sv";
import { zhDownload } from "./download-zh";

export const translationsDownload: Record<string, Record<string, string>> = {
	en: enDownload,
	de: deDownload,
	fr: frDownload,
	es: esDownload,
	zh: zhDownload,
	ja: jaDownload,
	sv: svDownload,
	pt: ptDownload,
	nl: nlDownload,
	ko: koDownload,
	it: itDownload,
};

export type LangDownload = keyof typeof translationsDownload;

export function tDownload(lang: string, key: string): string {
	return (
		translationsDownload[lang]?.[key] ?? translationsDownload.en[key] ?? key
	);
}
