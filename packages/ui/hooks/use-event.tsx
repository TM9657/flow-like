"use client";

import { listen } from "@tauri-apps/api/event";
import { useState, useEffect, useCallback } from "react";

export function useEvent(event: string, fn: (event: any) => void, deps: any[] = []) {
    const [unlisten, setUnlisten] = useState<(() => void) | null>(null);

    const init = useCallback(async () => {
        if (unlisten) {
            unlisten();  // Clean up the previous listener
        }
        const newUnlisten = await listen(event, fn);
        setUnlisten(() => newUnlisten);
    }, [event, fn]);  // Include `event` and `fn` in the dependencies of `init`

    useEffect(() => {
        init();

        return () => {
            if (unlisten) {
                unlisten();  // Clean up the listener when deps change or component unmounts
            }
        };
    }, deps);  // The effect depends on `deps`, same as before

    // Optionally, return a cleanup function or listener status if needed
    return { unlisten };
}
