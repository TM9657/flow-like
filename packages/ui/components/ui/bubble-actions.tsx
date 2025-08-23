"use client";

import { MoreHorizontal } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { cn } from "../../lib";

interface BubbleAction {
	id: string;
	label: string;
	icon?: React.ReactNode;
	onClick: () => void;
	variant?: "default" | "destructive";
	disabled?: boolean;
}

interface BubbleActionsProps {
	actions: BubbleAction[];
	children?: React.ReactNode;
	className?: string;
	side?: "top" | "bottom" | "left" | "right";
	align?: "start" | "center" | "end";
	trigger?: "hover" | "click";
	style?: React.CSSProperties;
}

const BubbleMenu = ({
	actions,
	position,
	side,
	align,
	style,
	onActionClick,
	onMouseEnter,
	onMouseLeave,
}: {
	actions: BubbleAction[];
	position: { x: number; y: number };
	side: "top" | "bottom" | "left" | "right";
	align: "start" | "center" | "end";
	style?: React.CSSProperties;
	onActionClick: (action: BubbleAction) => void;
	onMouseEnter?: () => void;
	onMouseLeave?: () => void;
}) => {
	const isVertical = side === "left" || side === "right";

	const getTransform = () => {
		if (side === "top" || side === "bottom") {
			return align === "center"
				? "translateX(-50%)"
				: align === "start"
					? "translateX(0)"
					: "translateX(-100%)";
		}
		return align === "center"
			? "translateY(-50%)"
			: align === "start"
				? "translateY(0)"
				: "translateY(-100%)";
	};

	return (
		<div
			className="fixed z-9999 pointer-events-auto"
			onMouseEnter={onMouseEnter}
			onMouseLeave={onMouseLeave}
			style={{
				left: position.x,
				top: position.y,
				transform: getTransform(),
				...style,
			}}
		>
			<div
				className={cn(
					"bg-background/95 backdrop-blur-xs border border-border/50 rounded-full shadow-xl",
					"animate-in fade-in-0 zoom-in-95 duration-200",
					"flex items-center gap-1 p-1",
					isVertical ? "flex-col" : "flex-row",
				)}
			>
				{actions.map((action, index) => (
					<button
						key={action.id}
						onClick={() => onActionClick(action)}
						disabled={action.disabled}
						title={action.label}
						className={cn(
							"w-8 h-8 rounded-full flex items-center justify-center",
							"hover:scale-110 active:scale-95 transition-all duration-200",
							"hover:bg-accent hover:text-accent-foreground",
							"disabled:opacity-50 disabled:pointer-events-none",
							"focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring",
							action.variant === "destructive" &&
								"text-destructive-foreground hover:bg-destructive",
						)}
						style={{
							animationDelay: `${index * 50}ms`,
						}}
					>
						<span className="w-4 h-4 flex items-center justify-center">
							{action.icon}
						</span>
					</button>
				))}
			</div>
		</div>
	);
};

const DefaultTrigger = () => (
	<button
		type="button"
		className="w-8 h-8 rounded-full flex items-center justify-center bg-background/80 backdrop-blur-xs border border-border/50 hover:bg-accent hover:text-accent-foreground transition-all duration-200 hover:scale-105 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring shadow-sm"
	>
		<MoreHorizontal className="w-4 h-4" />
	</button>
);

export function BubbleActions({
	actions,
	children,
	className,
	style,
	side = "top",
	align = "center",
	trigger = "hover",
}: Readonly<BubbleActionsProps>) {
	const [isOpen, setIsOpen] = useState(false);
	const [position, setPosition] = useState({ x: 0, y: 0 });
	const triggerRef = useRef<HTMLElement>(null);
	const timeoutRef = useRef<NodeJS.Timeout>(null);

	const calculatePosition = useCallback(() => {
		if (!triggerRef.current) return;

		const rect = triggerRef.current.getBoundingClientRect();
		const offset = 12;

		const positions = {
			top: {
				x:
					align === "start"
						? rect.left
						: align === "end"
							? rect.right
							: rect.left + rect.width / 2,
				y: rect.top - offset,
			},
			bottom: {
				x:
					align === "start"
						? rect.left
						: align === "end"
							? rect.right
							: rect.left + rect.width / 2,
				y: rect.bottom + offset,
			},
			left: {
				x: rect.left - offset,
				y:
					align === "start"
						? rect.top
						: align === "end"
							? rect.bottom
							: rect.top + rect.height / 2,
			},
			right: {
				x: rect.right + offset,
				y:
					align === "start"
						? rect.top
						: align === "end"
							? rect.bottom
							: rect.top + rect.height / 2,
			},
		};

		setPosition(positions[side]);
	}, [side, align]);

	const handleMouseEnter = useCallback(() => {
		if (timeoutRef.current) {
			clearTimeout(timeoutRef.current);
		}
		setIsOpen(true);
	}, []);

	const handleMouseLeave = useCallback(() => {
		timeoutRef.current = setTimeout(() => setIsOpen(false), 200);
	}, []);

	useEffect(() => {
		if (isOpen) {
			calculatePosition();
			const handleUpdate = () => calculatePosition();
			window.addEventListener("scroll", handleUpdate, true);
			window.addEventListener("resize", handleUpdate);
			return () => {
				window.removeEventListener("scroll", handleUpdate, true);
				window.removeEventListener("resize", handleUpdate);
			};
		}
	}, [isOpen, calculatePosition]);

	useEffect(() => {
		if (trigger === "click" && isOpen) {
			const handleClickOutside = (event: MouseEvent) => {
				if (
					triggerRef.current &&
					!triggerRef.current.contains(event.target as Node)
				) {
					setIsOpen(false);
				}
			};
			document.addEventListener("mousedown", handleClickOutside);
			return () =>
				document.removeEventListener("mousedown", handleClickOutside);
		}
	}, [isOpen, trigger]);

	useEffect(
		() => () => {
			if (timeoutRef.current) clearTimeout(timeoutRef.current);
		},
		[],
	);

	const handleActionClick = useCallback((action: BubbleAction) => {
		action.onClick();
		setIsOpen(false);
	}, []);

	const triggerProps =
		trigger === "hover"
			? { onMouseEnter: handleMouseEnter, onMouseLeave: handleMouseLeave }
			: { onClick: () => setIsOpen(!isOpen) };

	return (
		<>
			<span ref={triggerRef} {...triggerProps} className={className}>
				{children || <DefaultTrigger />}
			</span>

			{isOpen &&
				createPortal(
					<BubbleMenu
						style={style}
						actions={actions}
						position={position}
						side={side}
						align={align}
						onActionClick={handleActionClick}
						onMouseEnter={trigger === "hover" ? handleMouseEnter : undefined}
						onMouseLeave={trigger === "hover" ? handleMouseLeave : undefined}
					/>,
					document.body,
				)}
		</>
	);
}
