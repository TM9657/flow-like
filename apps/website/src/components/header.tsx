import { ChartBar, ChevronDown, Menu, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { BsDiscord, BsGithub } from "react-icons/bs";
import {
	LuArrowRight,
	LuBookMarked,
	LuBookOpen,
	LuBot,
	LuBuilding2,
	LuDownload,
	LuExternalLink,
	LuFactory,
	LuFileStack,
	LuFileText,
	LuGlobe,
	LuLandmark,
	LuScale,
	LuServer,
	LuShieldCheck,
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
const webAppUrl = "https://app.flow-like.com";
const studioName = "Flow-Like Studio";

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

function useTranslation() {
	const [lang, setLang] = useState<Lang>("en");

	useEffect(() => {
		if (typeof window === "undefined") return;
		const path = window.location.pathname;
		const langs = Object.keys(languages) as Lang[];
		for (const l of langs) {
			if (l !== "en" && (path.startsWith(`/${l}/`) || path === `/${l}`)) {
				setLang(l);
				return;
			}
		}
		setLang("en");
	}, []);

	const t = (key: string): string => {
		return translationsCommon[lang]?.[key] ?? translationsCommon.en[key] ?? key;
	};

	return { t, lang };
}

function getLocalizedPath(currentLang: Lang, targetLang: Lang) {
	if (typeof window === "undefined") return "/";
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
}

interface DropdownItem {
	label: string;
	href: string;
	icon?: React.ComponentType<{ className?: string }>;
	description?: string;
	external?: boolean;
	highlight?: boolean;
	divider?: boolean;
}

function NavDropdown({
	label,
	items,
}: {
	label: string;
	items: DropdownItem[];
}) {
	const [open, setOpen] = useState(false);
	const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	const handleMouseEnter = () => {
		if (timeoutRef.current) clearTimeout(timeoutRef.current);
		setOpen(true);
	};

	const handleMouseLeave = () => {
		timeoutRef.current = setTimeout(() => setOpen(false), 50);
	};

	return (
		<div
			className="relative"
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<button
				type="button"
				className="flex items-center gap-1 text-sm font-medium text-foreground/70 hover:text-foreground transition-colors duration-300 px-3 py-2"
				onClick={() => setOpen(!open)}
			>
				{label}
				<ChevronDown
					className={`w-3.5 h-3.5 transition-transform duration-300 ${open ? "rotate-180" : ""}`}
				/>
			</button>

			{open && (
				<div className="absolute top-full left-0 pt-2 z-50">
					<div className="bg-background/95 backdrop-blur-lg border border-border/50 rounded-xl shadow-xl shadow-black/10 p-2 min-w-[240px]">
						{items.map((item) => (
							<div key={item.href}>
								{item.divider && <div className="my-1.5 mx-2 border-t border-border/30" />}
								<a
								href={item.href}
								target={item.external ? "_blank" : undefined}
								rel={item.external ? "noreferrer" : undefined}
								className={`flex items-start gap-3 px-3 py-2.5 rounded-lg transition-colors duration-300 ${
									item.highlight
										? "text-primary hover:bg-primary/10"
										: "text-foreground/80 hover:bg-muted/50"
								}`}
								onClick={() => setOpen(false)}
							>
								{item.icon && <item.icon className="w-4 h-4 mt-0.5 shrink-0" />}
								<div className="flex-1 min-w-0">
									<div className="flex items-center gap-1.5">
										<span className="font-medium text-sm">{item.label}</span>
										{item.external && (
											<LuExternalLink className="w-3 h-3 opacity-50" />
										)}
									</div>
									{item.description && (
										<p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
											{item.description}
										</p>
									)}
								</div>
							</a>
							</div>
						))}
					</div>
				</div>
			)}
		</div>
	);
}

function LanguageSelector({ currentLang }: { currentLang: Lang }) {
	const [open, setOpen] = useState(false);
	const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	const handleMouseEnter = () => {
		if (timeoutRef.current) clearTimeout(timeoutRef.current);
		setOpen(true);
	};

	const handleMouseLeave = () => {
		timeoutRef.current = setTimeout(() => setOpen(false), 50);
	};

	return (
		<div
			className="relative"
			onMouseEnter={handleMouseEnter}
			onMouseLeave={handleMouseLeave}
		>
			<button
				type="button"
				onClick={() => setOpen(!open)}
				className="flex items-center gap-1.5 px-2 py-1.5 rounded-lg text-sm text-foreground/70 hover:text-foreground hover:bg-muted/50 transition-all duration-300"
				aria-label="Select language"
			>
				<span className="text-base leading-none">{langFlags[currentLang]}</span>
				<span className="uppercase text-xs font-medium">{currentLang}</span>
				<ChevronDown
					className={`w-3 h-3 transition-transform duration-300 ${open ? "rotate-180" : ""}`}
				/>
			</button>

			{open && (
				<div className="absolute top-full right-0 pt-2 z-50">
					<div className="bg-background/95 backdrop-blur-lg border border-border/50 rounded-xl shadow-xl shadow-black/10 p-2 min-w-[180px] max-h-[320px] overflow-y-auto">
						{(Object.entries(languages) as [Lang, string][]).map(
							([code, name]) => (
								<a
									key={code}
									href={getLocalizedPath(currentLang, code)}
									className={`flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-all duration-300 ${
										code === currentLang
											? "bg-primary/10 text-primary font-medium"
											: "text-foreground/70 hover:bg-muted/50 hover:text-foreground"
									}`}
									onClick={() => setOpen(false)}
								>
									<span className="text-lg">{langFlags[code]}</span>
									<span>{name}</span>
								</a>
							),
						)}
					</div>
				</div>
			)}
		</div>
	);
}

function MobileMenu({
	open,
	onClose,
	t,
	currentLang,
}: {
	open: boolean;
	onClose: () => void;
	t: (key: string) => string;
	currentLang: Lang;
}) {
	const [mounted, setMounted] = useState(false);
	const [langOpen, setLangOpen] = useState(false);

	useEffect(() => setMounted(true), []);

	useEffect(() => {
		if (open) {
			document.body.style.overflow = "hidden";
			return () => {
				document.body.style.overflow = "";
			};
		}
	}, [open]);

	if (!mounted) return null;

	return createPortal(
		<div
			className={`fixed inset-0 z-100 lg:hidden transition-opacity duration-300 ${
				open
					? "opacity-100 pointer-events-auto"
					: "opacity-0 pointer-events-none"
			}`}
			role="dialog"
			aria-modal="true"
		>
			<button
				type="button"
				aria-label="Close menu"
				onClick={onClose}
				className="absolute inset-0 w-full h-full bg-black/40 backdrop-blur-sm"
			/>
			<div
				className={`absolute top-0 right-0 w-full max-w-sm h-full bg-background/95 backdrop-blur-lg border-l border-border/50 shadow-2xl transition-transform duration-300 ease-out overflow-y-auto ${
					open ? "translate-x-0" : "translate-x-full"
				}`}
			>
				<div className="flex items-center justify-between p-4 border-b border-border/30 sticky top-0 bg-background/95 backdrop-blur-lg z-10">
					<a href="/" className="flex items-center gap-2" onClick={onClose}>
						<img alt="logo" src="/icon.webp" className="h-8 w-8" />
						<span className="font-semibold text-lg">Flow Like</span>
					</a>
					<button
						type="button"
						onClick={onClose}
						className="p-2 rounded-lg hover:bg-muted/50 transition-colors duration-300"
						aria-label="Close menu"
					>
						<X className="w-5 h-5" />
					</button>
				</div>

				<nav className="p-4 space-y-6">
					{/* Product Section */}
					<div>
						<p className="text-xs text-muted-foreground uppercase tracking-wider mb-3 px-3 font-medium">
							Product
						</p>
						<div className="space-y-1">
							<MobileNavItem
								href="/24-hour-solution"
								icon={LuZap}
								label={t("header.24h")}
								highlight
								onClick={onClose}
							/>
							<MobileNavItem
								href="/modern-bi"
								icon={ChartBar}
								label="Business Intelligence"
								onClick={onClose}
							/>
							<MobileNavItem
								href="/compare"
								icon={LuScale}
								label={t("header.compare")}
								onClick={onClose}
							/>
							<MobileNavItem
								href="/developers"
								icon={LuBookOpen}
								label="For Developers"
								onClick={onClose}
							/>
							<MobileNavItem
								href="/pitch"
								icon={LuBuilding2}
								label="For CIOs"
								onClick={onClose}
							/>
						<div className="my-1 mx-2 border-t border-border/20" />
							<MobileNavItem href="/industries/shopfloor" icon={LuFactory} label="Shopfloor" onClick={onClose} />
							<MobileNavItem href="/industries/finance" icon={LuLandmark} label="Finance" onClick={onClose} />
							<MobileNavItem href="/industries/office" icon={LuFileStack} label="Office" onClick={onClose} />
							<MobileNavItem href="/industries/ai-agents" icon={LuBot} label="AI Agents" onClick={onClose} />
							<MobileNavItem href="/industries/gov-defense" icon={LuShieldCheck} label="Gov & Defense" onClick={onClose} />
						</div>
					</div>

					{/* Resources Section */}
					<div>
						<p className="text-xs text-muted-foreground uppercase tracking-wider mb-3 px-3 font-medium">
							Resources
						</p>
						<div className="space-y-1">
							<MobileNavItem
								href="https://docs.flow-like.com"
								icon={LuBookMarked}
								label={t("header.docs")}
								external
								onClick={onClose}
							/>
							<MobileNavItem
								href="https://docs.flow-like.com/start/getting-started"
								icon={LuBookOpen}
								label="Getting Started"
								external
								onClick={onClose}
							/>
							<MobileNavItem
								href="https://docs.flow-like.com/self-hosting"
								icon={LuServer}
								label="Self-Hosting"
								external
								onClick={onClose}
							/>
							<MobileNavItem
								href="/blog/"
								icon={LuFileText}
								label={t("header.blog")}
								onClick={onClose}
							/>
						</div>
					</div>

					{/* Community Section */}
					<div>
						<p className="text-xs text-muted-foreground uppercase tracking-wider mb-3 px-3 font-medium">
							Community
						</p>
						<div className="flex gap-2 px-3">
							<a
								href="https://github.com/TM9657/flow-like"
								target="_blank"
								rel="noreferrer"
								className="flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg bg-muted/50 hover:bg-muted transition-colors duration-300"
							>
								<BsGithub className="w-4 h-4" />
								<span className="text-sm">GitHub</span>
							</a>
							<a
								href="https://discord.com/invite/KTWMrS2/"
								target="_blank"
								rel="noreferrer"
								className="flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg bg-muted/50 hover:bg-muted transition-colors duration-300"
							>
								<BsDiscord className="w-4 h-4" />
								<span className="text-sm">Discord</span>
							</a>
						</div>
					</div>

					{/* Language Section */}
					<div>
						<p className="text-xs text-muted-foreground uppercase tracking-wider mb-3 px-3 font-medium">
							Language
						</p>
						<button
							type="button"
							onClick={() => setLangOpen(!langOpen)}
							className="w-full flex items-center justify-between px-3 py-3 rounded-lg hover:bg-muted/50 transition-colors duration-300"
						>
							<div className="flex items-center gap-3">
								<LuGlobe className="w-5 h-5" />
								<span className="font-medium">
									{langFlags[currentLang]} {languages[currentLang]}
								</span>
							</div>
							<ChevronDown
								className={`w-4 h-4 transition-transform duration-300 ${langOpen ? "rotate-180" : ""}`}
							/>
						</button>
						{langOpen && (
							<div className="mt-2 grid grid-cols-2 gap-1 px-3">
								{(Object.entries(languages) as [Lang, string][]).map(
									([code, name]) => (
										<a
											key={code}
											href={getLocalizedPath(currentLang, code)}
											className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-all duration-300 ${
												code === currentLang
													? "bg-primary/10 text-primary font-medium"
													: "text-foreground/70 hover:bg-muted/50"
											}`}
											onClick={onClose}
										>
											<span className="text-lg">{langFlags[code]}</span>
											<span className="truncate">{name}</span>
										</a>
									),
								)}
							</div>
						)}
					</div>
				</nav>

				<div className="sticky bottom-0 p-4 border-t border-border/30 bg-background/95 backdrop-blur-lg">
					<a
						href={webAppUrl}
						target="_blank"
						rel="noreferrer"
						onClick={onClose}
						className="w-full mb-2 group flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg border border-border/70 bg-background text-foreground font-medium hover:bg-muted/40 transition-colors duration-300"
					>
						<LuExternalLink className="w-4 h-4" />
						Open Web App
					</a>
					<a
						href="/download"
						onClick={onClose}
						className="w-full group flex items-center justify-center gap-2 py-2.5 px-4 rounded-lg bg-primary text-primary-foreground font-medium hover:bg-primary/90 transition-colors duration-300"
					>
						<LuDownload className="w-4 h-4" />
						{t("header.download")} Studio
						<LuArrowRight className="w-4 h-4 ml-auto transition-transform duration-300 group-hover:translate-x-1" />
					</a>
				</div>
			</div>
		</div>,
		document.body,
	);
}

function MobileNavItem({
	href,
	icon: Icon,
	label,
	highlight,
	external,
	onClick,
}: {
	href: string;
	icon: React.ComponentType<{ className?: string }>;
	label: string;
	highlight?: boolean;
	external?: boolean;
	onClick: () => void;
}) {
	return (
		<a
			href={href}
			target={external ? "_blank" : undefined}
			rel={external ? "noreferrer" : undefined}
			onClick={onClick}
			className={`flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all ${
				highlight
					? "text-primary bg-primary/5 hover:bg-primary/10"
					: "text-foreground/80 hover:bg-muted/50"
			}`}
		>
			<Icon className="w-5 h-5" />
			<span className="font-medium">{label}</span>
			{external && (
				<LuExternalLink className="w-3.5 h-3.5 ml-auto opacity-40" />
			)}
		</a>
	);
}

export function Header() {
	const { t, lang } = useTranslation();
	const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
	const [scrolled, setScrolled] = useState(false);

	useEffect(() => {
		const handleScroll = () => setScrolled(window.scrollY > 20);
		window.addEventListener("scroll", handleScroll, { passive: true });
		return () => window.removeEventListener("scroll", handleScroll);
	}, []);

	const productItems: DropdownItem[] = [
		{
			label: t("header.24h"),
			href: "/24-hour-solution",
			icon: LuZap,
			description: "Get a custom solution in 24 hours",
			highlight: true,
		},
		{
			label: "Business Intelligence",
			href: "/modern-bi",
			icon: ChartBar,
			description: "AI-powered data pipelines",
		},
		{
			label: t("header.compare"),
			href: "/compare",
			icon: LuScale,
			description: "See how we stack up",
		},
		{
			label: "For Developers",
			href: "/developers",
			icon: LuBookOpen,
			description: "Build workflows or write custom nodes",
		},
		{
			label: "For CIOs",
			href: "/pitch",
			icon: LuBuilding2,
			description: "Executive overview & ROI",
		},
		{
			label: "Shopfloor",
			href: "/industries/shopfloor",
			icon: LuFactory,
			description: "Machines, sensors & PLCs",
			divider: true,
		},
		{
			label: "Finance",
			href: "/industries/finance",
			icon: LuLandmark,
			description: "Reconciliation & compliance",
		},
		{
			label: "Office",
			href: "/industries/office",
			icon: LuFileStack,
			description: "Documents & approvals",
		},
		{
			label: "AI Agents",
			href: "/industries/ai-agents",
			icon: LuBot,
			description: "LLMs, RAG & multi-agent",
		},
		{
			label: "Gov & Defense",
			href: "/industries/gov-defense",
			icon: LuShieldCheck,
			description: "Air-gapped & sovereign",
		},
	];

	const resourceItems: DropdownItem[] = [
		{
			label: t("header.docs"),
			href: "https://docs.flow-like.com",
			icon: LuBookMarked,
			external: true,
		},
		{
			label: "Getting Started",
			href: "https://docs.flow-like.com/start/getting-started",
			icon: LuBookOpen,
			external: true,
		},
		{
			label: "Self-Hosting",
			href: "https://docs.flow-like.com/self-hosting",
			icon: LuServer,
			external: true,
		},
		{
			label: t("header.blog"),
			href: "/blog/",
			icon: LuFileText,
		},
	];

	return (
		<>
			<header
				className={`w-full fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
					scrolled
						? "h-14 bg-background/80 backdrop-blur-lg border-b border-border/50 shadow-sm"
						: "h-16 bg-transparent"
				}`}
			>
				<div className="max-w-7xl mx-auto h-full px-4 flex items-center justify-between">
					{/* Logo */}
					<a href="/" className="flex items-center gap-2.5 group shrink-0">
						<img
							alt="Flow Like logo"
							src="/icon.webp"
							className={`transition-all duration-300 ${scrolled ? "h-8 w-8" : "h-10 w-10"}`}
						/>
						<span className="font-semibold text-lg tracking-tight group-hover:text-primary transition-colors duration-300">
							Flow Like
						</span>
					</a>

					{/* Desktop Navigation */}
					<nav className="hidden lg:flex items-center gap-1">
						<NavDropdown label="Product" items={productItems} />
						<NavDropdown label="Resources" items={resourceItems} />
						<a
							href="/pricing"
							className="px-3 py-2 text-sm font-medium text-foreground/70 hover:text-foreground transition-colors duration-300"
						>
							Pricing
						</a>
					</nav>

					{/* Desktop Actions */}
					<div className="hidden lg:flex items-center gap-2">
						<LanguageSelector currentLang={lang} />

						<div className="flex items-center border-l border-border/40 pl-3 ml-1">
							<a
								href="https://github.com/TM9657/flow-like"
								target="_blank"
								rel="noreferrer"
								aria-label="GitHub"
								className="p-2 rounded-lg text-foreground/60 hover:text-foreground hover:bg-muted/50 transition-all duration-300"
							>
								<BsGithub className="w-4 h-4" />
							</a>
							<a
								href="https://discord.com/invite/KTWMrS2/"
								target="_blank"
								rel="noreferrer"
								aria-label="Discord"
								className="p-2 rounded-lg text-foreground/60 hover:text-foreground hover:bg-muted/50 transition-all duration-300"
							>
								<BsDiscord className="w-4 h-4" />
							</a>
						</div>

						<a
							href={webAppUrl}
							target="_blank"
							rel="noreferrer"
							className="ml-2 flex items-center gap-2 py-1.5 px-3 rounded-lg border border-border/70 bg-background text-foreground text-sm font-medium hover:bg-muted/50 transition-colors duration-300"
						>
							<LuExternalLink className="w-4 h-4" />
							Open Web App
						</a>

						<a
							href="/download"
							className="flex items-center gap-2 py-1.5 px-3 rounded-lg bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-colors duration-300"
							title={studioName}
						>
							<LuDownload className="w-4 h-4" />
							{t("header.download")} Studio
						</a>
					</div>

					{/* Mobile Menu Button */}
					<button
						type="button"
						onClick={() => setMobileMenuOpen(true)}
						className="lg:hidden p-2 rounded-lg hover:bg-muted/50 transition-colors duration-300"
						aria-label="Open menu"
					>
						<Menu className="w-5 h-5" />
					</button>
				</div>
			</header>

			<MobileMenu
				open={mobileMenuOpen}
				onClose={() => setMobileMenuOpen(false)}
				t={t}
				currentLang={lang}
			/>
		</>
	);
}
