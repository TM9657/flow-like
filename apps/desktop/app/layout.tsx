"use client";
import { QueryClient } from "@tanstack/react-query";
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import { ThemeProvider } from "@tm9657/flow-like-ui/components/theme-provider";
import { Toaster } from "@tm9657/flow-like-ui/components/ui/sonner";
import { TooltipProvider } from "@tm9657/flow-like-ui/components/ui/tooltip";
import "@tm9657/flow-like-ui/globals.css";
import { createIDBPersister } from "@tm9657/flow-like-ui/lib/persister";
import { ReactFlowProvider } from "@xyflow/react";
import { Inter } from "next/font/google";
import { Suspense } from "react";
import { AppSidebar } from "../components/app-sidebar";
import { TauriProvider } from "../components/tauri-provider";
import ToastProvider from "../components/toast-provider";
import PostHogPageView from "./PostHogPageView";
import { PHProvider } from "./provider";

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
			<PHProvider>
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
								<ToastProvider/>
								<TauriProvider>
									<Suspense
										fallback={
											<div className="flex flex-1 justify-center items-center">
												{"Loading..."}
											</div>
										}
									>
										<PostHogPageView />
										<ThemeProvider
											attribute="class"
											defaultTheme="system"
											enableSystem
											disableTransitionOnChange
										>
											<AppSidebar>{children}</AppSidebar>
										</ThemeProvider>
									</Suspense>
								</TauriProvider>
							</body>
						</TooltipProvider>
					</PersistQueryClientProvider>
				</ReactFlowProvider>
			</PHProvider>
		</html>
	);
}
