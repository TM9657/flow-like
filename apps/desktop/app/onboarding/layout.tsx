import { Suspense } from "react";

export default function OnboardingLayout({
	children,
}: Readonly<{ children: React.ReactNode }>) {
	return (
		<div
			className={[
				"fixed inset-0 z-50 bg-background",
				// Let the container size to the viewport via inset, avoid double sizing with padding
				"overflow-y-auto overscroll-y-contain",
				// Start content at the top on small screens to avoid cut-off; center on md+
				"flex flex-col items-center justify-start md:justify-center",
				"p-4",
				"pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]",
			].join(" ")}
		>
			<BackgroundFX />
			<Suspense fallback={null}>{children}</Suspense>
		</div>
	);
}

const BackgroundFX = () => (
	<div aria-hidden="true" className="pointer-events-none fixed inset-0 z-0 overflow-hidden">
		{/* Subtle gradient wash */}
		<div className="absolute inset-0 bg-gradient-to-b from-primary/10 via-background to-secondary/10" />
		{/* Animated blobs */}
		<div className="absolute -top-24 -left-24 h-80 w-80 rounded-full bg-gradient-to-tr from-primary/30 via-primary/20 to-transparent blur-3xl animate-pulse" />
		<div className="absolute -bottom-32 -right-24 h-[28rem] w-[28rem] rounded-full bg-gradient-to-tr from-secondary/30 via-secondary/20 to-transparent blur-3xl animate-[spin_40s_linear_infinite]" />
	</div>
);