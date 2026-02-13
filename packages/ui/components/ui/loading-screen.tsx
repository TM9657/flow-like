"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "../../lib";

interface LoadingScreenProps {
	message?: string;
	progress?: number;
	className?: string;
}

const loadingMessages = [
	"Initializing flow engine",
	"Connecting neural pathways",
	"Syncing data streams",
	"Building execution graph",
	"Preparing workspace",
	"Loading node catalog",
	"Establishing connections",
	"Compiling workflows",
];

interface Node {
	id: number;
	x: number;
	y: number;
	vx: number;
	vy: number;
	radius: number;
	pulse: number;
	color: string;
}

interface Connection {
	from: number;
	to: number;
	progress: number;
	speed: number;
	active: boolean;
}

const NODE_COLORS = [
	"rgba(59, 130, 246, 0.8)", // blue
	"rgba(168, 85, 247, 0.8)", // purple
	"rgba(236, 72, 153, 0.8)", // pink
	"rgba(34, 197, 94, 0.8)", // green
	"rgba(251, 146, 60, 0.8)", // orange
];

function seededRandom(seed: number) {
	const x = Math.sin(seed) * 10000;
	return x - Math.floor(x);
}

function MorphingBlob({
	delay = 0,
	className,
}: { delay?: number; className?: string }) {
	return (
		<div
			className={cn(
				"absolute rounded-full blur-3xl opacity-30 animate-morph",
				className,
			)}
			style={{
				animationDelay: `${delay}s`,
			}}
		/>
	);
}

function FloatingParticle({ index }: { index: number }) {
	const style = {
		left: `${seededRandom(index * 7) * 100}%`,
		animationDelay: `${seededRandom(index * 13) * 5}s`,
		animationDuration: `${8 + seededRandom(index * 17) * 7}s`,
	};

	return (
		<div
			className="absolute w-1 h-1 rounded-full bg-primary/40 animate-float-up"
			style={style}
		/>
	);
}

function GlowingOrb() {
	return (
		<div className="relative w-40 h-40 flex items-center justify-center">
			{/* Outer glow rings */}
			<div className="absolute inset-0 rounded-full bg-gradient-to-r from-blue-500/20 via-purple-500/20 to-pink-500/20 animate-spin-slow blur-xl" />
			<div className="absolute inset-2 rounded-full bg-gradient-to-r from-pink-500/15 via-blue-500/15 to-purple-500/15 animate-spin-reverse blur-lg" />

			{/* Orbiting particles */}
			{[...Array(3)].map((_, i) => (
				<div
					key={i}
					className="absolute w-2 h-2 rounded-full bg-primary animate-orbit"
					style={{
						animationDelay: `${i * -2}s`,
						animationDuration: "6s",
					}}
				/>
			))}

			{/* Main container */}
			<div className="relative w-28 h-28 rounded-2xl bg-gradient-to-br from-background/80 to-background/40 backdrop-blur-xl border border-white/10 shadow-2xl flex items-center justify-center overflow-hidden group">
				{/* Inner gradient animation */}
				<div className="absolute inset-0 bg-gradient-to-br from-blue-500/10 via-purple-500/10 to-pink-500/10 animate-gradient" />

				{/* Shimmer effect */}
				<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity">
					<div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full animate-shimmer-slow" />
				</div>

				{/* Flow icon */}
				<svg
					className="w-12 h-12 text-primary relative z-10"
					viewBox="0 0 24 24"
					fill="none"
				>
					{/* Animated flow paths */}
					<path
						d="M4 8h4a4 4 0 0 1 4 4v0a4 4 0 0 0 4 4h4"
						stroke="currentColor"
						strokeWidth="2"
						strokeLinecap="round"
						className="animate-dash"
						style={{ strokeDasharray: "60", strokeDashoffset: "60" }}
					/>
					<path
						d="M4 16h4a4 4 0 0 0 4-4v0a4 4 0 0 1 4-4h4"
						stroke="currentColor"
						strokeWidth="2"
						strokeLinecap="round"
						className="animate-dash-reverse"
						style={{ strokeDasharray: "60", strokeDashoffset: "60" }}
					/>
					{/* Node dots */}
					<circle
						cx="4"
						cy="8"
						r="2"
						fill="currentColor"
						className="animate-pulse"
					/>
					<circle
						cx="4"
						cy="16"
						r="2"
						fill="currentColor"
						className="animate-pulse"
						style={{ animationDelay: "0.2s" }}
					/>
					<circle
						cx="12"
						cy="12"
						r="2.5"
						fill="currentColor"
						className="animate-pulse"
						style={{ animationDelay: "0.4s" }}
					/>
					<circle
						cx="20"
						cy="8"
						r="2"
						fill="currentColor"
						className="animate-pulse"
						style={{ animationDelay: "0.6s" }}
					/>
					<circle
						cx="20"
						cy="16"
						r="2"
						fill="currentColor"
						className="animate-pulse"
						style={{ animationDelay: "0.8s" }}
					/>
				</svg>
			</div>

			{/* Corner accents */}
			<div className="absolute top-4 left-4 w-8 h-8 border-l-2 border-t-2 border-primary/30 rounded-tl-lg animate-pulse" />
			<div
				className="absolute bottom-4 right-4 w-8 h-8 border-r-2 border-b-2 border-primary/30 rounded-br-lg animate-pulse"
				style={{ animationDelay: "0.5s" }}
			/>
		</div>
	);
}

function ModernProgressBar({ progress }: { progress: number }) {
	return (
		<div className="w-72 space-y-3">
			<div className="relative h-1.5 bg-muted/30 rounded-full overflow-hidden backdrop-blur-sm">
				{/* Background glow */}
				<div
					className="absolute inset-y-0 left-0 bg-gradient-to-r from-blue-500/50 via-purple-500/50 to-pink-500/50 blur-sm transition-all duration-700 ease-out"
					style={{ width: `${Math.min(progress, 100)}%` }}
				/>
				{/* Main bar */}
				<div
					className="absolute inset-y-0 left-0 bg-gradient-to-r from-blue-500 via-purple-500 to-pink-500 rounded-full transition-all duration-700 ease-out"
					style={{ width: `${Math.min(progress, 100)}%` }}
				>
					{/* Animated highlight */}
					<div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/40 to-transparent animate-progress-shine" />
				</div>
				{/* Leading glow dot */}
				{progress > 0 && progress < 100 && (
					<div
						className="absolute top-1/2 -translate-y-1/2 w-3 h-3 -ml-1.5 bg-white rounded-full shadow-lg shadow-purple-500/50 transition-all duration-700 ease-out"
						style={{ left: `${Math.min(progress, 100)}%` }}
					>
						<div className="absolute inset-0 rounded-full bg-white animate-ping opacity-75" />
					</div>
				)}
			</div>
			<div className="flex justify-between items-center text-xs">
				<span className="text-muted-foreground/60 font-medium">Loading</span>
				<span className="text-primary font-mono font-semibold">
					{Math.round(progress)}%
				</span>
			</div>
		</div>
	);
}

function TypewriterText({
	text,
	className,
}: { text: string; className?: string }) {
	const [displayText, setDisplayText] = useState("");
	const [showCursor, setShowCursor] = useState(true);

	useEffect(() => {
		setDisplayText("");
		let index = 0;
		const interval = setInterval(() => {
			if (index < text.length) {
				setDisplayText(text.slice(0, index + 1));
				index++;
			} else {
				clearInterval(interval);
			}
		}, 30);

		return () => clearInterval(interval);
	}, [text]);

	useEffect(() => {
		const cursorInterval = setInterval(() => {
			setShowCursor((prev) => !prev);
		}, 530);
		return () => clearInterval(cursorInterval);
	}, []);

	return (
		<span className={className}>
			{displayText}
			<span
				className={cn(
					"ml-0.5 text-primary",
					showCursor ? "opacity-100" : "opacity-0",
				)}
			>
				|
			</span>
		</span>
	);
}

function FlowGraph() {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const containerRef = useRef<HTMLDivElement>(null);
	const animationRef = useRef<number>(0);
	const nodesRef = useRef<Node[]>([]);
	const connectionsRef = useRef<Connection[]>([]);
	const mouseRef = useRef({ x: -1000, y: -1000 });
	const [isReady, setIsReady] = useState(false);

	const initializeGraph = useCallback((width: number, height: number) => {
		// Ensure we have valid dimensions
		if (width <= 0 || height <= 0) return;

		const nodeCount = Math.max(
			15,
			Math.min(Math.floor((width * height) / 40000), 25),
		);
		const nodes: Node[] = [];

		for (let i = 0; i < nodeCount; i++) {
			nodes.push({
				id: i,
				x: seededRandom(i * 7 + 1) * width,
				y: seededRandom(i * 13 + 2) * height,
				vx: (seededRandom(i * 17 + 3) - 0.5) * 0.5,
				vy: (seededRandom(i * 23 + 4) - 0.5) * 0.5,
				radius: 4 + seededRandom(i * 29 + 5) * 5,
				pulse: seededRandom(i * 31 + 6) * Math.PI * 2,
				color: NODE_COLORS[i % NODE_COLORS.length],
			});
		}

		const connections: Connection[] = [];
		for (let i = 0; i < nodes.length; i++) {
			const connectionCount = 1 + Math.floor(seededRandom(i * 37 + 7) * 2);
			for (let j = 0; j < connectionCount; j++) {
				const target = Math.floor(
					seededRandom(i * 41 + j * 43 + 8) * nodes.length,
				);
				if (target !== i) {
					connections.push({
						from: i,
						to: target,
						progress: seededRandom(i * 47 + j * 53 + 9),
						speed: 0.002 + seededRandom(i * 59 + j * 61 + 10) * 0.003,
						active: seededRandom(i * 67 + j * 71 + 11) > 0.3,
					});
				}
			}
		}

		nodesRef.current = nodes;
		connectionsRef.current = connections;
		setIsReady(true);
	}, []);

	useEffect(() => {
		const canvas = canvasRef.current;
		const container = containerRef.current;
		if (!canvas || !container) return;

		const ctx = canvas.getContext("2d");
		if (!ctx) return;

		const setupCanvas = () => {
			const rect = container.getBoundingClientRect();
			const width = rect.width || window.innerWidth;
			const height = rect.height || window.innerHeight;

			if (width <= 0 || height <= 0) return;

			const dpr = window.devicePixelRatio || 1;
			canvas.width = width * dpr;
			canvas.height = height * dpr;
			canvas.style.width = `${width}px`;
			canvas.style.height = `${height}px`;
			ctx.scale(dpr, dpr);
			initializeGraph(width, height);
		};

		// Initialize immediately, then also retry on next frame if needed
		setupCanvas();
		const initTimeout = requestAnimationFrame(() => {
			if (nodesRef.current.length === 0) setupCanvas();
		});

		const handleMouseMove = (e: MouseEvent) => {
			mouseRef.current = { x: e.clientX, y: e.clientY };
		};

		const handleResize = () => {
			setupCanvas();
		};

		window.addEventListener("resize", handleResize);
		window.addEventListener("mousemove", handleMouseMove);

		const animate = () => {
			const rect = container.getBoundingClientRect();
			const width = rect.width || window.innerWidth;
			const height = rect.height || window.innerHeight;
			const nodes = nodesRef.current;
			const connections = connectionsRef.current;
			const mouse = mouseRef.current;

			// Skip render if no nodes initialized yet
			if (nodes.length === 0) {
				animationRef.current = requestAnimationFrame(animate);
				return;
			}

			ctx.clearRect(0, 0, width, height);

			// Update and draw connections
			for (const conn of connections) {
				const fromNode = nodes[conn.from];
				const toNode = nodes[conn.to];
				if (!fromNode || !toNode) continue;

				const dx = toNode.x - fromNode.x;
				const dy = toNode.y - fromNode.y;
				const dist = Math.sqrt(dx * dx + dy * dy);

				if (dist > 300) continue;

				// Draw connection line
				const alpha = Math.max(0, 1 - dist / 300) * 0.15;
				ctx.beginPath();
				ctx.moveTo(fromNode.x, fromNode.y);
				ctx.lineTo(toNode.x, toNode.y);
				ctx.strokeStyle = `rgba(148, 163, 184, ${alpha})`;
				ctx.lineWidth = 1;
				ctx.stroke();

				// Animate data flow particle
				if (conn.active) {
					conn.progress += conn.speed;
					if (conn.progress > 1) conn.progress = 0;

					const px = fromNode.x + dx * conn.progress;
					const py = fromNode.y + dy * conn.progress;

					const gradient = ctx.createRadialGradient(px, py, 0, px, py, 6);
					gradient.addColorStop(0, "rgba(59, 130, 246, 0.8)");
					gradient.addColorStop(1, "rgba(59, 130, 246, 0)");

					ctx.beginPath();
					ctx.arc(px, py, 6, 0, Math.PI * 2);
					ctx.fillStyle = gradient;
					ctx.fill();
				}
			}

			// Update and draw nodes
			for (const node of nodes) {
				// Mouse interaction
				const mdx = mouse.x - node.x;
				const mdy = mouse.y - node.y;
				const mouseDist = Math.sqrt(mdx * mdx + mdy * mdy);
				if (mouseDist < 150 && mouseDist > 0) {
					const force = (150 - mouseDist) / 150;
					node.vx -= (mdx / mouseDist) * force * 0.1;
					node.vy -= (mdy / mouseDist) * force * 0.1;
				}

				// Update position
				node.x += node.vx;
				node.y += node.vy;
				node.vx *= 0.99;
				node.vy *= 0.99;
				node.pulse += 0.02;

				// Bounce off edges
				if (node.x < 0 || node.x > width) node.vx *= -1;
				if (node.y < 0 || node.y > height) node.vy *= -1;
				node.x = Math.max(0, Math.min(width, node.x));
				node.y = Math.max(0, Math.min(height, node.y));

				// Draw node glow
				const pulseScale = 1 + Math.sin(node.pulse) * 0.2;
				const glowRadius = node.radius * 3 * pulseScale;

				const glow = ctx.createRadialGradient(
					node.x,
					node.y,
					0,
					node.x,
					node.y,
					glowRadius,
				);
				glow.addColorStop(0, node.color.replace("0.8", "0.3"));
				glow.addColorStop(1, "transparent");

				ctx.beginPath();
				ctx.arc(node.x, node.y, glowRadius, 0, Math.PI * 2);
				ctx.fillStyle = glow;
				ctx.fill();

				// Draw node core
				ctx.beginPath();
				ctx.arc(node.x, node.y, node.radius * pulseScale, 0, Math.PI * 2);
				ctx.fillStyle = node.color;
				ctx.fill();

				// Inner highlight
				ctx.beginPath();
				ctx.arc(
					node.x - node.radius * 0.3,
					node.y - node.radius * 0.3,
					node.radius * 0.3,
					0,
					Math.PI * 2,
				);
				ctx.fillStyle = "rgba(255, 255, 255, 0.4)";
				ctx.fill();
			}

			animationRef.current = requestAnimationFrame(animate);
		};

		animate();

		return () => {
			cancelAnimationFrame(initTimeout);
			window.removeEventListener("resize", handleResize);
			window.removeEventListener("mousemove", handleMouseMove);
			cancelAnimationFrame(animationRef.current);
		};
	}, [initializeGraph]);

	return (
		<div ref={containerRef} className="absolute inset-0">
			<canvas
				ref={canvasRef}
				className="absolute inset-0 pointer-events-none"
				style={{
					opacity: isReady ? 0.8 : 0,
					transition: "opacity 0.1s ease-in",
				}}
			/>
		</div>
	);
}

function PulsingRings({ progress }: { progress: number }) {
	return (
		<div className="relative w-32 h-32 hidden">
			{/* Hidden - replaced by GlowingOrb */}
		</div>
	);
}

export function LoadingScreen({
	message,
	progress = 0,
	className,
}: Readonly<LoadingScreenProps>) {
	const [currentMessage, setCurrentMessage] = useState(
		message ?? loadingMessages[0],
	);
	const [messageIndex, setMessageIndex] = useState(0);

	useEffect(() => {
		if (message) {
			setCurrentMessage(message);
			return;
		}

		const interval = setInterval(() => {
			setMessageIndex((prev) => {
				const next = (prev + 1) % loadingMessages.length;
				setCurrentMessage(loadingMessages[next]);
				return next;
			});
		}, 3000);

		return () => clearInterval(interval);
	}, [message]);

	return (
		<div
			className={cn(
				"fixed inset-0 bg-background flex items-center justify-center overflow-hidden",
				className,
			)}
		>
			{/* Animated gradient background */}
			<div className="absolute inset-0 bg-gradient-to-br from-background via-background to-background">
				<div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top_left,rgba(59,130,246,0.15),transparent_50%)]" />
				<div className="absolute inset-0 bg-[radial-gradient(ellipse_at_bottom_right,rgba(168,85,247,0.15),transparent_50%)]" />
				<div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(236,72,153,0.08),transparent_60%)] animate-pulse-slow" />
			</div>

			{/* Morphing blobs */}
			<MorphingBlob
				delay={0}
				className="w-96 h-96 -top-48 -left-48 bg-gradient-to-br from-blue-500/30 to-purple-500/30"
			/>
			<MorphingBlob
				delay={2}
				className="w-80 h-80 -bottom-40 -right-40 bg-gradient-to-br from-purple-500/30 to-pink-500/30"
			/>
			<MorphingBlob
				delay={4}
				className="w-64 h-64 top-1/3 -right-32 bg-gradient-to-br from-pink-500/20 to-orange-500/20"
			/>

			{/* Interactive flow graph */}
			<FlowGraph />

			{/* Floating particles */}
			<div className="absolute inset-0 overflow-hidden pointer-events-none">
				{[...Array(20)].map((_, i) => (
					<FloatingParticle key={i} index={i} />
				))}
			</div>

			{/* Subtle grid overlay */}
			<div
				className="absolute inset-0 opacity-[0.02]"
				style={{
					backgroundImage: `linear-gradient(rgba(255,255,255,0.1) 1px, transparent 1px),
                                    linear-gradient(90deg, rgba(255,255,255,0.1) 1px, transparent 1px)`,
					backgroundSize: "60px 60px",
				}}
			/>

			{/* Vignette effect */}
			<div className="absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_0%,rgba(0,0,0,0.3)_100%)] pointer-events-none" />

			{/* Main content */}
			<div className="relative z-10 flex flex-col items-center gap-10 animate-fade-in">
				<GlowingOrb />

				{/* Text content with glassmorphism */}
				<div className="text-center space-y-6 max-w-md px-6">
					<div className="relative">
						<h2 className="text-xl font-medium text-foreground/90 tracking-wide h-7 overflow-hidden">
							<TypewriterText text={currentMessage} />
						</h2>
					</div>

					{progress > 0 && (
						<div className="animate-fade-in-delay">
							<ModernProgressBar progress={progress} />
						</div>
					)}

					<p className="text-sm text-muted-foreground/50 font-light">
						Preparing your creative workspace
					</p>
				</div>

				{/* Animated wave dots */}
				<div className="flex gap-2">
					{[...Array(5)].map((_, i) => (
						<div
							key={i}
							className="w-2 h-2 rounded-full bg-gradient-to-br from-primary to-primary/60"
							style={{
								animation: "wave-dot 1.4s ease-in-out infinite",
								animationDelay: `${i * 0.12}s`,
							}}
						/>
					))}
				</div>
			</div>

			<style>{`
				@keyframes morph {
					0%, 100% {
						border-radius: 60% 40% 30% 70% / 60% 30% 70% 40%;
						transform: rotate(0deg) scale(1);
					}
					25% {
						border-radius: 30% 60% 70% 40% / 50% 60% 30% 60%;
					}
					50% {
						border-radius: 50% 60% 30% 60% / 30% 40% 70% 50%;
						transform: rotate(180deg) scale(1.1);
					}
					75% {
						border-radius: 60% 30% 50% 40% / 60% 50% 30% 70%;
					}
				}

				.animate-morph {
					animation: morph 15s ease-in-out infinite;
				}

				@keyframes float-up {
					0% {
						transform: translateY(100vh) rotate(0deg);
						opacity: 0;
					}
					10% {
						opacity: 1;
					}
					90% {
						opacity: 1;
					}
					100% {
						transform: translateY(-100vh) rotate(360deg);
						opacity: 0;
					}
				}

				.animate-float-up {
					animation: float-up linear infinite;
				}

				@keyframes orbit {
					from {
						transform: rotate(0deg) translateX(70px) rotate(0deg);
					}
					to {
						transform: rotate(360deg) translateX(70px) rotate(-360deg);
					}
				}

				.animate-orbit {
					animation: orbit 6s linear infinite;
				}

				@keyframes spin-slow {
					from { transform: rotate(0deg); }
					to { transform: rotate(360deg); }
				}

				.animate-spin-slow {
					animation: spin-slow 20s linear infinite;
				}

				.animate-spin-reverse {
					animation: spin-slow 15s linear infinite reverse;
				}

				@keyframes gradient {
					0%, 100% {
						opacity: 0.5;
					}
					50% {
						opacity: 1;
					}
				}

				.animate-gradient {
					animation: gradient 3s ease-in-out infinite;
				}

				@keyframes dash {
					0% {
						stroke-dashoffset: 60;
					}
					50% {
						stroke-dashoffset: 0;
					}
					100% {
						stroke-dashoffset: -60;
					}
				}

				.animate-dash {
					animation: dash 3s ease-in-out infinite;
				}

				.animate-dash-reverse {
					animation: dash 3s ease-in-out infinite reverse;
				}

				@keyframes shimmer-slow {
					0% {
						transform: translateX(-100%);
					}
					100% {
						transform: translateX(200%);
					}
				}

				.animate-shimmer-slow {
					animation: shimmer-slow 3s ease-in-out infinite;
				}

				@keyframes progress-shine {
					0% {
						transform: translateX(-100%) skewX(-15deg);
					}
					100% {
						transform: translateX(300%) skewX(-15deg);
					}
				}

				.animate-progress-shine {
					animation: progress-shine 2s ease-in-out infinite;
				}

				@keyframes wave-dot {
					0%, 60%, 100% {
						transform: translateY(0) scale(1);
						opacity: 0.4;
					}
					30% {
						transform: translateY(-12px) scale(1.2);
						opacity: 1;
					}
				}

				@keyframes fade-in {
					from {
						opacity: 0;
						transform: translateY(10px);
					}
					to {
						opacity: 1;
						transform: translateY(0);
					}
				}

				.animate-fade-in {
					animation: fade-in 0.5s ease-out forwards;
				}

				.animate-fade-in-delay {
					animation: fade-in 0.5s ease-out 0.2s forwards;
					opacity: 0;
				}

				@keyframes pulse-ring {
					0% {
						transform: scale(1);
						opacity: 0.4;
					}
					100% {
						transform: scale(1.5);
						opacity: 0;
					}
				}

				.animate-pulse-slow {
					animation: pulse 4s ease-in-out infinite;
				}
			`}</style>
		</div>
	);
}
