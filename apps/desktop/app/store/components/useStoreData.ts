"use client";

import { openUrl as open } from "@tauri-apps/plugin-opener";
import {
	type IApp,
	IAppVisibility,
	type IMetadata,
	useBackend,
	useInvoke,
} from "@tm9657/flow-like-ui";
import type { AppRouterInstance } from "next/dist/shared/lib/app-router-context.shared-runtime";
import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";

export function useStoreData(
	id: string | undefined,
	router: AppRouterInstance,
) {
	const backend = useBackend();
	const [isPurchasing, setIsPurchasing] = useState(false);

	const apps = useInvoke(backend.appState.getApps, backend.appState, []);
	const app = useInvoke<IApp, [appId: string]>(
		backend.appState.getApp,
		backend.appState,
		[id!],
		!!id,
	);
	const meta = useInvoke<
		IMetadata,
		[appId: string, language?: string | undefined]
	>(backend.appState.getAppMeta, backend.appState, [id!], !!id);

	const isMember = useMemo(
		() => !!(id && apps.data?.some(([a]) => a.id === id)),
		[apps.data, id],
	);

	const formatPrice = useCallback((price?: number | null) => {
		if (!price || price <= 0) return "Free";
		return `€${(price / 100).toFixed(2)}`;
	}, []);

	const onUse = useCallback(() => {
		if (!id) return;
		router.push(`/use?id=${id}`);
	}, [id, router]);

	const onSettings = useCallback(() => {
		if (!id) return;
		router.push(`/library/config?id=${id}`);
	}, [id, router]);

	const onBuy = useCallback(async () => {
		if (!id || isPurchasing) return;

		setIsPurchasing(true);
		try {
			const result = await backend.appState.purchaseApp(id);

			if (result.alreadyMember) {
				toast.info("You already own this app!");
				await apps.refetch?.();
				router.push(`/use?id=${id}`);
				return;
			}

			if (result.checkoutUrl) {
				// Open checkout in system browser (desktop)
				await open(result.checkoutUrl);
				toast.info("Opening checkout in your browser...");
			} else {
				toast.error("Unable to start purchase. Please try again.");
			}
		} catch (e) {
			console.error("Purchase error:", e);
			toast.error("Failed to start purchase. Please try again later.");
		} finally {
			setIsPurchasing(false);
		}
	}, [id, isPurchasing, backend.appState, apps, router]);

	const onJoinOrRequest = useCallback(async () => {
		const data = app.data;
		if (!data || !id) return;
		try {
			if (data.price && data.price > 0) {
				await backend.appState.requestJoinApp(
					data.id,
					"Interested in trying out your app!",
				);
				toast.success(
					"Request to join app sent! The author will review your request.",
				);
				await apps.refetch?.();
				return;
			}

			if (data.visibility === IAppVisibility.PublicRequestAccess) {
				await backend.appState.requestJoinApp(
					data.id,
					"Interested in trying out your app!",
				);
				toast.success(
					"Request to join app sent! The author will review your request.",
				);
				await apps.refetch?.();
				return;
			}

			if (data.visibility !== IAppVisibility.Public) {
				toast.error(
					"You don't have access to this app. Please request access from the author.",
				);
				return;
			}

			await backend.appState.requestJoinApp(
				data.id,
				"Interested in trying out your app!",
			);
			toast.success("Joined app! You can now access it.");
			await apps.refetch?.();
			await router.push(`/use?id=${data.id}`);
		} catch (e) {
			toast.error("Failed to request to join app. Please try again later.");
		}
	}, [app.data, id, backend.appState, apps, router]);

	const coverUrl = meta.data?.thumbnail || "/placeholder-thumbnail.webp";
	const iconUrl = meta.data?.icon || "/app-logo.webp";
	const appName = meta.data?.name || app.data?.id || "App";
	const priceLabel = formatPrice(app.data?.price ?? null);

	return {
		apps,
		app,
		meta,
		isMember,
		isPurchasing,
		coverUrl,
		iconUrl,
		appName,
		priceLabel,
		onUse,
		onSettings,
		onBuy,
		onJoinOrRequest,
	} as const;
}
