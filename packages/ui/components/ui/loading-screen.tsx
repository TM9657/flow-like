"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { cn } from "../../lib";

interface LoadingScreenProps {
	message?: string;
	progress?: number;
	className?: string;
}

const loadingMessages = [
	"Initializing flow engine...",
	"Connecting neural pathways...",
	"Syncing data streams...",
	"Building execution graph...",
	"Preparing workspace...",
	"Loading node catalog...",
	"Establishing connections...",
	"Compiling workflows...",
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

function FlowGraph() {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const containerRef = useRef<HTMLDivElement>(null);
	const animationRef = useRef<number>(0);
	const nodesRef = useRef<Node[]>([]);
	const connectionsRef = useRef<Connection[]>([]);
	const mouseRef = useRef({ x: -1000, y: -1000 });
	const [isReady, setIsReady] = useState(false);

	const initializeGraph = useCallback(
		(width: number, height: number) => {
			// Ensure we have valid dimensions
			if (width <= 0 || height <= 0) return;

			const nodeCount = Math.max(15, Math.min(Math.floor((width * height) / 40000), 25));
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
		},
		[],
	);

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
				style={{ opacity: isReady ? 0.8 : 0, transition: "opacity 0.1s ease-in" }}
			/>
		</div>
	);
}

function PulsingRings({ progress }: { progress: number }) {
	return (
		<div className="relative w-32 h-32">
			{[0, 1, 2].map((i) => (
				<div
					key={i}
					className="absolute inset-0 rounded-full border-2 border-primary/20"
					style={{
						animation: `pulse-ring 2s ease-out infinite`,
						animationDelay: `${i * 0.4}s`,
					}}
				/>
			))}

			{/* Progress ring */}
			<svg className="absolute inset-0 w-full h-full -rotate-90">
				<circle
					cx="64"
					cy="64"
					r="58"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					className="text-muted/30"
				/>
				<circle
					cx="64"
					cy="64"
					r="58"
					fill="none"
					stroke="url(#gradient)"
					strokeWidth="3"
					strokeLinecap="round"
					strokeDasharray={`${2 * Math.PI * 58}`}
					strokeDashoffset={`${2 * Math.PI * 58 * (1 - progress / 100)}`}
					className="transition-all duration-500 ease-out"
				/>
				<defs>
					<linearGradient id="gradient" x1="0%" y1="0%" x2="100%" y2="100%">
						<stop offset="0%" stopColor="rgb(59, 130, 246)" />
						<stop offset="50%" stopColor="rgb(168, 85, 247)" />
						<stop offset="100%" stopColor="rgb(236, 72, 153)" />
					</linearGradient>
				</defs>
			</svg>

			{/* Center icon */}
			<div className="absolute inset-0 flex items-center justify-center">
				<div className="relative">
					<div className="w-12 h-12 rounded-xl bg-gradient-to-br from-primary/20 to-primary/5 backdrop-blur-sm border border-primary/20 flex items-center justify-center">
						<svg
							className="w-6 h-6 text-primary animate-pulse"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							strokeWidth="2"
						>
							<path d="M12 2L2 7l10 5 10-5-10-5z" />
							<path d="M2 17l10 5 10-5" />
							<path d="M2 12l10 5 10-5" />
						</svg>
					</div>
				</div>
			</div>
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
		}, 2500);

		return () => clearInterval(interval);
	}, [message]);

	const dots = useMemo(() => {
		const count = Math.floor((Date.now() / 400) % 4);
		return ".".repeat(count);
	}, [messageIndex]);

	const [animatedDots, setAnimatedDots] = useState("");
	useEffect(() => {
		const interval = setInterval(() => {
			setAnimatedDots((prev) => (prev.length >= 3 ? "" : prev + "."));
		}, 400);
		return () => clearInterval(interval);
	}, []);

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

			{/* Interactive flow graph */}
			<FlowGraph />

			{/* Subtle grid overlay */}
			<div
				className="absolute inset-0 opacity-[0.03]"
				style={{
					backgroundImage: `linear-gradient(rgba(255,255,255,0.1) 1px, transparent 1px),
                                    linear-gradient(90deg, rgba(255,255,255,0.1) 1px, transparent 1px)`,
					backgroundSize: "60px 60px",
				}}
			/>

			{/* Main content */}
			<div className="relative z-10 flex flex-col items-center gap-8">
				<PulsingRings progress={progress} />

				{/* Text content */}
				<div className="text-center space-y-3 max-w-md px-6">
					<h2 className="text-lg font-medium text-foreground/90 tracking-wide">
						{currentMessage}
						<span className="text-primary">{animatedDots}</span>
					</h2>

					{progress > 0 && (
						<div className="space-y-2">
							<div className="h-1 w-64 mx-auto bg-muted/50 rounded-full overflow-hidden">
								<div
									className="h-full bg-gradient-to-r from-blue-500 via-purple-500 to-pink-500 rounded-full transition-all duration-500 ease-out relative"
									style={{ width: `${Math.min(progress, 100)}%` }}
								>
									<div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/30 to-transparent animate-shimmer" />
								</div>
							</div>
							<p className="text-xs text-muted-foreground font-mono">
								{Math.round(progress)}%
							</p>
						</div>
					)}

					<p className="text-sm text-muted-foreground/70">
						Setting up your workspace
					</p>
				</div>

				{/* Animated dots indicator */}
				<div className="flex gap-1.5">
					{[0, 1, 2, 3, 4].map((i) => (
						<div
							key={i}
							className="w-1.5 h-1.5 rounded-full bg-primary/60"
							style={{
								animation: "dot-wave 1.4s ease-in-out infinite",
								animationDelay: `${i * 0.1}s`,
							}}
						/>
					))}
				</div>
			</div>

			<style jsx>{`
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

				@keyframes dot-wave {
					0%,
					60%,
					100% {
						transform: translateY(0);
						opacity: 0.4;
					}
					30% {
						transform: translateY(-6px);
						opacity: 1;
					}
				}

				@keyframes shimmer {
					0% {
						transform: translateX(-100%);
					}
					100% {
						transform: translateX(100%);
					}
				}

				.animate-shimmer {
					animation: shimmer 1.5s ease-in-out infinite;
				}

				.animate-pulse-slow {
					animation: pulse 4s ease-in-out infinite;
				}
			`}</style>
		</div>
	);
}
