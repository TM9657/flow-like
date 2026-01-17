"use client";

import {
	Brain,
	Code,
	Coffee,
	Cpu,
	Globe,
	Heart,
	Hexagon,
	Lightbulb,
	Loader2,
	Rocket,
	Shield,
	Sparkles,
	Star,
	Wand2,
	Zap,
} from "lucide-react";
import { useEffect, useState } from "react";
import { cn } from "../../lib";

interface LoadingScreenProps {
	message?: string;
	progress?: number;
	className?: string;
}

const loadingMessages = [
	"Preparing something amazing...",
	"Initializing awesome...",
	"Crafting the perfect experience...",
	"Assembling digital magic...",
	"Loading brilliance...",
	"Weaving code into reality...",
	"Summoning the flow state...",
	"Materializing innovation...",
	"Brewing digital excellence...",
	"Orchestrating perfection...",
];

const floatingIcons = [
	{ Icon: Zap, delay: 0, color: "text-yellow-400", size: "w-6 h-6" },
	{ Icon: Sparkles, delay: 0.5, color: "text-purple-400", size: "w-5 h-5" },
	{ Icon: Code, delay: 1, color: "text-blue-400", size: "w-7 h-7" },
	{ Icon: Rocket, delay: 1.5, color: "text-red-400", size: "w-6 h-6" },
	{ Icon: Brain, delay: 2, color: "text-green-400", size: "w-5 h-5" },
	{ Icon: Lightbulb, delay: 2.5, color: "text-orange-400", size: "w-6 h-6" },
	{ Icon: Star, delay: 3, color: "text-pink-400", size: "w-5 h-5" },
	{ Icon: Heart, delay: 3.5, color: "text-rose-400", size: "w-6 h-6" },
];

const backgroundIcons = [
	{ Icon: Hexagon, color: "text-muted-foreground/20", size: "w-12 h-12" },
	{ Icon: Coffee, color: "text-muted-foreground/15", size: "w-8 h-8" },
	{ Icon: Cpu, color: "text-muted-foreground/20", size: "w-10 h-10" },
	{ Icon: Globe, color: "text-muted-foreground/15", size: "w-14 h-14" },
	{ Icon: Shield, color: "text-muted-foreground/20", size: "w-9 h-9" },
	{ Icon: Wand2, color: "text-muted-foreground/15", size: "w-11 h-11" },
];

// Seeded random number generator for deterministic values
function seededRandom(seed: number) {
	const x = Math.sin(seed) * 10000;
	return x - Math.floor(x);
}

export function LoadingScreen({
	message = loadingMessages[0],
	progress = 0,
	className,
}: Readonly<LoadingScreenProps>) {
	const [currentMessage, setCurrentMessage] = useState(message);
	const [dots, setDots] = useState("");
	const [isMounted, setIsMounted] = useState(false);
	const [backgroundElements, setBackgroundElements] = useState<
		Array<{
			id: number;
			x: number;
			y: number;
			icon: (typeof backgroundIcons)[0];
			animationDelay: number;
			animationDuration: number;
		}>
	>([]);

	// Generate deterministic particle positions using seeded random
	const particles = Array.from({ length: 30 }, (_, i) => ({
		id: i,
		x: seededRandom(i * 7 + 1) * 100,
		y: seededRandom(i * 13 + 2) * 100,
		delay: seededRandom(i * 17 + 3) * 8,
		duration: 3 + seededRandom(i * 23 + 4) * 2,
	}));

	useEffect(() => {
		setIsMounted(true);

		// Generate random background elements (client-side only)
		const elements = Array.from({ length: 5 }, (_, i) => ({
			id: i,
			x: seededRandom(i * 31 + 5) * 100,
			y: seededRandom(i * 37 + 6) * 100,
			icon: backgroundIcons[
				Math.floor(seededRandom(i * 41 + 7) * backgroundIcons.length)
			],
			animationDelay: seededRandom(i * 43 + 8) * 5,
			animationDuration: 8 + seededRandom(i * 47 + 9) * 4,
		}));
		setBackgroundElements(elements);

		const messageInterval = setInterval(() => {
			setCurrentMessage(
				loadingMessages[Math.floor(Math.random() * loadingMessages.length)],
			);
		}, 3000);

		const dotsInterval = setInterval(() => {
			setDots((prev) => (prev.length >= 3 ? "" : prev + "."));
		}, 500);

		return () => {
			clearInterval(messageInterval);
			clearInterval(dotsInterval);
		};
	}, []);

	return (
		<div
			className={cn(
				"fixed inset-0 bg-background flex items-center justify-center overflow-hidden",
				className,
			)}
		>
			{/* Dynamic background gradient */}
			<div className="absolute inset-0 bg-linear-to-br from-background via-muted/30 to-background animate-gradient-shift" />

			{/* Animated mesh overlay */}
			<div className="absolute inset-0 opacity-40">
				<div className="absolute inset-0 bg-[radial-gradient(circle_at_25%_25%,var(--muted)_1px,transparent_1px)] bg-size-[50px_50px] animate-mesh-drift" />
				<div className="absolute inset-0 bg-[radial-gradient(circle_at_75%_75%,var(--border)_1px,transparent_1px)] bg-size-[80px_80px] animate-mesh-drift-reverse" />
			</div>

			{/* Floating grid overlay */}
			<div className="absolute inset-0 bg-[linear-gradient(to_right,var(--border)_1px,transparent_1px),linear-gradient(to_bottom,var(--border)_1px,transparent_1px)] bg-size-[120px_120px] opacity-20 animate-grid-float" />

			{/* Large background geometric shapes */}
			<div className="absolute inset-0 overflow-hidden">
				<div className="absolute -top-40 -left-40 w-80 h-80 border border-muted-foreground/10 rounded-full animate-float-massive" />
				<div className="absolute -bottom-32 -right-32 w-64 h-64 border border-muted-foreground/10 rotate-45 animate-float-massive-reverse" />
				<div className="absolute top-1/4 -left-20 w-40 h-40 border border-muted-foreground/10 rounded-full animate-pulse-gentle" />
			</div>

			{/* Background icon constellation */}
			{isMounted &&
				backgroundElements.map((element) => (
					<div
						key={element.id}
						className="absolute animate-float-background"
						style={{
							left: `${element.x}%`,
							top: `${element.y}%`,
							animationDelay: `${element.animationDelay}s`,
							animationDuration: `${element.animationDuration}s`,
						}}
					>
						<element.icon.Icon
							className={cn(
								element.icon.size,
								element.icon.color,
								"animate-spin-very-slow",
							)}
						/>
					</div>
				))}

			{/* Floating colorful icons */}
			{floatingIcons.map(({ Icon, delay, color, size }, i) => (
				<div
					key={`floating-icon-${delay}-${color}-${size}`}
					className="absolute animate-float-orbital opacity-70 hover:opacity-100 transition-opacity duration-300"
					style={{
						left: `${15 + i * 10}%`,
						top: `${25 + Math.sin(i * 1.2) * 30}%`,
						animationDelay: `${delay}s`,
						animationDuration: "10s",
					}}
				>
					<div className="relative">
						{/* Glow effect */}
						<div className={cn("absolute inset-0 blur-md opacity-60", color)}>
							<Icon className={size} />
						</div>
						<Icon className={cn(size, color, "relative z-10 drop-shadow-lg")} />
					</div>
				</div>
			))}

			{/* Particle trail effect */}
			<div className="absolute inset-0">
				{particles.map((particle) => (
					<div
						key={particle.id}
						className="absolute w-1 h-1 bg-muted-foreground/30 rounded-full animate-particle-trail"
						style={{
							left: `${particle.x}%`,
							top: `${particle.y}%`,
							animationDelay: `${particle.delay}s`,
							animationDuration: `${particle.duration}s`,
						}}
					/>
				))}
			</div>

			{/* Main loading content */}
			<div className="relative z-10 text-center space-y-2 px-8 max-w-lg">
				{/* Elegant central loader */}
				<div className="relative flex flex-col items-center justify-center">
					<div className="relative">
						{/* Subtle outer ring */}
						<div className="absolute inset-0 rounded-full border border-border/50 w-32 h-32 left-1/2 top-1/2 -ml-16 -mt-16 animate-spin-elegant" />

						{/* Inner container */}
						<div className="relative bg-card/95 backdrop-blur-xs rounded-full p-8 border border-border shadow-lg w-24 h-24">
							<Loader2 className="w-8 h-8 animate-spin text-foreground mx-auto" />
						</div>

						{/* Minimal orbiting dots */}
						<div className="absolute inset-0 animate-spin-slow">
							{Array.from({ length: 6 }).map((_, i) => (
								<div
									key={i}
									className="absolute w-1.5 h-1.5 bg-muted-foreground rounded-full"
									style={{
										top: "50%",
										left: "50%",
										transform: `rotate(${i * 60}deg) translateX(40px) translateY(-50%)`,
									}}
								/>
							))}
						</div>
					</div>
				</div>

				{/* Elegant loading message */}
				<div className="space-y-4 pt-6">
					<h2 className="text-xl font-semibold text-foreground">
						{currentMessage}
						{dots}
					</h2>

					{/* Clean progress bar */}
					{progress > 0 && (
						<div className="w-80 mx-auto space-y-2">
							<div className="relative w-full bg-muted rounded-full h-1.5 overflow-hidden">
								<div
									className="h-full bg-foreground rounded-full transition-all duration-700 ease-out"
									style={{ width: `${Math.min(progress, 100)}%` }}
								/>
							</div>
							<p className="text-xs text-muted-foreground">
								{Math.round(progress)}% complete
							</p>
						</div>
					)}
				</div>

				{/* Simple subtitle */}
				<p className="text-muted-foreground">
					Please wait while we prepare everything
				</p>

				{/* Minimal loading indicators */}
				<div className="flex justify-center space-x-1 pt-4">
					{Array.from({ length: 3 }).map((_, i) => (
						<div
							key={i}
							className="w-1.5 h-1.5 bg-muted-foreground rounded-full animate-bounce-jump"
							style={{ animationDelay: `${i * 0.15}s` }}
						/>
					))}
				</div>
			</div>

			<style jsx>{`
                @keyframes bounce-jump {
                    0%, 20%, 50%, 80%, 100% {
                        transform: translateY(0);
                        opacity: 0.6;
                    }
                    40% {
                        transform: translateY(-12px);
                        opacity: 1;
                    }
                    60% {
                        transform: translateY(-6px);
                        opacity: 0.8;
                    }
                }

                @keyframes float-orbital {
                    0%, 100% { transform: translateY(0px) translateX(0px) rotate(0deg); }
                    25% { transform: translateY(-20px) translateX(15px) rotate(90deg); }
                    50% { transform: translateY(-40px) translateX(0px) rotate(180deg); }
                    75% { transform: translateY(-20px) translateX(-15px) rotate(270deg); }
                }

                @keyframes float-background {
                    0%, 100% { transform: translateY(0px) scale(1); opacity: 0.3; }
                    50% { transform: translateY(-30px) scale(1.1); opacity: 0.5; }
                }

                @keyframes float-massive {
                    0%, 100% { transform: translateY(0px) translateX(0px) rotate(0deg); }
                    50% { transform: translateY(-60px) translateX(30px) rotate(180deg); }
                }

                @keyframes float-massive-reverse {
                    0%, 100% { transform: translateY(0px) translateX(0px) rotate(0deg); }
                    50% { transform: translateY(60px) translateX(-30px) rotate(-180deg); }
                }

                @keyframes particle-trail {
                    0% { transform: translateY(0px) scale(1); opacity: 0; }
                    50% { transform: translateY(-100px) scale(1.5); opacity: 1; }
                    100% { transform: translateY(-200px) scale(0.5); opacity: 0; }
                }

                @keyframes gradient-shift {
                    0%, 100% { background-position: 0% 50%; }
                    50% { background-position: 100% 50%; }
                }

                @keyframes mesh-drift {
                    0%, 100% { transform: translate(0, 0); }
                    50% { transform: translate(25px, -25px); }
                }

                @keyframes mesh-drift-reverse {
                    0%, 100% { transform: translate(0, 0); }
                    50% { transform: translate(-20px, 20px); }
                }

                @keyframes grid-float {
                    0%, 100% { transform: translate(0, 0); }
                    50% { transform: translate(15px, -15px); }
                }

                @keyframes spin-elegant {
                    from { transform: rotate(0deg); }
                    to { transform: rotate(360deg); }
                }

                @keyframes spin-slow {
                    from { transform: rotate(0deg); }
                    to { transform: rotate(360deg); }
                }

                @keyframes spin-reverse {
                    from { transform: rotate(360deg); }
                    to { transform: rotate(0deg); }
                }

                @keyframes spin-very-slow {
                    from { transform: rotate(0deg); }
                    to { transform: rotate(360deg); }
                }

                @keyframes pulse-gentle {
                    0%, 100% { opacity: 0.3; transform: scale(1); }
                    50% { opacity: 0.6; transform: scale(1.05); }
                }

                @keyframes pulse-wave {
                    0%, 20%, 100% { opacity: 0.3; transform: scale(1); }
                    50% { opacity: 1; transform: scale(1.2); }
                }

                @keyframes shimmer {
                    0% { transform: translateX(-100%); }
                    100% { transform: translateX(100%); }
                }

                .animate-bounce-jump {
                    animation: bounce-jump 1.2s ease-in-out infinite;
                }

                .animate-float-orbital {
                    animation: float-orbital 8s ease-in-out infinite;
                }

                .animate-float-background {
                    animation: float-background 6s ease-in-out infinite;
                }

                .animate-float-massive {
                    animation: float-massive 12s ease-in-out infinite;
                }

                .animate-float-massive-reverse {
                    animation: float-massive-reverse 15s ease-in-out infinite;
                }

                .animate-particle-trail {
                    animation: particle-trail 4s ease-out infinite;
                }

                .animate-gradient-shift {
                    animation: gradient-shift 8s ease infinite;
                }

                .animate-mesh-drift {
                    animation: mesh-drift 10s ease-in-out infinite;
                }

                .animate-mesh-drift-reverse {
                    animation: mesh-drift-reverse 12s ease-in-out infinite;
                }

                .animate-grid-float {
                    animation: grid-float 8s ease-in-out infinite;
                }

                .animate-spin-elegant {
                    animation: spin-elegant 15s linear infinite;
                }

                .animate-spin-slow {
                    animation: spin-slow 10s linear infinite;
                }

                .animate-spin-reverse {
                    animation: spin-reverse 8s linear infinite;
                }

                .animate-spin-very-slow {
                    animation: spin-very-slow 20s linear infinite;
                }

                .animate-pulse-gentle {
                    animation: pulse-gentle 4s ease-in-out infinite;
                }

                .animate-pulse-wave {
                    animation: pulse-wave 1.8s ease-in-out infinite;
                }

                .animate-shimmer {
                    animation: shimmer 2s ease-in-out infinite;
                }
            `}</style>
		</div>
	);
}
