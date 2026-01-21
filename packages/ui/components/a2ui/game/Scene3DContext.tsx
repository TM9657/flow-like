"use client";

import { createContext, useContext } from "react";

// Context to indicate we're inside a Scene3D Canvas
export const Scene3DContext = createContext<boolean>(false);

export function useIsInsideScene3D(): boolean {
	return useContext(Scene3DContext);
}

export function Scene3DProvider({ children }: { children: React.ReactNode }) {
	return (
		<Scene3DContext.Provider value={true}>{children}</Scene3DContext.Provider>
	);
}
