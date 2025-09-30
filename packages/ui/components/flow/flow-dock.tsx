/**
 * Note: Use position fixed according to your needs
 * Desktop navbar is better positioned at the bottom
 * Mobile navbar is better positioned at bottom right.
 **/

import { PanelTopOpen } from "lucide-react";
import {
	AnimatePresence,
	type MotionValue,
	motion,
	useMotionValue,
	useSpring,
	useTransform,
} from "framer-motion";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { cn } from "../../lib/utils";

type IFlowDockItem = {
	title: string;
	icon: React.ReactNode;
	onClick: () => Promise<void>;
	separator?: string;
	highlight?: boolean;
};

export const FlowDock = memo(
	({
		items,
		desktopClassName,
		mobileClassName,
	}: {
		items: IFlowDockItem[];
		desktopClassName?: string;
		mobileClassName?: string;
	}) => {
		return (
			<>
				<FlowDockDesktop items={items} className={desktopClassName} />
				<FlowDockMobile items={items} className={mobileClassName} />
			</>
		);
	},
);

const FlowDockMobile = memo(
	({
		items,
		className,
	}: {
		items: IFlowDockItem[];
		className?: string;
	}) => {
		const [open, setOpen] = useState(false);
		const [placement, setPlacement] = useState<"up" | "down" | "left" | "right">(
			"down",
		);
		const containerRef = useRef<HTMLDivElement>(null);

		const handleToggle = useCallback(() => {
			setOpen((prev) => !prev);
		}, []);

		const handleItemSelected = useCallback(() => {
			setOpen(false);
		}, []);

		const computePlacement = useCallback(() => {
			const el = containerRef.current;
			if (!el) return;
			const rect = el.getBoundingClientRect();

			const availableUp = rect.top;
			const availableDown = window.innerHeight - rect.bottom;
			const availableLeft = rect.left;
			const availableRight = window.innerWidth - rect.right;

			// Smaller buttons (h-7 w-7) + gap-1.5
			const itemSize = 28;
			const gap = 6;
			const neededVertical = items.length * itemSize + Math.max(items.length - 1, 0) * gap;

			if (availableDown >= Math.min(neededVertical, 120)) {
				setPlacement("down");
				return;
			}
			if (availableUp >= Math.min(neededVertical, 120)) {
				setPlacement("up");
				return;
			}
			setPlacement(availableRight >= availableLeft ? "right" : "left");
		}, [items.length]);

		useEffect(() => {
			if (!open) return;
			computePlacement();
			const onResize = () => computePlacement();
			window.addEventListener("resize", onResize);

			const onClickOutside = (e: MouseEvent) => {
				if (!containerRef.current) return;
				if (!containerRef.current.contains(e.target as Node)) setOpen(false);
			};
			const onKey = (e: KeyboardEvent) => {
				if (e.key === "Escape") setOpen(false);
			};
			document.addEventListener("mousedown", onClickOutside);
			document.addEventListener("keydown", onKey);
			return () => {
				window.removeEventListener("resize", onResize);
				document.removeEventListener("mousedown", onClickOutside);
				document.removeEventListener("keydown", onKey);
			};
		}, [open, computePlacement]);

		const mobileItems = useMemo(
			() =>
				items.map((item, idx) => (
					<MobileItem
						key={item.title}
						item={item}
						index={idx}
						totalItems={items.length}
						placement={placement}
						onSelected={handleItemSelected}
					/>
				)),
			[items, placement, handleItemSelected],
		);

		return (
			<div
				ref={containerRef}
				className={cn(
					"relative inline-flex md:hidden items-center justify-center",
					className,
				)}
			>
				<AnimatePresence>
					{open && (
						<motion.div
							layoutId="nav"
							className={cn(
								"absolute z-50 rounded-xl border bg-popover/90 backdrop-blur p-1 shadow-lg",
								placement === "down" &&
									"top-full mt-3 left-1/2 -translate-x-1/2 flex flex-col gap-1.5",
								placement === "up" &&
									"bottom-full mb-3 left-1/2 -translate-x-1/2 flex flex-col gap-1.5",
								placement === "left" &&
									"right-full mr-3 top-1/2 -translate-y-1/2 flex flex-row gap-1.5",
								placement === "right" &&
									"left-full ml-3 top-1/2 -translate-y-1/2 flex flex-row gap-1.5",
							)}
						>
							{mobileItems}
						</motion.div>
					)}
				</AnimatePresence>
				<button
					onClick={handleToggle}
					aria-label="Toggle actions"
					aria-expanded={open}
					className="h-8 w-8 rounded-full bg-secondary hover:bg-secondary/80 flex items-center justify-center transition-colors"
				>
					<PanelTopOpen className="h-3.5 w-3.5 text-muted-foreground" />
				</button>
			</div>
		);
	},
);

const MobileItem = memo(
	({
		item,
		index,
		totalItems,
		placement,
		onSelected,
	}: {
		item: IFlowDockItem;
		index: number;
		totalItems: number;
		placement: "up" | "down" | "left" | "right";
		onSelected: () => void;
	}) => {
		const handleClick = useCallback(async () => {
			await item.onClick();
			onSelected();
		}, [item.onClick, onSelected]);

		const initialByPlacement = useMemo(() => {
			switch (placement) {
				case "down":
					return { opacity: 0, y: -8 } as const;
				case "up":
					return { opacity: 0, y: 8 } as const;
				case "left":
					return { opacity: 0, x: 8 } as const;
				case "right":
					return { opacity: 0, x: -8 } as const;
				default:
					return { opacity: 0 } as const;
			}
		}, [placement]);

		const exitByPlacement = useMemo(() => {
			switch (placement) {
				case "down":
					return { opacity: 0, y: -8, transition: { delay: index * 0.04 } } as const;
				case "up":
					return { opacity: 0, y: 8, transition: { delay: index * 0.04 } } as const;
				case "left":
					return { opacity: 0, x: 8, transition: { delay: index * 0.04 } } as const;
				case "right":
					return { opacity: 0, x: -8, transition: { delay: index * 0.04 } } as const;
				default:
					return { opacity: 0 } as const;
			}
		}, [placement, index]);

		return (
			<motion.div
				initial={initialByPlacement}
				animate={{ opacity: 1, x: 0, y: 0 }}
				exit={exitByPlacement}
				transition={{ delay: (totalItems - 1 - index) * 0.04 }}
			>
				<button
					onClick={handleClick}
					className="h-7 w-7 rounded-full bg-secondary hover:bg-secondary/80 flex items-center justify-center transition-colors"
				>
					<div className="h-3.5 w-3.5">{item.icon}</div>
				</button>
			</motion.div>
		);
	},
);

const FlowDockDesktop = memo(
	({
		items,
		className,
	}: {
		items: IFlowDockItem[];
		className?: string;
	}) => {
		const mouseX = useMotionValue(Number.POSITIVE_INFINITY);

		const handleMouseMove = useCallback(
			(e: React.MouseEvent) => {
				mouseX.set(e.pageX);
			},
			[mouseX],
		);

		const handleMouseLeave = useCallback(() => {
			mouseX.set(Number.POSITIVE_INFINITY);
		}, [mouseX]);

		const desktopItems = useMemo(
			() =>
				items.map((item) => (
					<div key={item.title} className="flex items-center gap-4">
						{item.separator === "left" && (
							<div className="h-10 w-[2px] rounded-full bg-border" />
						)}
						<IconContainer mouseX={mouseX} {...item} />
						{item.separator === "right" && (
							<div className="h-10 w-[2px] rounded-full bg-border" />
						)}
					</div>
				)),
			[items, mouseX],
		);

		return (
			<motion.div
				onMouseMove={handleMouseMove}
				onMouseLeave={handleMouseLeave}
				className={cn(
					"mx-auto hidden md:flex h-16 gap-4 items-end rounded-2xl bg-card border px-4 pb-3",
					className,
				)}
			>
				{desktopItems}
			</motion.div>
		);
	},
);

const IconContainer = memo(
	({
		mouseX,
		title,
		icon,
		highlight,
		onClick,
	}: Readonly<{
		mouseX: MotionValue;
		title: string;
		highlight?: boolean;
		icon: React.ReactNode;
		onClick: () => Promise<void>;
	}>) => {
		const ref = useRef<HTMLDivElement>(null);

		const distance = useTransform(mouseX, (val) => {
			const bounds = ref.current?.getBoundingClientRect() ?? { x: 0, width: 0 };
			return val - bounds.x - bounds.width / 2;
		});

		const widthTransform = useTransform(distance, [-150, 0, 150], [40, 80, 40]);
		const heightTransform = useTransform(
			distance,
			[-150, 0, 150],
			[40, 80, 40],
		);

		const widthTransformIcon = useTransform(
			distance,
			[-150, 0, 150],
			[20, 40, 20],
		);
		const heightTransformIcon = useTransform(
			distance,
			[-150, 0, 150],
			[20, 40, 20],
		);

		const springConfig = useMemo(
			() => ({
				mass: 0.1,
				stiffness: 150,
				damping: 12,
			}),
			[],
		);

		const width = useSpring(widthTransform, springConfig);
		const height = useSpring(heightTransform, springConfig);
		const widthIcon = useSpring(widthTransformIcon, springConfig);
		const heightIcon = useSpring(heightTransformIcon, springConfig);

		const [hovered, setHovered] = useState(false);

		const handleMouseEnter = useCallback(() => setHovered(true), []);
		const handleMouseLeave = useCallback(() => setHovered(false), []);

		return (
			<button onClick={onClick}>
				<motion.div
					ref={ref}
					style={{ width, height }}
					onMouseEnter={handleMouseEnter}
					onMouseLeave={handleMouseLeave}
					className={cn(
						"aspect-square rounded-full bg-secondary hover:bg-secondary/80 text-secondary-foreground flex items-center justify-center relative transition-colors",
						highlight &&
							"bg-primary hover:bg-primary/90 text-primary-foreground",
					)}
				>
					<motion.div
						style={{ width: widthIcon, height: heightIcon }}
						className="flex items-center justify-center"
					>
						{icon}
					</motion.div>
					<AnimatePresence>
						{hovered && (
							<motion.div
								initial={{ opacity: 0, y: 10, x: "-50%" }}
								animate={{ opacity: 1, y: 0, x: "-50%" }}
								exit={{ opacity: 0, y: 2, x: "-50%" }}
								className="px-2 py-0.5 whitespace-pre rounded-md bg-popover text-popover-foreground border absolute left-1/2  -bottom-8 w-fit text-xs"
							>
								{title}
							</motion.div>
						)}
					</AnimatePresence>
				</motion.div>
			</button>
		);
	},
);
