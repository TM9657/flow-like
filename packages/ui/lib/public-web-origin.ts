/** The Hub's public app address, shared by native handoff and copied links. */
export function publicWebOrigin(hubApp: unknown): string | undefined {
	if (typeof hubApp !== "string" || !hubApp.trim()) return;
	try {
		const url = new URL(hubApp.includes("://") ? hubApp : `https://${hubApp}`);
		if (
			url.protocol !== "https:" ||
			url.username ||
			url.password ||
			url.search ||
			url.hash ||
			url.pathname !== "/"
		)
			return;
		return url.origin;
	} catch {
		return;
	}
}

export async function resolveHubWebOrigin(
	apiOrigin: string,
	fetchHub: (url: string) => Promise<Response> = fetch,
): Promise<string> {
	if (apiOrigin.replace(/\/+$/, "") === "https://api.flow-like.com")
		return "https://app.flow-like.com";
	const response = await fetchHub(`${apiOrigin.replace(/\/+$/, "")}/api/v1`);
	if (!response.ok)
		throw new Error(
			"The Hub's public app address is unavailable. Try again when connected.",
		);
	const hub: unknown = await response.json();
	const origin = publicWebOrigin(
		hub && typeof hub === "object" && "app" in hub ? hub.app : undefined,
	);
	if (!origin)
		throw new Error(
			"This Hub has no public HTTPS app address configured for sharing predictions.",
		);
	return origin;
}
