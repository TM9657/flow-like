"use client";
export default function Layout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<main className="flex flex-col w-full overflow-hidden p-4 flex-1 min-h-0">
			{children}
		</main>
	);
}
