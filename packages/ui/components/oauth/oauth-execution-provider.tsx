"use client";

import {
	type ReactNode,
	createContext,
	useCallback,
	useContext,
	useEffect,
	useRef,
	useState,
} from "react";
import type { IOAuthConsentStore } from "../../db/oauth-db";
import { checkOAuthTokens } from "../../lib/oauth/helpers";
import type { OAuthService } from "../../lib/oauth/service";
import type {
	IOAuthProvider,
	IOAuthRuntime,
	IOAuthToken,
	IOAuthTokenStoreWithPending,
	IStoredOAuthToken,
} from "../../lib/oauth/types";
import type { IBoard } from "../../lib/schema/flow/board";
import type { IHub } from "../../lib/schema/hub/hub";
import { DeviceFlowDialog } from "./device-flow-dialog";
import { OAuthConsentDialog } from "./oauth-consent-dialog";

export interface OAuthExecutionContextValue {
	withOAuthCheck: <T>(
		board: IBoard,
		executor: (tokens?: Record<string, IOAuthToken>) => Promise<T>,
	) => Promise<T>;
}

const OAuthExecutionContext = createContext<OAuthExecutionContextValue | null>(
	null,
);

export function useOAuthExecutionContext() {
	const context = useContext(OAuthExecutionContext);
	if (!context) {
		throw new Error(
			"useOAuthExecutionContext must be used within OAuthExecutionProvider",
		);
	}
	return context;
}

interface OAuthRequiredEvent extends CustomEvent {
	detail: {
		missingProviders: IOAuthProvider[];
		appId: string;
		boardId: string;
		nodeId: string;
		payload?: object;
	};
}

interface PendingExecution {
	appId: string;
	boardId: string;
	nodeId: string;
	payload?: object;
}

export interface OAuthExecutionProviderProps {
	children: ReactNode;
	oauthService: OAuthService;
	runtime: IOAuthRuntime;
	tokenStore: IOAuthTokenStoreWithPending;
	consentStore: IOAuthConsentStore;
	hub?: IHub;
	onOAuthCallback?: (providerId: string, token: IStoredOAuthToken) => void;
	providerCacheRef?: React.MutableRefObject<Map<string, IOAuthProvider>>;
}

export function OAuthExecutionProvider({
	children,
	oauthService,
	runtime,
	tokenStore,
	consentStore,
	hub,
	onOAuthCallback,
	providerCacheRef,
}: OAuthExecutionProviderProps) {
	const [missingProviders, setMissingProviders] = useState<IOAuthProvider[]>(
		[],
	);
	const [currentAppId, setCurrentAppId] = useState<string | null>(null);
	const [pendingExecution, setPendingExecution] =
		useState<PendingExecution | null>(null);
	const [isDialogOpen, setIsDialogOpen] = useState(false);
	const [authorizedProviders, setAuthorizedProviders] = useState<Set<string>>(
		new Set(),
	);
	const [preAuthorizedProviders, setPreAuthorizedProviders] = useState<
		Set<string>
	>(new Set());
	const [deviceFlowProvider, setDeviceFlowProvider] =
		useState<IOAuthProvider | null>(null);
	const [pendingAutoConsent, setPendingAutoConsent] = useState<
		IOAuthProvider[]
	>([]);

	const pendingExecutionRef = useRef<PendingExecution | null>(null);
	const currentAppIdRef = useRef<string | null>(null);
	const missingProvidersRef = useRef<IOAuthProvider[]>([]);

	useEffect(() => {
		pendingExecutionRef.current = pendingExecution;
	}, [pendingExecution]);

	useEffect(() => {
		currentAppIdRef.current = currentAppId;
	}, [currentAppId]);

	useEffect(() => {
		missingProvidersRef.current = missingProviders;
	}, [missingProviders]);

	// Auto-consent for providers the user has previously remembered
	useEffect(() => {
		if (pendingAutoConsent.length === 0) return;

		const processAutoConsent = async () => {
			const provider = pendingAutoConsent[0];
			const remainingProviders = pendingAutoConsent.slice(1);
			setPendingAutoConsent(remainingProviders);

			const existingToken = await tokenStore.getToken(provider.id);
			if (existingToken && !tokenStore.isExpired(existingToken)) {
				setAuthorizedProviders((prev) => {
					const next = new Set(prev);
					next.add(provider.id);
					return next;
				});
				return;
			}

			if (provider.use_device_flow && provider.device_auth_url) {
				setDeviceFlowProvider(provider);
			} else {
				oauthService.startAuthorization(provider);
			}
		};

		processAutoConsent();
	}, [pendingAutoConsent, tokenStore, oauthService]);

	// Populate provider cache when dialog opens
	useEffect(() => {
		if (isDialogOpen && missingProviders.length > 0 && providerCacheRef) {
			const cache = new Map<string, IOAuthProvider>();
			for (const provider of missingProviders) {
				cache.set(provider.id, provider);
			}
			providerCacheRef.current = cache;
			setAuthorizedProviders(new Set());
		}
		return () => {
			if (!isDialogOpen && providerCacheRef) {
				providerCacheRef.current = new Map();
			}
		};
	}, [isDialogOpen, missingProviders, providerCacheRef]);

	// Listen for OAuth required events
	useEffect(() => {
		const handleOAuthRequired = async (event: Event) => {
			const oauthEvent = event as OAuthRequiredEvent;
			const {
				appId,
				boardId,
				nodeId,
				payload,
				missingProviders: allMissing,
			} = oauthEvent.detail;
			setCurrentAppId(appId);
			setPendingExecution({ appId, boardId, nodeId, payload });

			const consentedIds = await consentStore.getConsentedProviderIds(appId);

			const autoConsentProviders: IOAuthProvider[] = [];
			const needsDialogProviders: IOAuthProvider[] = [];
			const hasTokenNeedsConsent: Set<string> = new Set();

			for (const provider of allMissing) {
				if (consentedIds.has(provider.id)) {
					autoConsentProviders.push(provider);
				} else {
					const existingToken = await tokenStore.getToken(provider.id);
					if (existingToken && !tokenStore.isExpired(existingToken)) {
						hasTokenNeedsConsent.add(provider.id);
						needsDialogProviders.push(provider);
					} else {
						needsDialogProviders.push(provider);
					}
				}
			}

			if (autoConsentProviders.length > 0) {
				setPendingAutoConsent(autoConsentProviders);
			}

			if (needsDialogProviders.length > 0) {
				setAuthorizedProviders(new Set());
				setPreAuthorizedProviders(hasTokenNeedsConsent);
				setMissingProviders(needsDialogProviders);
				setIsDialogOpen(true);
			}
		};

		window.addEventListener("flow:oauth-required", handleOAuthRequired);
		return () => {
			window.removeEventListener("flow:oauth-required", handleOAuthRequired);
		};
	}, [consentStore, tokenStore]);

	const handleAuthorize = useCallback(
		async (providerId: string) => {
			const provider = missingProvidersRef.current.find(
				(p) => p.id === providerId,
			);
			if (!provider) return;

			if (provider.use_device_flow && provider.device_auth_url) {
				setDeviceFlowProvider(provider);
				return;
			}

			await oauthService.startAuthorization(provider);
		},
		[oauthService],
	);

	const handleConfirmAll = useCallback(
		async (rememberConsent: boolean) => {
			const appId = currentAppIdRef.current;
			const providers = missingProvidersRef.current;
			const execution = pendingExecutionRef.current;

			if (rememberConsent && appId) {
				for (const provider of providers) {
					await consentStore.setConsent(appId, provider.id, provider.scopes);
				}
			}

			setIsDialogOpen(false);
			setMissingProviders([]);
			setAuthorizedProviders(new Set());
			setPreAuthorizedProviders(new Set());

			if (execution) {
				window.dispatchEvent(
					new CustomEvent("flow:oauth-retry", {
						detail: {
							...execution,
							skipConsentCheck: true,
						},
					}),
				);
				setPendingExecution(null);
			}
		},
		[consentStore],
	);

	const handleDeviceFlowSuccess = useCallback(
		(token: IStoredOAuthToken) => {
			setDeviceFlowProvider(null);
			setAuthorizedProviders((prev) => {
				const next = new Set(prev);
				next.add(token.providerId);
				return next;
			});
			onOAuthCallback?.(token.providerId, token);
		},
		[onOAuthCallback],
	);

	const handleDeviceFlowCancel = useCallback(() => {
		setDeviceFlowProvider(null);
	}, []);

	const handleCancel = useCallback(() => {
		setIsDialogOpen(false);
		setMissingProviders([]);
		setAuthorizedProviders(new Set());
		setPreAuthorizedProviders(new Set());
		setPendingExecution(null);
	}, []);

	const withOAuthCheck = useCallback(
		async <T,>(
			board: IBoard,
			executor: (tokens?: Record<string, IOAuthToken>) => Promise<T>,
		): Promise<T> => {
			const result = await checkOAuthTokens(board, tokenStore, hub);

			if (result.missingProviders.length === 0) {
				const tokens =
					Object.keys(result.tokens).length > 0 ? result.tokens : undefined;
				return executor(tokens);
			}

			setMissingProviders(result.missingProviders);
			setIsDialogOpen(true);

			throw new Error("OAuth authorization required");
		},
		[tokenStore, hub],
	);

	// Register callback handler for OAuth callbacks
	const handleOAuthCallbackInternal = useCallback(
		(providerId: string, token: IStoredOAuthToken) => {
			setAuthorizedProviders((prev) => {
				const next = new Set(prev);
				next.add(providerId);
				return next;
			});
			onOAuthCallback?.(providerId, token);
		},
		[onOAuthCallback],
	);

	return (
		<OAuthExecutionContext.Provider value={{ withOAuthCheck }}>
			{children}
			<OAuthConsentDialog
				open={isDialogOpen}
				onOpenChange={setIsDialogOpen}
				providers={missingProviders}
				onAuthorize={handleAuthorize}
				onConfirmAll={handleConfirmAll}
				onCancel={handleCancel}
				authorizedProviders={authorizedProviders}
				preAuthorizedProviders={preAuthorizedProviders}
			/>
			<DeviceFlowDialog
				provider={deviceFlowProvider}
				oauthService={oauthService}
				runtime={runtime}
				onSuccess={handleDeviceFlowSuccess}
				onCancel={handleDeviceFlowCancel}
			/>
		</OAuthExecutionContext.Provider>
	);
}
