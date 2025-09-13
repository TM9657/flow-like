import { GitHubLogoIcon } from "@radix-ui/react-icons";
import { Button } from "@tm9657/flow-like-ui";
import {
	Book,
	Code2,
	Heart,
	MessageCircle,
	Rocket,
	Sparkles,
	Star,
	X,
	Zap,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

type WelcomeStep = "welcome" | "docs" | "discord" | "github";

const STEPS: WelcomeStep[] = ["welcome", "docs", "discord", "github"];

interface FloatingIconProps {
	icon: React.ComponentType<{ className?: string }>;
	className?: string;
	delay?: number;
	duration?: number;
}

interface AnimatedBackgroundProps {
	children: React.ReactNode;
	variant?: WelcomeStep;
}

export function TutorialDialog() {
	const [showTutorial, setShowTutorial] = useState(false);
	const [currentStep, setCurrentStep] = useState<WelcomeStep>("welcome");
	const [supportsBackdrop, setSupportsBackdrop] = useState<boolean>(true);

	useEffect(() => {
		const hasFinishedTutorial = localStorage.getItem("tutorial-finished");
		setShowTutorial(hasFinishedTutorial !== "true");
	}, []);

	// Lock background scroll while tutorial is shown
	useEffect(() => {
		if (showTutorial) {
			const prev = document.body.style.overflow;
			document.body.style.overflow = "hidden";
			return () => {
				document.body.style.overflow = prev;
			};
		}
	}, [showTutorial]);

	// Detect backdrop-filter support and provide a Linux WebKit fallback.
	useEffect(() => {
		try {
			const ua = navigator.userAgent.toLowerCase();
			const isLinux = ua.includes("linux");
			const hasBackdrop =
				(CSS && (CSS as any).supports && (CSS as any).supports("backdrop-filter", "blur(4px)")) ||
				(CSS && (CSS as any).supports && (CSS as any).supports("-webkit-backdrop-filter", "blur(4px)"));
			// Some Linux WebKit builds lie about supports; prefer hard fallback on Linux + WebKit.
			const isWebKit = /applewebkit\//.test(ua) && !/chrome\//.test(ua) ? true : /webkit/.test(ua);
			const forceFallback = isLinux && isWebKit;
			setSupportsBackdrop(Boolean(hasBackdrop) && !forceFallback);
		} catch {
			setSupportsBackdrop(false);
		}
	}, []);

	const handleSkip = () => {
		localStorage.setItem("tutorial-finished", "true");
		setShowTutorial(false);
	};

	const handleNext = () => {
		const currentIndex = STEPS.indexOf(currentStep);
		if (currentIndex < STEPS.length - 1) {
			setCurrentStep(STEPS[currentIndex + 1]);
		} else {
			handleSkip();
		}
	};

	const handlePrevious = () => {
		const currentIndex = STEPS.indexOf(currentStep);
		if (currentIndex > 0) {
			setCurrentStep(STEPS[currentIndex - 1]);
		}
	};

	const getBackgroundGradient = (variant: WelcomeStep) => {
		switch (variant) {
			case "discord":
				return "bg-linear-to-br from-[#5865F2]/50 via-[#7289DA]/12 to-[#5865F2]/36";
			case "github":
				return "bg-linear-to-br from-foreground/20 via-foreground/10 to-foreground/8";
			case "docs":
				return "bg-linear-to-br from-primary/20 via-blue-500/10 to-primary/15";
			default:
				return "bg-linear-to-br from-primary/18 via-purple-500/8 to-secondary/12";
		}
	};

	const FloatingIcon = ({
		icon: Icon,
		className = "",
		delay = 0,
		duration = 4,
	}: FloatingIconProps) => (
		<div
			className={`absolute animate-bounce ${className}`}
			style={{
				animationDelay: `${delay}s`,
				animationDuration: `${duration}s`,
				animationIterationCount: "infinite",
			}}
		>
			<Icon className="w-4 h-4 text-muted-foreground/30" />
		</div>
	);

	const AnimatedBackground = ({
		children,
		variant = "welcome",
	}: AnimatedBackgroundProps) => (
		<div className="relative min-h-screen flex items-center justify-center p-3 sm:p-6">
			<div className={`absolute inset-0 ${getBackgroundGradient(variant)}`} />
			{/* Tone down pulses on small screens */}
			<div
				className="absolute inset-0 bg-linear-to-tr from-accent/10 via-transparent to-primary/10 animate-pulse opacity-40 sm:opacity-100"
				style={{ animationDuration: "8s" }}
			/>
			<div
				className="absolute inset-0 bg-linear-to-bl from-secondary/8 via-transparent to-accent/8 animate-pulse opacity-40 sm:opacity-100"
				style={{ animationDuration: "12s", animationDelay: "4s" }}
			/>

			{/* Hide most floating icons on very small screens to reduce clutter */}
			<div className="hidden sm:block">
				<FloatingIcon icon={Star} className="top-20 left-[10%]" delay={0} />
				<FloatingIcon icon={Sparkles} className="top-32 right-[15%]" delay={1.5} />
				<FloatingIcon icon={Code2} className="bottom-40 left-[8%]" delay={3} />
				<FloatingIcon icon={Heart} className="bottom-24 right-[20%]" delay={4.5} />
				<FloatingIcon icon={Rocket} className="top-48 left-[25%]" delay={2} />
				<FloatingIcon icon={Zap} className="bottom-52 right-[12%]" delay={3.5} />
				<FloatingIcon icon={Sparkles} className="top-64 right-[8%]" delay={1} />
				<FloatingIcon icon={Star} className="bottom-32 left-[18%]" delay={4} />
			</div>

			<div
				className="absolute top-[20%] left-[5%] w-32 h-32 bg-primary/8 rounded-full blur-2xl animate-pulse max-sm:opacity-40"
				style={{ animationDuration: "6s" }}
			/>
			<div
				className="absolute bottom-[25%] right-[8%] w-24 h-24 bg-secondary/8 rounded-full blur-2xl animate-pulse max-sm:opacity-40"
				style={{ animationDuration: "8s", animationDelay: "2s" }}
			/>

			<div className="relative z-10 w-full h-full flex items-stretch justify-center">
				{children}
			</div>
		</div>
	);

	const stepData = {
		welcome: {
			title: "Welcome to Flow Like",
			description:
				"Your comprehensive solution for modern software development",
		},
		docs: {
			title: "Documentation & Guides",
			description: "Everything you need to master Flow Like effectively",
		},
		discord: {
			title: "Join Our Community",
			description: "Connect with developers, get help, and share feedback",
		},
		github: {
			title: "Open Source Project",
			description: "Explore, contribute, and customize Flow Like",
		},
	};

	const FeatureItem = ({
		icon: Icon,
		text,
		color = "primary",
	}: {
		icon: React.ComponentType<{ className?: string }>;
		text: string;
		color?: string;
	}) => (
		<div
			className={
				supportsBackdrop
					? "flex items-center gap-3 p-3 rounded-xl bg-background/30 backdrop-blur-md border border-border/40 shadow-lg"
					: "flex items-center gap-3 p-3 rounded-xl bg-card border border-border/60 shadow-md"
			}
		>
			<Icon className={`w-5 h-5 text-${color}`} />
			<span className="text-sm font-medium">{text}</span>
		</div>
	);

	const BulletPoint = ({
		text,
		color = "primary",
	}: { text: string; color?: string }) => (
		<div className="flex items-center gap-2 text-sm">
			<div className={`w-2 h-2 bg-${color} rounded-full`} />
			<span>{text}</span>
		</div>
	);

	const WelcomeStep = () => (
		<div className="grid grid-cols-1 sm:grid-cols-2 gap-6 sm:gap-8 h-full p-6 sm:p-8">
			<div className="flex flex-col justify-center">
				<div className="relative mb-6">
					<img
						src="/app-logo.webp"
						alt="Flow Like Logo"
						className="w-28 h-28 mx-auto"
					/>
				</div>
				<h2 className="text-4xl font-bold text-center mb-4">
					<span className="text-primary">Flow</span> Like
				</h2>
				<div className="w-20 h-0.5 bg-primary mx-auto" />
			</div>

			<div className="flex flex-col justify-center space-y-6">
				<div className="space-y-3">
					<FeatureItem icon={Rocket} text="Scalable Development" />
					<FeatureItem icon={Zap} text="Lightning Fast Performance" />
					<FeatureItem icon={Heart} text="Developer Friendly" />
				</div>
			</div>
		</div>
	);

	const DocsStep = () => (
		<div className="grid grid-cols-1 sm:grid-cols-2 gap-6 sm:gap-8 h-full p-6 sm:p-8">
			<div className="flex flex-col justify-center items-center">
				<div
					className={
						supportsBackdrop
							? "w-28 h-28 bg-primary/20 backdrop-blur-md rounded-3xl flex items-center justify-center mb-6 border border-primary/30 shadow-lg"
							: "w-28 h-28 bg-primary/15 rounded-3xl flex items-center justify-center mb-6 border border-primary/30 shadow-lg"
					}
				>
					<Book className="w-14 h-14 text-primary" />
				</div>
			</div>

			<div className="flex flex-col justify-center space-y-6">
				<div className="space-y-3">
					<BulletPoint text="Quick Start Guide" />
					<BulletPoint text="API Reference" />
					<BulletPoint text="Best Practices" />
					<BulletPoint text="Advanced Features" />
				</div>
				<Button
					className={
						supportsBackdrop
							? "gap-2 w-fit bg-primary/90 backdrop-blur-xs hover:bg-primary"
							: "gap-2 w-fit bg-primary hover:bg-primary/90"
					}
					onClick={() => window.open("https://docs.flow-like.com", "_blank")}
				>
					<Book className="w-4 h-4" />
					Open Documentation
				</Button>
			</div>
		</div>
	);

	const DiscordStep = () => (
		<div className="grid grid-cols-1 sm:grid-cols-2 gap-6 sm:gap-8 h-full p-6 sm:p-8">
			<div className="flex flex-col justify-center items-center">
				<div
					className={
						supportsBackdrop
							? "w-28 h-28 bg-[#5865F2]/20 backdrop-blur-md rounded-3xl flex items-center justify-center mb-6 border border-[#5865F2]/30 shadow-lg"
							: "w-28 h-28 bg-[#5865F2]/15 rounded-3xl flex items-center justify-center mb-6 border border-[#5865F2]/30 shadow-lg"
					}
				>
					<MessageCircle className="w-14 h-14 text-[#5865F2]" />
				</div>
			</div>

			<div className="flex flex-col justify-center space-y-6">
				<div className="space-y-3">
					<BulletPoint text="Get Help & Support" color="[#5865F2]" />
					<BulletPoint text="Share Your Projects" color="[#5865F2]" />
					<BulletPoint text="Feature Discussions" color="[#5865F2]" />
					<BulletPoint text="Connect with Developers" color="[#5865F2]" />
				</div>
				<Button
					className={
						supportsBackdrop
							? "gap-2 w-fit bg-[#5865F2]/90 backdrop-blur-xs hover:bg-[#5865F2] text-white"
							: "gap-2 w-fit bg-[#5865F2] hover:bg-[#5865F2]/90 text-white"
					}
					onClick={() => window.open("https://discord.gg/mdBA9kMjFJ", "_blank")}
				>
					<MessageCircle className="w-4 h-4" />
					Join Discord
				</Button>
			</div>
		</div>
	);

	const GithubStep = () => (
		<div className="grid grid-cols-1 sm:grid-cols-2 gap-6 sm:gap-8 h-full p-6 sm:p-8">
			<div className="flex flex-col justify-center items-center">
				<div
					className={
						supportsBackdrop
							? "w-28 h-28 bg-foreground/20 backdrop-blur-md rounded-3xl flex items-center justify-center mb-6 border border-foreground/30 shadow-lg"
							: "w-28 h-28 bg-foreground/15 rounded-3xl flex items-center justify-center mb-6 border border-foreground/30 shadow-lg"
					}
				>
					<GitHubLogoIcon className="w-14 h-14 text-foreground" />
				</div>
			</div>

			<div className="flex flex-col justify-center space-y-6">
				<div className="space-y-3">
					<BulletPoint text="Explore Source Code" color="foreground" />
					<BulletPoint text="Report Issues" color="foreground" />
					<BulletPoint text="Submit Pull Requests" color="foreground" />
					<BulletPoint text="Star the Repository" color="foreground" />
				</div>
				<Button
					variant="outline"
					className={
						supportsBackdrop
							? "gap-2 w-fit border-foreground/40 hover:bg-foreground/10 bg-background/30 backdrop-blur-xs"
							: "gap-2 w-fit border-foreground/40 hover:bg-foreground/10 bg-card"
					}
					onClick={() =>
						window.open("https://github.com/TM9657/flow-like", "_blank")
					}
				>
					<GitHubLogoIcon className="w-4 h-4" />
					View Repository
				</Button>
			</div>
		</div>
	);

	const stepComponents = {
		welcome: WelcomeStep,
		docs: DocsStep,
		discord: DiscordStep,
		github: GithubStep,
	};

	const CurrentStepComponent = stepComponents[currentStep];

	if (!showTutorial) return null;

	return (
		<div className="fixed inset-0 z-50">
			<AnimatedBackground variant={currentStep}>
				{/* Container: mobile full-screen, desktop card */}
				<div
					className={
						`w-full sm:w-[750px] max-w-[100vw] sm:max-w-[90vw] h-[100dvh] sm:h-auto ${
							supportsBackdrop
								? "bg-background/25 backdrop-blur-2xl border"
								: "bg-card border"
						} border-border/40 sm:rounded-3xl sm:shadow-2xl overflow-hidden flex flex-col`
					}
				>
					<div
						className={
							supportsBackdrop
								? "p-6 sm:p-8 border-b border-border/30 bg-background/15 backdrop-blur-xl"
								: "p-6 sm:p-8 border-b border-border/30 bg-card"
						}
					>
						<div className="flex items-center justify-between">
							<div className="text-center flex-1">
								<h1 className="text-3xl font-bold">
									{stepData[currentStep].title}
								</h1>
								<p className="text-muted-foreground mt-2 text-lg">
									{stepData[currentStep].description}
								</p>
							</div>
							<Button
								variant="ghost"
								size="sm"
								className={supportsBackdrop ? "ml-6 hover:bg-background/40 backdrop-blur-xs rounded-xl" : "ml-6 hover:bg-muted/60 rounded-xl"}
								onClick={() => setShowTutorial(false)}
							>
								<X className="w-5 h-5" />
							</Button>
						</div>
					</div>

					{/* Body */}
					<div
						className={supportsBackdrop ? "flex-1 min-h-0 h-[calc(100dvh-200px)] sm:h-[450px] bg-background/10 backdrop-blur-lg" : "flex-1 min-h-0 h-[calc(100dvh-200px)] sm:h-[450px] bg-card"}
					>
						<CurrentStepComponent />
					</div>

					{/* Footer */}
					<div
						className={supportsBackdrop ? "p-6 sm:p-8 bg-background/15 backdrop-blur-xl border-t border-border/30" : "p-6 sm:p-8 bg-card border-t border-border/30"}
					>
						<div className="flex justify-center gap-3 mb-8">
							{STEPS.map((step) => (
								<div
									key={step}
									className={`w-3 h-3 rounded-full transition-all duration-300 ${
										step === currentStep
											? "bg-primary scale-125 shadow-lg"
											: "bg-muted-foreground/40"
									}`}
								/>
							))}
						</div>

						<div className="flex justify-between">
							<div>
								{currentStep !== "welcome" && (
									<Button
										variant="outline"
										onClick={handlePrevious}
										className={
											supportsBackdrop
												? "bg-background/40 backdrop-blur-md border-border/50 hover:bg-background/60 rounded-xl"
												: "bg-muted/60 border-border/50 hover:bg-muted rounded-xl"
										}
									>
										Previous
									</Button>
								)}
							</div>
							<div className="flex gap-4">
								<Button
									variant="ghost"
									onClick={handleSkip}
									className={supportsBackdrop ? "hover:bg-background/40 backdrop-blur-xs rounded-xl" : "hover:bg-muted rounded-xl"}
								>
									Skip Tour
								</Button>
								<Button
									onClick={handleNext}
									className={supportsBackdrop ? "bg-primary/90 backdrop-blur-xs hover:bg-primary rounded-xl" : "bg-primary hover:bg-primary/90 rounded-xl"}
								>
									{currentStep === "github" ? "Get Started" : "Next"}
								</Button>
							</div>
						</div>
					</div>
				</div>
			</AnimatedBackground>
		</div>
	);
}
