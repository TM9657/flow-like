import type { Metadata, Viewport } from "next";
import "@tm9657/flow-like-ui/global.css";
import { Inter } from "next/font/google";
import { ClientProviders } from "../components/client-providers";

const inter = Inter({ subsets: ["latin"], preload: true });

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://app.flow-like.com";

export const metadata: Metadata = {
	title: {
		default: "App | Flow-Like",
		template: "%s | Flow-Like",
	},
	description: "Enterprise-grade workflow automation built for scale.",
	keywords: [
		"workflow automation",
		"enterprise automation",
		"orchestration",
		"flow-like",
		"AI automation",
	],
	authors: [{ name: "TM9657 GmbH" }],
	creator: "TM9657 GmbH",
	metadataBase: new URL(siteUrl),
	openGraph: {
		type: "website",
		locale: "en_US",
		url: siteUrl,
		siteName: "Flow-Like",
		title: "Flow-Like",
		description: "Enterprise-grade workflow automation built for scale.",
		images: [
			{
				url: "/og.png",
				width: 1600,
				height: 900,
				alt: "Flow-Like",
			},
		],
	},
	twitter: {
		card: "summary_large_image",
		site: "@greatco_de",
		creator: "@greatco_de",
		title: "Flow-Like | Enterprise Automation",
		description: "Enterprise-grade workflow automation built for scale.",
		images: ["/og.png"],
	},
	icons: {
		icon: [
			{ url: "/favicon.svg", type: "image/svg+xml" },
			{ url: "/favicon-32x32.png", sizes: "32x32", type: "image/png" },
			{ url: "/favicon-16x16.png", sizes: "16x16", type: "image/png" },
		],
		apple: [{ url: "/apple-touch-icon.png", sizes: "180x180" }],
	},
	manifest: "/site.webmanifest",
	robots: {
		index: true,
		follow: true,
	},
};

export const viewport: Viewport = {
	themeColor: "#ffffff",
	width: "device-width",
	initialScale: 1,
	maximumScale: 1,
	viewportFit: "cover",
};

export default function RootLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<html lang="en" suppressHydrationWarning suppressContentEditableWarning>
			<body className={inter.className}>
				<ClientProviders>{children}</ClientProviders>
			</body>
		</html>
	);
}
