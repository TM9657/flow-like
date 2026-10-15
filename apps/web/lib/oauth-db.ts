import {
	configureOAuthSession,
	useBackendStore,
} from "@flow-like/flow-like-ui";
import type { AuthContextProps } from "react-oidc-context";

configureOAuthSession(() => {
	const backend = useBackendStore.getState().backend as {
		profile?: { hub?: string };
		auth?: AuthContextProps;
	} | null;
	const user = backend?.auth?.user;
	const hub = backend?.profile?.hub;
	if (!hub || !user?.profile.sub) return undefined;
	return { hub, subject: user.profile.sub, issuer: user.profile.iss };
});

export {
	oauthDB,
	oauthConsentStore,
	oauthTokenStore,
} from "@flow-like/flow-like-ui";
