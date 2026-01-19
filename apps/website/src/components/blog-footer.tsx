import { useEffect, useState } from "react";
import { translationsCommon } from "../i18n/locales/pages/common";

type Lang =
	| "en"
	| "de"
	| "es"
	| "fr"
	| "zh"
	| "ja"
	| "ko"
	| "pt"
	| "it"
	| "nl"
	| "sv";

function useTranslation() {
	const [lang, setLang] = useState<Lang>("en");

	useEffect(() => {
		if (typeof window === "undefined") return;
		const path = window.location.pathname;
		const langs: Lang[] = [
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
		];
		for (const l of langs) {
			if (path.startsWith(`/${l}/`) || path === `/${l}`) {
				setLang(l);
				return;
			}
		}
		setLang("en");
	}, []);

	const t = (key: string): string => {
		return translationsCommon[lang]?.[key] ?? translationsCommon.en[key] ?? key;
	};

	return { t };
}

export function BlogFooter() {
	const { t } = useTranslation();

	return (
		<footer className="w-full flex flex-row items-center h-10 z-20 bg-transparent justify-between px-2 gap-2">
			<div>
				<small>{t("footer.copyright")}</small>
			</div>
			<div className="flex flex-row items-center gap-2">
				<a href="/eula">
					<small>{t("footer.eula")}</small>
				</a>
				<a href="/privacy-policy">
					<small>{t("footer.privacy")}</small>
				</a>
				<a
					href="https://great-co.de/legal-notice"
					target="_blank"
					rel="noreferrer"
				>
					<small>{t("footer.legal")}</small>
				</a>
			</div>
		</footer>
	);
}
