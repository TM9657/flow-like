import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { AuthContext, useAuth } from "react-oidc-context";
import { Toaster } from "sonner";
import { WebUserState } from "../../apps/web/lib/web-states/user-state";
import { HomePage } from "../../packages/ui/components/home/home-page";
import type { IHomeLayout } from "../../packages/ui/components/home/types";
import { TooltipProvider } from "../../packages/ui/components/ui/tooltip";
import { useAssistantSurface } from "../../packages/ui/state/assistant-surface";
import {
	useBackend,
	useBackendStore,
} from "../../packages/ui/state/backend-state";
import "../../packages/ui/global.css";

const client = new QueryClient({
	defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});

function SaveIntegrationFixture() {
	const original = useRef(useBackend());
	const auth = useAuth();
	const [ready, setReady] = useState(false);
	useEffect(() => {
		const profile = {
			id: "save-integration-profile",
			name: "Save integration profile",
			hub: "https://api.flow-like.com",
			bits: [],
			apps: [],
		};
		useBackendStore.getState().setBackend({
			...original.current,
			profile,
			userState: new WebUserState({ auth, profile, queryClient: client }),
		});
		Object.assign(window, {
			saveIntegrationQa: {
				snapshot: () =>
					useAssistantSurface.getState().homeSurface?.getSnapshot(),
				stage: (layout: IHomeLayout) => {
					const surface = useAssistantSurface.getState().homeSurface;
					if (!surface) throw new Error("Home surface unavailable");
					const snapshot = surface.getSnapshot();
					return surface.stageLayout(layout, {
						expectedProfileId: snapshot.profileId,
						expectedFingerprint: snapshot.candidateFingerprint,
					});
				},
				refresh: () => client.invalidateQueries({ queryKey: ["getProfile"] }),
			},
		});
		setReady(true);
	}, [auth]);
	return (
		<div className="flex h-screen min-h-0 flex-col bg-background text-foreground">
			{ready && <HomePage />}
			<Toaster />
		</div>
	);
}

const root = document.getElementById("root");
if (!root) throw new Error("Home save fixture root is missing");
createRoot(root).render(
	<AuthContext.Provider
		value={
			{
				isAuthenticated: true,
				isLoading: false,
				user: {
					access_token: "fixture-only",
					profile: { sub: "save-integration-user" },
				},
			} as never
		}
	>
		<QueryClientProvider client={client}>
			<TooltipProvider>
				<SaveIntegrationFixture />
			</TooltipProvider>
		</QueryClientProvider>
	</AuthContext.Provider>,
);
