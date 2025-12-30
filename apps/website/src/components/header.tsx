import { Button } from "@tm9657/flow-like-ui";
import { useEffect, useState } from "react";
import { BsDiscord, BsGithub, BsTwitterX } from "react-icons/bs";
import { LuBookHeart, LuBookMarked, LuDownload, LuZap } from "react-icons/lu";
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

export function Header() {
	const { t } = useTranslation();

	return (
		<header className="w-full flex flex-row items-center absolute top-0 left-0 right-0 h-16 z-20 backdrop-blur-sm shadow-md bg-background/40 justify-between">
			<a href="/" className="flex flex-row items-center px-2 gap-2">
				<img alt="logo" src="/icon.webp" className="h-12 w-12" />
				<h3 className="hidden sm:block">Flow Like</h3>
			</a>
			<div className="flex flex-row items-center px-2 gap-2">
				<a href="/24-hour-solution">
					<Button
						variant={"outline"}
						className="border-primary/50 text-primary hover:bg-primary/10"
					>
						<LuZap className="w-5 h-5" />
						<span className="hidden md:inline">{t("header.24h")}</span>
					</Button>
				</a>
				<a href="/blog/">
					<Button variant={"outline"}>
						<LuBookHeart width={5} height={5} className="w-5 h-5" />
						{t("header.blog")}
					</Button>
				</a>
				<a href="https://docs.flow-like.com" target="_blank" rel="noreferrer">
					<Button variant={"outline"}>
						<LuBookMarked className="w-5 h-5" />
						{t("header.docs")}
					</Button>
				</a>
				<a
					href="https://github.com/TM9657/flow-like"
					target="_blank"
					rel="noreferrer"
				>
					<Button variant={"outline"} size={"icon"}>
						<BsGithub width={5} height={5} className="w-5 h-5" />
					</Button>
				</a>
				<a href="https://x.com/greadco_de" target="_blank" rel="noreferrer">
					<Button variant={"outline"} size={"icon"}>
						<BsTwitterX width={5} height={5} className="w-5 h-5" />
					</Button>
				</a>
				<a
					href="https://discord.com/invite/KTWMrS2/"
					target="_blank"
					rel="noreferrer"
				>
					<Button variant={"outline"} size={"icon"}>
						<BsDiscord width={5} height={5} className="w-5 h-5" />
					</Button>
				</a>
				<a href="/download">
					<Button>
						<LuDownload className="w-5 h-5" />
						{t("header.download")}
					</Button>
				</a>
			</div>
		</header>
	);
}
