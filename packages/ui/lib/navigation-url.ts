/** Script and data URLs can contain executable documents. */
export function isScriptNavigationUrl(value: string): boolean {
	try {
		const protocol = new URL(value, "https://navigation.invalid/").protocol;
		return (
			protocol === "javascript:" ||
			protocol === "vbscript:" ||
			protocol === "data:"
		);
	} catch {
		return false;
	}
}
