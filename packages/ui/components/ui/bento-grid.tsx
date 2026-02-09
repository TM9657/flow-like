import { cn } from "../../lib/utils";

export const BentoGrid = ({
	className,
	children,
}: {
	className?: string;
	children?: React.ReactNode;
}) => {
	return (
		<div
			className={cn(
				"grid md:auto-rows-[18rem] grid-cols-1 md:grid-cols-3 gap-4 mx-auto max-w-(--breakpoint-2xl)",
				className,
			)}
		>
			{children}
		</div>
	);
};

export const BentoGridItem = ({
	className,
	title,
	description,
	header,
	icon,
}: {
	className?: string;
	title?: string | React.ReactNode;
	description?: string | React.ReactNode;
	header?: React.ReactNode;
	icon?: React.ReactNode;
}) => {
	return (
		<div
			className={cn(
				"row-span-1 rounded-xl group/bento hover:shadow-xl transition-all duration-300 shadow-sm p-4 bg-card/80 backdrop-blur-sm border border-border/50 hover:border-primary/20 hover:bg-card justify-between flex flex-col space-y-4 color-card-foreground relative overflow-hidden",
				className,
			)}
		>
			{/* Subtle gradient overlay on hover */}
			<div className="absolute inset-0 bg-gradient-to-br from-primary/[0.02] to-transparent opacity-0 group-hover/bento:opacity-100 transition-opacity duration-300 pointer-events-none" />
			<div className="relative z-10">{header}</div>
			<div className="relative z-10 transition-all duration-200">
				{icon}
				<div className="font-sans font-bold text-foreground mb-2 mt-2">
					{title}
				</div>
				<div className="font-sans font-normal text-muted-foreground text-xs">
					{description}
				</div>
			</div>
		</div>
	);
};
