"use client";
import { motion } from "framer-motion";
import { cn } from "../../lib/utils";

/** Box lid lifts up then the body draws in. Turns lime on hover. */
export function AnimatedPackageIcon({ className }: { className?: string }) {
	return (
		<motion.svg
			xmlns="http://www.w3.org/2000/svg"
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2"
			strokeLinecap="round"
			strokeLinejoin="round"
			className={cn("group-hover/icon:text-lime-400 transition-colors duration-150", className)}
		>
			{/* Box body */}
			<motion.path
				d="M12 22L3 17V7l9-5 9 5v10z"
				variants={{
					initial: { pathLength: 1 },
					hover: {
						pathLength: [0, 1],
						transition: { duration: 0.5, ease: "easeInOut" },
					},
				}}
			/>
			{/* Centre seam */}
			<motion.line
				x1="12"
				y1="22"
				x2="12"
				y2="12"
				variants={{
					initial: { pathLength: 1 },
					hover: {
						pathLength: [0, 1],
						transition: { duration: 0.25, ease: "linear", delay: 0.45 },
					},
				}}
			/>
			{/* Horizontal strap */}
			<motion.path
				d="M3.3 7L12 12l8.7-5"
				variants={{
					initial: { pathLength: 1 },
					hover: {
						pathLength: [0, 1],
						transition: { duration: 0.25, ease: "linear", delay: 0.5 },
					},
				}}
			/>
			{/* Package label */}
			<motion.path
				d="M16 5.5l-4 2.3-4-2.3"
				variants={{
					initial: { pathLength: 1 },
					hover: {
						pathLength: [0, 1],
						transition: { duration: 0.2, ease: "linear", delay: 0.65 },
					},
				}}
			/>
		</motion.svg>
	);
}
