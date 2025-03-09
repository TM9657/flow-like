"use client";

import { ThemeProvider as NextThemesProvider } from "next-themes";
import type { ThemeProviderProps } from "next-themes";
import * as React from "react";

export function ThemeProvider({
	children,
	...props
}: Readonly<ThemeProviderProps>) {
	return <NextThemesProvider {...props}>{children}</NextThemesProvider>;
}
