"use client";
import {
	Architects_Daughter,
	DM_Sans,
	Fira_Code,
	Geist,
	Geist_Mono,
	IBM_Plex_Mono,
	IBM_Plex_Sans,
	Inter,
	JetBrains_Mono,
	Libre_Baskerville,
	Lora,
	Merriweather,
	Montserrat,
	Open_Sans,
	Outfit,
	Oxanium,
	Playfair_Display,
	Plus_Jakarta_Sans,
	Poppins,
	Roboto,
	Roboto_Mono,
	Source_Code_Pro,
	Source_Serif_4,
	Space_Grotesk,
	Space_Mono,
} from "next/font/google";
import "@tm9657/flow-like-ui/global.css";
import {
	PersistQueryClientProvider,
	QueryClient,
	ReactFlowProvider,
} from "@tm9657/flow-like-ui";
import { ThemeProvider } from "@tm9657/flow-like-ui/components/theme-provider";
import { Toaster } from "@tm9657/flow-like-ui/components/ui/sonner";
import { TooltipProvider } from "@tm9657/flow-like-ui/components/ui/tooltip";
import { createIDBPersister } from "@tm9657/flow-like-ui/lib/persister";
import { AppSidebar } from "../components/app-sidebar";
import { DesktopAuthProvider } from "../components/auth-provider";
import GlobalAnchorHandler from "../components/global-anchor-component";
import { TauriProvider } from "../components/tauri-provider";
import { ThemeLoader } from "../components/theme-loader";
import ToastProvider from "../components/toast-provider";
import PostHogPageView from "./PostHogPageView";
import { PHProvider } from "./provider";
import { UpdateProvider } from "../components/update-provider";

const persister = createIDBPersister();
const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			networkMode: "offlineFirst",
		},
	},
});

const inter = Inter({ subsets: ["latin"], preload: true });
const dmSans = DM_Sans({ subsets: ["latin"], preload: true });
const firaCode = Fira_Code({ subsets: ["latin"], preload: true });
const geist = Geist({ subsets: ["latin"], preload: true });
const geistMono = Geist_Mono({ subsets: ["latin"], preload: true });
const ibmPlexMono = IBM_Plex_Mono({
	subsets: ["latin"],
	weight: ["100", "200", "300", "400", "500", "600", "700"],
	preload: true,
});
const ibmPlexSans = IBM_Plex_Sans({
	subsets: ["latin"],
	weight: ["100", "200", "300", "400", "500", "600", "700"],
	preload: true,
});
const jetBrainsMono = JetBrains_Mono({ subsets: ["latin"], preload: true });
const libreBaskerville = Libre_Baskerville({
	subsets: ["latin"],
	weight: ["400", "700"],
	preload: true,
});
const lora = Lora({ subsets: ["latin"], preload: true });
const merriweather = Merriweather({ subsets: ["latin"], preload: true });
const montserrat = Montserrat({ subsets: ["latin"], preload: true });
const openSans = Open_Sans({ subsets: ["latin"], preload: true });
const outfit = Outfit({ subsets: ["latin"], preload: true });
const oxanium = Oxanium({ subsets: ["latin"], preload: true });
const playfairDisplay = Playfair_Display({ subsets: ["latin"], preload: true });
const plusJakartaSans = Plus_Jakarta_Sans({
	subsets: ["latin"],
	preload: true,
});
const poppins = Poppins({
	subsets: ["latin"],
	weight: ["100", "200", "300", "400", "500", "600", "700", "800", "900"],
	preload: true,
});
const roboto = Roboto({
	subsets: ["latin"],
	weight: ["100", "300", "400", "500", "700", "900"],
	preload: true,
});
const robotoMono = Roboto_Mono({ subsets: ["latin"], preload: true });
const sourceCodePro = Source_Code_Pro({ subsets: ["latin"], preload: true });
const sourceSerif4 = Source_Serif_4({ subsets: ["latin"], preload: true });
const spaceGrotesk = Space_Grotesk({ subsets: ["latin"], preload: true });
const spaceMono = Space_Mono({
	subsets: ["latin"],
	weight: ["400", "700"],
	preload: true,
});
const architectsDaughter = Architects_Daughter({
	subsets: ["latin"],
	weight: ["400"],
	preload: true,
});

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
						<body className={inter.className}>
							<UpdateProvider/>
							<GlobalAnchorHandler />
							<ThemeProvider
								attribute="class"
								defaultTheme="system"
								enableSystem
								storageKey="theme"
								disableTransitionOnChange
							>
								<TooltipProvider>
									<Toaster />
									<ToastProvider />
									<TauriProvider>
										<DesktopAuthProvider>
											<PostHogPageView />
											<ThemeLoader />
											<AppSidebar>{children}</AppSidebar>
										</DesktopAuthProvider>
									</TauriProvider>
								</TooltipProvider>
							</ThemeProvider>
						</body>
					</PersistQueryClientProvider>
				</ReactFlowProvider>
			</PHProvider>
		</html>
	);
}
