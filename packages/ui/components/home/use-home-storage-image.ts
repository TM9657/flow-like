"use client";

import { useQuery } from "@tanstack/react-query";
import {
	isExpiredAssetUrl,
	recoverStableAssetUrl,
	signedUrlExpiry,
} from "../../lib/stable-asset-url";
import { useBackend, useBackendReady } from "../../state/backend-state";
import { useHomeScope } from "./home-content/shared";

const REFRESH_MARGIN_MS = 5 * 60_000;
const MIN_REFRESH_MS = 30_000;

export interface HomeStorageImageState {
	src: string | undefined;
	isLoading: boolean;
	error: boolean;
	retry: () => void;
}

export function useHomeStorageImage(
	appId: string | undefined,
	path: string | undefined,
): HomeStorageImageState {
	const backend = useBackend();
	const ready = useBackendReady();
	const scope = useHomeScope();
	const enabled = Boolean(ready && appId && path);
	const query = useQuery({
		queryKey: ["home", ...scope, "storage-image-url", appId, path],
		enabled,
		queryFn: async ({ signal }) => {
			if (!appId || !path) throw new Error("Choose an app and image.");
			const results = await backend.storageState.downloadStorageItems(appId, [
				path,
			]);
			signal.throwIfAborted();
			const result = results.find((item) => item.prefix === path);
			if (!result?.url || result.error)
				throw new Error("The image download could not be authorized.");
			if (isExpiredAssetUrl(result.url)) {
				recoverStableAssetUrl(result.url);
				throw new Error("The image download URL has expired.");
			}
			return result.url;
		},
		staleTime: (query) => {
			if (!query.state.data) return 0;
			const expiry = signedUrlExpiry(query.state.data);
			return expiry
				? Math.max(0, expiry - query.state.dataUpdatedAt - REFRESH_MARGIN_MS)
				: Number.POSITIVE_INFINITY;
		},
		refetchInterval: (query) => {
			if (query.state.status === "error") return false;
			const expiry = signedUrlExpiry(query.state.data);
			return expiry
				? Math.max(MIN_REFRESH_MS, expiry - Date.now() - REFRESH_MARGIN_MS)
				: false;
		},
		refetchOnMount: "always",
		retry: false,
	});
	const expired = isExpiredAssetUrl(query.data);
	return {
		src: enabled && !query.isError && !expired ? query.data : undefined,
		isLoading: enabled && !query.isError && (!query.data || expired),
		error: enabled && query.isError,
		retry: () => {
			recoverStableAssetUrl(query.data);
			void query.refetch();
		},
	};
}
