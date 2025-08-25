import { useState } from "react";
import { Button } from "@tm9657/flow-like-ui";
import { BsDiscord, BsGithub, BsTwitterX } from "react-icons/bs";
import { LuBookHeart, LuBookMarked, LuDownload } from "react-icons/lu";
import { X } from "lucide-react";

export function BlogHeader() {
	// Mobile menu open state
	const [open, setOpen] = useState(false);

	// Small helper to render nav links with staggered animation on mobile
	const MobileLink = ({ href, onClick, children, icon, external }: {
		href: string;
		onClick?: () => void;
		children: React.ReactNode;
		icon?: React.ReactNode;
		external?: boolean;
	}) => {
		// index-based stagger is handled by inline style below where used
		return (
			<a
				href={href}
				target={external ? "_blank" : undefined}
				rel={external ? "noreferrer" : undefined}
				onClick={onClick}
				className={
					"flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform"
				}
			>
				{icon}
				<span>{children}</span>
			</a>
		);
	};

	// Animated hamburger that morphs into an X using 3 bars
	const Hamburger: React.FC<{ open: boolean; className?: string; onClick?: () => void }> = ({ open, className, onClick }) => {
		return (
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
	};

	return (
		<header className="w-full flex flex-row items-center sticky top-0 left-0 right-0 min-h-16 h-16 z-20 backdrop-blur-sm shadow-md bg-background/40 justify-between px-2">
			<a href="/" className="flex flex-row items-center gap-2">
				<img alt="logo" src="/icon.webp" className="h-12 w-12" />
				<h3 className="hidden sm:block">Flow Like</h3>
			</a>

			{/* Desktop / tablet nav (hidden on small screens) */}
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

			{/* Mobile controls (visible on small screens only) */}
			<div className="flex items-center gap-2 sm:hidden">
				{/* Blog CTA: always visible on mobile and prominent */}
				<a
					href="/blog/"
					className="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-primary text-primary-foreground font-medium"
				>
					<LuBookHeart className="w-5 h-5" />
					<span>Blog</span>
				</a>

				{/* Animated Hamburger */}
				<Hamburger open={open} onClick={() => setOpen((s) => !s)} />
			</div>

			{/* Mobile menu overlay — always rendered so we can animate open/close */}
			<div
				className={`fixed inset-0 z-30 sm:hidden pointer-events-none transition-opacity duration-300 ${
					open ? "opacity-100 pointer-events-auto" : "opacity-0"
				}`}
				role="dialog"
				aria-modal={open}
				aria-hidden={!open}
			>
				{/* Panel: slides down/up */}
				<div
					className={`transform transition-transform duration-300 ${
						open ? "translate-y-0" : "-translate-y-4"
					}`}
				>
					<div className="flex items-center justify-between p-4 bg-background">
						<a href="/" className="flex items-center gap-2">
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

					<nav className="px-6 py-4 space-y-4 bg-background">
						{/* each item has a small stagger so it feels less clunky */}
						<a
							href="/blog/"
							onClick={() => setOpen(false)}
							className={`flex items-center gap-3 px-4 py-3 rounded-md bg-primary text-primary-foreground font-medium transition transform ${
								open ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-2"
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
							onClick={() => setOpen(false)}
							className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
								open ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-2"
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
							onClick={() => setOpen(false)}
							className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
								open ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-2"
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
							onClick={() => setOpen(false)}
							className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
								open ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-2"
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
							onClick={() => setOpen(false)}
							className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
								open ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-2"
							}`}
							style={{ transitionDelay: open ? "260ms" : "0ms" }}
						>
							<BsDiscord className="w-5 h-5" />
							<span>Discord</span>
						</a>

						<a
							href="/download"
							onClick={() => setOpen(false)}
							className={`flex items-center gap-3 px-4 py-3 rounded-md hover:bg-background/50 transition transform ${
								open ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-2"
							}`}
							style={{ transitionDelay: open ? "310ms" : "0ms" }}
						>
							<LuDownload className="w-5 h-5" />
							<span>Download</span>
						</a>
					</nav>
				</div>
			</div>
		</header>
	);
}
