import { Suspense } from "react";

export default function OnboardingLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<div
			className={
				"fixed inset-0 bg-background z-50 flex flex-col justify-center items-center overflow-auto min-h-[100svh] max-w-[100dvw] p-4"
			}
		>
			<Suspense>{children}</Suspense>
		</div>
	);
}
