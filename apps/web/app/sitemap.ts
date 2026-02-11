import type { MetadataRoute } from "next";

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://app.flow-like.com";

const publicRoutes = ["/", "/store", "/store/packages", "/store/explore/apps"];

export default function sitemap(): MetadataRoute.Sitemap {
	const lastModified = new Date();

	return publicRoutes.map((route) => ({
		url: `${siteUrl}${route}`,
		lastModified,
		changeFrequency: route === "/" ? "daily" : "weekly",
		priority: route === "/" ? 1 : 0.7,
	}));
}
