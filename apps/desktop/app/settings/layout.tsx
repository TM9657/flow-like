"use client";
export default function Layout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<main className="flex min-h-screen flex-col w-full p-4 max-h-[100dvh] overflow-y-auto">
			{children}
		</main>
	);
}
