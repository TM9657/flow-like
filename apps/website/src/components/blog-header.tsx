import { Button } from "@tm9657/flow-like-ui";
import { Globe, X } from "lucide-react";
import type React from "react";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { BsDiscord, BsGithub, BsTwitterX } from "react-icons/bs";
import {
	LuBookHeart,
	LuBookMarked,
	LuChartBarStacked,
	LuDownload,
	LuZap,
} from "react-icons/lu";
import { translationsCommon } from "../i18n/locales/pages/common";

const languages = {
	en: "English",
	de: "Deutsch",
	es: "Español",
	fr: "Français",
	zh: "中文",
	ja: "日本語",
	ko: "한국어",
	pt: "Português",
	it: "Italiano",
	nl: "Nederlands",
	sv: "Svenska",
} as const;

const langFlags: Record<string, string> = {
	en: "🇺🇸",
	de: "🇩🇪",
	es: "🇪🇸",
	fr: "🇫🇷",
	zh: "🇨🇳",
	ja: "🇯🇵",
	ko: "🇰🇷",
	pt: "🇧🇷",
	it: "🇮🇹",
	nl: "🇳🇱",
	sv: "🇸🇪",
};

type Lang = keyof typeof languages;

const STORAGE_KEY = "flow-like-lang-preference";

function saveLangPreference(lang: Lang) {
	try {
		localStorage.setItem(STORAGE_KEY, lang);
	} catch (e) {
		// localStorage not available
	}
}

function detectCurrentLang(): Lang {
	if (typeof window === "undefined") return "en";
	const path = window.location.pathname;
	for (const lang of Object.keys(languages) as Lang[]) {
		if (path.startsWith(`/${lang}/`) || path === `/${lang}`) {
			return lang;
		}
	}
	return "en";
}

function useTranslation() {
	const [lang, setLang] = useState<Lang>("en");

	useEffect(() => {
		setLang(detectCurrentLang());
	}, []);

	const t = (key: string): string => {
		return translationsCommon[lang]?.[key] ?? translationsCommon.en[key] ?? key;
	};

	return { lang, t };
}

function LanguageSwitcher() {
	const [open, setOpen] = useState(false);
	const [currentLang, setCurrentLang] = useState<Lang>("en");
	const [isMobile, setIsMobile] = useState(false);
	const { t } = useTranslation();

	useEffect(() => {
		const path = window.location.pathname;
		for (const lang of Object.keys(languages) as Lang[]) {
			if (path.startsWith(`/${lang}/`) || path === `/${lang}`) {
				setCurrentLang(lang);
				return;
			}
		}
		setCurrentLang("en");
	}, []);

	useEffect(() => {
		const checkMobile = () => setIsMobile(window.innerWidth < 640);
		checkMobile();
		window.addEventListener("resize", checkMobile);
		return () => window.removeEventListener("resize", checkMobile);
	}, []);

	useEffect(() => {
		if (open && isMobile) {
			document.body.style.overflow = "hidden";
			return () => {
				document.body.style.overflow = "";
			};
		}
	}, [open, isMobile]);

	const getLocalizedPath = (targetLang: Lang) => {
		let path = window.location.pathname;
		for (const l of Object.keys(languages)) {
			if (path.startsWith(`/${l}/`) || path === `/${l}`) {
				path = path.slice(l.length + 1) || "/";
				break;
			}
		}
		if (targetLang === "en") {
			return path || "/";
		}
		return `/${targetLang}${path === "/" ? "" : path}`;
	};

	return (
		<div className="relative">
			<button
				type="button"
				onClick={() => setOpen(!open)}
				className="group flex items-center gap-2 rounded-full border border-border/40 bg-background/60 px-3 py-1.5 text-sm font-medium backdrop-blur-md transition-all hover:border-primary/50 hover:bg-background/80 hover:shadow-lg hover:shadow-primary/5"
				aria-label="Select language"
			>
				<span className="text-base leading-none">{langFlags[currentLang]}</span>
				<span className="hidden sm:inline uppercase text-foreground/70 group-hover:text-foreground transition-colors">
					{currentLang}
				</span>
				<svg
					className="size-3 opacity-50 transition-transform group-hover:opacity-80"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					strokeWidth="2"
				>
					<path
						strokeLinecap="round"
						strokeLinejoin="round"
						d="M19 9l-7 7-7-7"
					/>
				</svg>
			</button>

			{open && !isMobile && (
				<>
					<button
						type="button"
						className="fixed inset-0 z-40"
						onClick={() => setOpen(false)}
						aria-label="Close language menu"
					/>
					<div className="absolute right-0 top-full z-50 mt-2 animate-in fade-in slide-in-from-top-2 duration-200">
						<div className="grid grid-cols-2 gap-1 p-2 rounded-xl border border-border/50 bg-background/95 shadow-xl shadow-black/10 backdrop-blur-lg min-w-[240px]">
							{(Object.entries(languages) as [Lang, string][]).map(
								([code, name]) => (
									<a
										key={code}
										href={getLocalizedPath(code)}
										className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-all ${
											code === currentLang
												? "bg-primary/10 text-primary font-medium ring-1 ring-primary/20"
												: "text-foreground/70 hover:bg-muted hover:text-foreground"
										}`}
										onClick={() => {
											saveLangPreference(code);
											setOpen(false);
										}}
									>
										<span className="text-lg">{langFlags[code]}</span>
										<span>{name}</span>
									</a>
								),
							)}
						</div>
					</div>
				</>
			)}

			{open &&
				isMobile &&
				createPortal(
					<>
						<div
							className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm animate-in fade-in duration-300"
							onClick={() => setOpen(false)}
						/>
						<div className="fixed inset-x-0 bottom-0 z-50 animate-in slide-in-from-bottom duration-300">
							<div
								className="bg-background rounded-t-2xl border-t border-border/50 shadow-2xl"
								style={{
									paddingBottom: "max(1rem, env(safe-area-inset-bottom))",
								}}
							>
								<div className="flex justify-center pt-3 pb-2">
									<div className="w-10 h-1 rounded-full bg-muted-foreground/30" />
								</div>
								<div className="px-4 pb-2">
									<h3 className="text-lg font-semibold text-foreground">
										{t("header.selectLanguage")}
									</h3>
								</div>
								<div className="grid grid-cols-2 gap-2 p-4 pt-2 max-h-[60vh] overflow-y-auto">
									{(Object.entries(languages) as [Lang, string][]).map(
										([code, name]) => (
											<a
												key={code}
												href={getLocalizedPath(code)}
												className={`flex items-center gap-3 px-4 py-3 rounded-xl text-base transition-all active:scale-95 ${
													code === currentLang
														? "bg-primary/10 text-primary font-medium ring-2 ring-primary/30"
														: "bg-muted/50 text-foreground/80 hover:bg-muted"
												}`}
												onClick={() => {
													saveLangPreference(code);
													setOpen(false);
												}}
											>
												<span className="text-2xl">{langFlags[code]}</span>
												<span>{name}</span>
											</a>
										),
									)}
								</div>
								<div className="p-4 pt-0">
									<button
										type="button"
										onClick={() => setOpen(false)}
										className="w-full py-3 rounded-xl bg-muted text-foreground/70 font-medium transition-colors hover:bg-muted/80 active:scale-[0.98]"
									>
										{t("header.cancel")}
									</button>
								</div>
							</div>
						</div>
					</>,
					document.body,
				)}
		</div>
	);
}

export function BlogHeader() {
	const [open, setOpen] = useState(false);
	const [mounted, setMounted] = useState(false);
	const { t } = useTranslation();

	useEffect(() => setMounted(true), []);

	// lock background scroll when menu is open
	useEffect(() => {
		const root = document.documentElement;
		if (open) {
			const prev = root.style.overflow;
			root.style.overflow = "hidden";
			return () => {
				root.style.overflow = prev;
			};
		}
	}, [open]);

	const handleNavLinkClick = (_e?: React.MouseEvent) => {
		// let the navigation start before closing
		setTimeout(() => setOpen(false), 150);
	};

	const Hamburger: React.FC<{
		open: boolean;
		className?: string;
		onClick?: () => void;
	}> = ({ open, className, onClick }) => (
		<button
			type="button"
			aria-label={open ? "Close menu" : "Open menu"}
			onClick={onClick}
			className={`relative w-10 h-10 inline-flex items-center justify-center rounded-md hover:bg-muted/50 transition-colors ${className ?? ""}`}
		>
			<span
				className={`block absolute w-5 h-0.5 bg-foreground rounded-full transition-all duration-300 ease-in-out ${
					open ? "rotate-45" : "-translate-y-1.5"
				}`}
			/>
			<span
				className={`block absolute w-5 h-0.5 bg-foreground rounded-full transition-all duration-200 ease-in-out ${
					open ? "opacity-0 scale-0" : "opacity-100 scale-100"
				}`}
			/>
			<span
				className={`block absolute w-5 h-0.5 bg-foreground rounded-full transition-all duration-300 ease-in-out ${
					open ? "-rotate-45" : "translate-y-1.5"
				}`}
			/>
		</button>
	);

	// ---- Portal overlay (renders to <body>) ----
	const MobileOverlay = mounted
		? createPortal(
				<div
					className={`fixed inset-0 z-[100] sm:hidden transition-opacity duration-300 ${
						open
							? "opacity-100 pointer-events-auto"
							: "opacity-0 pointer-events-none"
					}`}
					role="dialog"
					aria-modal="true"
					aria-hidden={!open}
				>
					{/* Backdrop */}
					<button
						aria-label="Close menu backdrop"
						onClick={() => setOpen(false)}
						className="absolute inset-0 w-full h-full bg-black/30 supports-backdrop-filter:backdrop-blur-sm"
					/>
					{/* Panel */}
					<div
						className={`relative w-full max-h-[85vh] overflow-auto bg-background shadow-lg transition-transform duration-300 ${
							open ? "translate-y-0" : "-translate-y-4"
						}`}
						onClick={(e) => e.stopPropagation()}
					>
						<div className="flex items-center justify-between p-4">
							<a
								href="/"
								className="flex items-center gap-2"
								onClick={handleNavLinkClick}
							>
								<img alt="logo" src="/icon.webp" className="h-10 w-10" />
								<span className="font-semibold">Flow Like</span>
							</a>
							<button
								aria-label="Close menu"
								onClick={() => setOpen(false)}
								className="p-2 rounded-md hover:bg-background/50"
							>
								<X className="w-6 h-6" />
							</button>
						</div>

						<nav className="px-6 pb-6 space-y-4">
							<a
								href="/modern-bi"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md border border-emerald-500/50 text-emerald-600 font-medium transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "15ms" : "0ms" }}
							>
								<LuChartBarStacked className="w-5 h-5" />
								<span>Flow-Like for BI</span>
							</a>

							<a
								href="/24-hour-solution"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md border border-primary/50 text-primary font-medium transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "30ms" : "0ms" }}
							>
								<LuZap className="w-5 h-5" />
								<span>{t("header.24h")}</span>
							</a>

							<a
								href="/blog/"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md bg-primary text-primary-foreground font-medium transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "60ms" : "0ms" }}
							>
								<LuBookHeart className="w-5 h-5" />
								<span>{t("header.blog")}</span>
							</a>

							<a
								href="https://docs.flow-like.com"
								target="_blank"
								rel="noreferrer"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "110ms" : "0ms" }}
							>
								<LuBookMarked className="w-5 h-5" />
								<span>{t("header.docs")}</span>
							</a>

							<a
								href="https://github.com/TM9657/flow-like"
								target="_blank"
								rel="noreferrer"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "160ms" : "0ms" }}
							>
								<BsGithub className="w-5 h-5" />
								<span>GitHub</span>
							</a>

							<a
								href="https://x.com/greatco_de"
								target="_blank"
								rel="noreferrer"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "210ms" : "0ms" }}
							>
								<BsTwitterX className="w-5 h-5" />
								<span>X</span>
							</a>

							<a
								href="https://discord.com/invite/KTWMrS2/"
								target="_blank"
								rel="noreferrer"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "260ms" : "0ms" }}
							>
								<BsDiscord className="w-5 h-5" />
								<span>Discord</span>
							</a>

							<a
								href="/download"
								onClick={handleNavLinkClick}
								className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "310ms" : "0ms" }}
							>
								<LuDownload className="w-5 h-5" />
								<span>{t("header.download")}</span>
							</a>

							{/* Language selection in mobile menu */}
							<div
								className={`pt-4 border-t border-border/50 transition transform ${
									open
										? "opacity-100 translate-x-0"
										: "opacity-0 -translate-x-2"
								}`}
								style={{ transitionDelay: open ? "360ms" : "0ms" }}
							>
								<div className="flex items-center gap-2 px-4 py-2 text-sm text-muted-foreground">
									<Globe className="w-4 h-4" />
									<span>{t("header.language")}</span>
								</div>
								<div className="grid grid-cols-3 gap-2 px-4">
									{(Object.entries(languages) as [Lang, string][]).map(
										([code, name]) => {
											const isCurrentLang = (() => {
												const path =
													typeof window !== "undefined"
														? window.location.pathname
														: "";
												for (const l of Object.keys(languages)) {
													if (path.startsWith(`/${l}/`) || path === `/${l}`) {
														return code === l;
													}
												}
												return code === "en";
											})();
											return (
												<a
													key={code}
													href={(() => {
														const path =
															typeof window !== "undefined"
																? window.location.pathname
																: "/";
														let cleanPath = path;
														for (const l of Object.keys(languages)) {
															if (
																cleanPath.startsWith(`/${l}/`) ||
																cleanPath === `/${l}`
															) {
																cleanPath =
																	cleanPath.slice(l.length + 1) || "/";
																break;
															}
														}
														if (code === "en") return cleanPath || "/";
														return `/${code}${cleanPath === "/" ? "" : cleanPath}`;
													})()}
													onClick={() => {
														saveLangPreference(code);
														handleNavLinkClick();
													}}
													className={`flex items-center justify-center gap-1.5 px-2 py-2 rounded-lg text-sm transition-all ${
														isCurrentLang
															? "bg-primary/10 text-primary font-medium ring-1 ring-primary/20"
															: "bg-muted/50 text-foreground/80 hover:bg-muted"
													}`}
												>
													<span className="text-base">{langFlags[code]}</span>
													<span className="uppercase text-xs">{code}</span>
												</a>
											);
										},
									)}
								</div>
							</div>
						</nav>
					</div>
				</div>,
				document.body,
			)
		: null;

	return (
		<>
			<header className="w-full flex flex-row items-center sticky top-0 left-0 right-0 min-h-16 h-16 z-50 backdrop-blur-sm shadow-md bg-background/80 justify-between px-2">
				<a href="/" className="flex flex-row items-center gap-2 shrink-0">
					<img alt="logo" src="/icon.webp" className="h-12 w-12" />
					<h3 className="hidden sm:block">Flow Like</h3>
				</a>

				{/* Desktop nav (lg+) */}
				<div className="hidden lg:flex flex-row items-center gap-2">
					<a href="/modern-bi">
						<Button
							variant={"outline"}
							className="border-emerald-500/50 text-emerald-600 hover:bg-emerald-500/10"
						>
							<LuChartBarStacked className="w-5 h-5" />
							Flow-Like for BI
						</Button>
					</a>
					<a href="/24-hour-solution">
						<Button
							variant={"outline"}
							className="border-primary/50 text-primary hover:bg-primary/10"
						>
							<LuZap className="w-5 h-5" />
							{t("header.24h")}
						</Button>
					</a>
					<a href="/blog/">
						<Button variant={"outline"}>
							<LuBookHeart className="w-5 h-5" />
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
							<BsGithub className="w-5 h-5" />
						</Button>
					</a>
					<a href="https://x.com/greatco_de" target="_blank" rel="noreferrer">
						<Button variant={"outline"} size={"icon"}>
							<BsTwitterX className="w-5 h-5" />
						</Button>
					</a>
					<a
						href="https://discord.com/invite/KTWMrS2/"
						target="_blank"
						rel="noreferrer"
					>
						<Button variant={"outline"} size={"icon"}>
							<BsDiscord className="w-5 h-5" />
						</Button>
					</a>
					<a href="/download">
						<Button>
							<LuDownload className="w-5 h-5" />
							{t("header.download")}
						</Button>
					</a>
					<LanguageSwitcher />
				</div>

				{/* Tablet nav (sm to lg) */}
				<div className="hidden sm:flex lg:hidden flex-row items-center gap-1.5">
					<a href="/modern-bi">
						<Button
							variant={"outline"}
							size={"icon"}
							className="border-emerald-500/50 text-emerald-600 hover:bg-emerald-500/10"
						>
							<LuChartBarStacked className="w-5 h-5" />
						</Button>
					</a>
					<a href="/24-hour-solution">
						<Button
							variant={"outline"}
							size={"icon"}
							className="border-primary/50 text-primary hover:bg-primary/10"
						>
							<LuZap className="w-5 h-5" />
						</Button>
					</a>
					<a href="/blog/">
						<Button variant={"outline"} size={"icon"}>
							<LuBookHeart className="w-5 h-5" />
						</Button>
					</a>
					<a href="https://docs.flow-like.com" target="_blank" rel="noreferrer">
						<Button variant={"outline"} size={"icon"}>
							<LuBookMarked className="w-5 h-5" />
						</Button>
					</a>
					<a
						href="https://github.com/TM9657/flow-like"
						target="_blank"
						rel="noreferrer"
					>
						<Button variant={"outline"} size={"icon"}>
							<BsGithub className="w-5 h-5" />
						</Button>
					</a>
					<a href="https://x.com/greatco_de" target="_blank" rel="noreferrer">
						<Button variant={"outline"} size={"icon"}>
							<BsTwitterX className="w-5 h-5" />
						</Button>
					</a>
					<a
						href="https://discord.com/invite/KTWMrS2/"
						target="_blank"
						rel="noreferrer"
					>
						<Button variant={"outline"} size={"icon"}>
							<BsDiscord className="w-5 h-5" />
						</Button>
					</a>
					<a href="/download">
						<Button size={"icon"}>
							<LuDownload className="w-5 h-5" />
						</Button>
					</a>
					<LanguageSwitcher />
				</div>

				{/* Mobile controls */}
				<div className="flex items-center gap-2 sm:hidden">
					<LanguageSwitcher />
					<Hamburger open={open} onClick={() => setOpen((s) => !s)} />
				</div>
			</header>

			{/* Portal overlay */}
			{MobileOverlay}
		</>
	);
}
