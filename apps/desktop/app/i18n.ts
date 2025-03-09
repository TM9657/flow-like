import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import HttpApi from "i18next-http-backend";
import { initReactI18next } from "react-i18next";

i18n
	// detect user language
	// learn more: https://github.com/i18next/i18next-browser-languageDetector
	.use(HttpApi)
	.use(LanguageDetector)
	// pass the i18n instance to react-i18next.
	.use(initReactI18next)
	// init i18next
	// for all options read: https://www.i18next.com/overview/configuration-options
	.init({
		debug: true,
		lng: "en",
		fallbackLng: "en",
		backend: {
			loadPath: "/locales/{{lng}}/{{ns}}.json",
			allowMultiLoading: true,
		},
		interpolation: {
			escapeValue: false, // not needed for react as it escapes by default
		},
		saveMissing: false,
		saveMissingTo: "fallback",
	});

if (process.env.NODE_ENV === "development")
	i18n.on("missingKey", async (lngs, namespace, key, res) => {
		const url = "http://localhost:5544/";
		const data = {
			lngs,
			namespace,
			key,
			res,
		};
		const response = await fetch(url, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify(data),
		});
	});

export default i18n;
