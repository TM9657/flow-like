export function argString(args: Record<string, unknown>, key: string): string {
	const value = args[key];
	return typeof value === "string" ? value : "";
}

export function argBool(
	args: Record<string, unknown>,
	key: string,
): boolean | undefined {
	const value = args[key];
	if (typeof value === "boolean") return value;
	if (value === "true") return true;
	if (value === "false") return false;
	return undefined;
}

export function argObject(
	args: Record<string, unknown>,
	key: string,
): Record<string, unknown> | undefined {
	const value = args[key];
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: undefined;
}
