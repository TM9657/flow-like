"use client";

import { AppearanceStudio } from "../../../settings/appearance/appearance-studio";

/**
 * The app-wide stylesheet, opened as an editor document.
 *
 * Both entry points — the Appearance section under app config and the styles tab in the
 * flow editor — render the same studio; only the chrome around it differs.
 */
export function AppStylesDocument({
	appId,
	className,
}: Readonly<{
	appId: string;
	className?: string;
}>) {
	return <AppearanceStudio appId={appId} className={className} />;
}
