"use client";

import { useCallback, useContext, useEffect, useState } from "react";
import { AuthContext } from "react-oidc-context";
import { getApiOrigin } from "../../../../lib/api-url";
import { useBackend, useBackendReady } from "../../../../state/backend-state";
import type { IQueryState } from "../../../../state/backend-state/query-state";

type Resource<T> = {
	identity: string;
	owner: IQueryState;
	revision: number;
	data?: T;
	error?: unknown;
	status: "loading" | "success" | "error";
};

function useForecastResource<T>(
	appId: string,
	resourceId: string,
	userScoped: boolean,
	load: (query: IQueryState) => Promise<T>,
) {
	const backend = useBackend();
	const ready = useBackendReady();
	const auth = useContext(AuthContext);
	const [revision, setRevision] = useState(0);
	const [resource, setResource] = useState<Resource<T>>();
	const identity = JSON.stringify([
		getApiOrigin(backend.profile),
		backend.profile?.id ?? "",
		auth?.isAuthenticated ? auth.user?.profile.sub : "local",
		appId,
		userScoped,
		resourceId,
	]);
	const enabled = ready && !auth?.isLoading && !!appId && !!resourceId;
	const refetch = useCallback(() => setRevision((value) => value + 1), []);
	useEffect(() => {
		if (!enabled) {
			setResource(undefined);
			return;
		}
		let active = true;
		const owner = backend.queryState;
		const request = { identity, owner, revision };
		setResource({ ...request, status: "loading" });
		void Promise.resolve()
			.then(() => {
				if ([appId, resourceId].some((id) => id === "." || id === ".."))
					throw new Error("Choose a valid app and saved prediction.");
				return load(owner);
			})
			.then(
				(data) => {
					if (active) setResource({ ...request, status: "success", data });
				},
				(error: unknown) => {
					if (active) setResource({ ...request, status: "error", error });
				},
			);
		return () => {
			active = false;
		};
	}, [
		appId,
		resourceId,
		backend.queryState,
		enabled,
		identity,
		load,
		revision,
	]);
	useEffect(() => {
		if (!enabled) return;
		window.addEventListener("focus", refetch);
		const saved = (event: Event) => {
			const detail = (event as CustomEvent).detail;
			if (detail?.appId === appId && detail.userScoped === userScoped)
				refetch();
		};
		window.addEventListener("flow-like:forecast-saved", saved);
		return () => {
			window.removeEventListener("focus", refetch);
			window.removeEventListener("flow-like:forecast-saved", saved);
		};
	}, [enabled, refetch, appId, userScoped]);
	// Keep snapshots out of the persisted query cache, and never show another
	// session's data between rendering a new identity and starting its request.
	const current =
		enabled &&
		resource?.identity === identity &&
		resource.owner === backend.queryState &&
		resource.revision === revision
			? resource
			: undefined;
	return {
		data: current?.status === "success" ? current.data : undefined,
		isLoading:
			!!appId && !!resourceId && (!current || current.status === "loading"),
		isError: current?.status === "error",
		error: current?.error,
		refetch,
	};
}

export function useSavedForecast(
	appId: string,
	forecastId: string,
	userScoped = false,
) {
	const load = useCallback(
		async (query: IQueryState) => {
			const saved = await query.getForecast(appId, forecastId, userScoped);
			if (saved.app_id !== appId || saved.id !== forecastId)
				throw new Error(
					"The saved prediction does not match the requested app and prediction.",
				);
			return saved;
		},
		[appId, forecastId, userScoped],
	);
	return useForecastResource(appId, forecastId, userScoped, load);
}

export function useSavedForecasts(appId: string, userScoped = false) {
	const load = useCallback(
		async (query: IQueryState) => {
			const saved = await query.listForecasts(appId, userScoped);
			if (saved.some((item) => item.app_id !== appId))
				throw new Error(
					"The saved predictions do not match the requested app.",
				);
			return saved;
		},
		[appId, userScoped],
	);
	return useForecastResource(appId, "list", userScoped, load);
}
