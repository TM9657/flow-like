/** Script URLs execute in the current document instead of opening a destination. */
export function isScriptNavigationUrl(value: string): boolean {
	try {
		const protocol = new URL(value, "https://navigation.invalid/").protocol;
		return protocol === "javascript:" || protocol === "vbscript:";
	} catch {
		return false;
	}
}
