import { Button } from "@tm9657/flow-like-ui";
import { X } from "lucide-react";
import type React from "react";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { BsDiscord, BsGithub, BsTwitterX } from "react-icons/bs";
import { LuBookHeart, LuBookMarked, LuDownload } from "react-icons/lu";

export function BlogHeader() {
	const [open, setOpen] = useState(false);
	const [mounted, setMounted] = useState(false);

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
			aria-label={open ? "Close menu" : "Open menu"}
			onClick={onClick}
			className={`relative w-8 h-8 inline-flex items-center justify-center ${className ?? ""}`}
		>
			<span
				className={`block absolute left-1/2 top-1/2 w-6 h-[2px] bg-foreground transition-transform duration-300 ease-in-out transform origin-center ${
					open ? "translate-y-0 rotate-45" : "-translate-y-2"
				}`}
				style={{ transformOrigin: "center" }}
			/>
			<span
				className={`block absolute left-1/2 top-1/2 w-6 h-[2px] bg-foreground transition-opacity duration-200 ease-in-out transform ${
					open ? "opacity-0 scale-90" : "opacity-100 scale-100"
				}`}
				style={{ transform: "translateX(-50%) translateY(-50%)" }}
			/>
			<span
				className={`block absolute left-1/2 top-1/2 w-6 h-[2px] bg-foreground transition-transform duration-300 ease-in-out transform origin-center ${
					open ? "translate-y-0 -rotate-45" : "translate-y-2"
				}`}
				style={{ transformOrigin: "center" }}
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
						className="absolute inset-0 w-full h-full bg-black/30 supports-[backdrop-filter]:backdrop-blur-sm"
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
								<span>Blog</span>
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
								<span>Docs</span>
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
								<span>Download</span>
							</a>
						</nav>
					</div>
				</div>,
				document.body,
			)
		: null;

	return (
		<>
			<header className="w-full flex flex-row items-center sticky top-0 left-0 right-0 min-h-16 h-16 z-20 backdrop-blur-sm shadow-md bg-background/40 justify-between px-2">
				<a href="/" className="flex flex-row items-center gap-2">
					<img alt="logo" src="/icon.webp" className="h-12 w-12" />
					<h3 className="hidden sm:block">Flow Like</h3>
				</a>

				{/* Desktop / tablet nav */}
				<div className="hidden sm:flex flex-row items-center gap-2">
					<a href="/blog/">
						<Button variant={"outline"}>
							<LuBookHeart className="w-5 h-5" />
							Blog
						</Button>
					</a>
					<a href="https://docs.flow-like.com" target="_blank" rel="noreferrer">
						<Button variant={"outline"}>
							<LuBookMarked className="w-5 h-5" />
							Docs
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
							Download
						</Button>
					</a>
				</div>

				{/* Mobile controls */}
				<div className="flex items-center gap-2 sm:hidden">
					<a
						href="/blog/"
						className="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-primary text-primary-foreground font-medium"
					>
						<LuBookHeart className="w-5 h-5" />
						<span>Blog</span>
					</a>
					<Hamburger open={open} onClick={() => setOpen((s) => !s)} />
				</div>
			</header>

			{/* Portal overlay */}
			{MobileOverlay}
		</>
	);
}
