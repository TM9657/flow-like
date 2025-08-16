import { getCollection } from "astro:content";
import rss from "@astrojs/rss";

export async function GET(context) {
	const posts = (
		await getCollection("blog", ({ data }) =>
			import.meta.env.PROD ? !data.draft : true,
		)
	).sort((a, b) => b.data.date - a.data.date);

	return rss({
		title: "Your Blog",
		description: "Newest posts",
		site: context.site, // set "site" in astro.config for absolute URLs
		items: posts.map((p) => ({
			title: p.data.title,
			description: p.data.description,
			link: "/" + p.slug.split("-").slice(3).join("-") + "/",
			pubDate: p.data.date,
		})),
	});
}
