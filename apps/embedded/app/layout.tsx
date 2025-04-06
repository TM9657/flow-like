"use client";
import "@tm9657/flow-like-ui/globals.css";

import {
	PersistQueryClientProvider,
	QueryClient,
	ReactFlowProvider,
	ThemeProvider,
	Toaster,
	TooltipProvider,
	createIDBPersister,
} from "@tm9657/flow-like-ui";
import { Inter } from "next/font/google";
import { Suspense } from "react";

const inter = Inter({ subsets: ["latin"] });

const persister = createIDBPersister();
const queryClient = new QueryClient();

export default function RootLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<html lang="en" suppressHydrationWarning suppressContentEditableWarning>
			{/* <ReactScan /> */}
			{/* <PHProvider> */}
			<ReactFlowProvider>
				<PersistQueryClientProvider
					client={queryClient}
					persistOptions={{
						persister,
					}}
				>
					<TooltipProvider>
						<Toaster />
						<body className={inter.className}>
							<Suspense
								fallback={
									<div className="flex flex-1 justify-center items-center">
										{"Loading..."}
									</div>
								}
							>
								<ThemeProvider
									attribute="class"
									defaultTheme="system"
									enableSystem
									disableTransitionOnChange
								>
									{children}
								</ThemeProvider>
							</Suspense>
						</body>
					</TooltipProvider>
				</PersistQueryClientProvider>
			</ReactFlowProvider>
			{/* </PHProvider> */}
		</html>
	);
}
