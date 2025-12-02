"use client";

import {
	type IBoard,
	type IOAuthProvider,
	type IOAuthToken,
	type IStoredOAuthToken,
	OAuthConsentDialog,
	checkOAuthTokens,
} from "@tm9657/flow-like-ui";
import {
	type ReactNode,
	createContext,
	useCallback,
	useContext,
	useEffect,
	useRef,
	useState,
} from "react";
import { oauthConsentStore, oauthTokenStore } from "../lib/oauth-db";
import { oauthService } from "../lib/oauth-service";
import { DeviceFlowDialog } from "./device-flow-dialog";
import {
	clearProviderCache,
	setProviderCache,
	useOAuthCallbackListener,
} from "./oauth-callback-handler";

interface OAuthExecutionContextValue {
	/** Execute a function with OAuth check - shows consent dialog if needed */
	withOAuthCheck: <T>(
		board: IBoard,
		executor: (tokens?: Record<string, IOAuthToken>) => Promise<T>,
	) => Promise<T>;
}

const OAuthExecutionContext = createContext<OAuthExecutionContextValue | null>(
	null,
);

export function useOAuthExecution() {
	const context = useContext(OAuthExecutionContext);
	if (!context) {
		throw new Error(
			"useOAuthExecution must be used within OAuthExecutionProvider",
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

export function OAuthExecutionProvider({ children }: { children: ReactNode }) {
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

	// Refs to avoid stale closures
	const pendingExecutionRef = useRef<PendingExecution | null>(null);
	const currentAppIdRef = useRef<string | null>(null);
	const missingProvidersRef = useRef<IOAuthProvider[]>([]);

	// Keep refs in sync with state
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

			// Check if we already have a valid token for this provider
			const existingToken = await oauthTokenStore.getToken(provider.id);
			if (existingToken && !oauthTokenStore.isExpired(existingToken)) {
				// Token exists - no need to re-authorize, just mark provider as ready
				console.log(
					`[OAuthProvider] Auto-consent: Provider ${provider.id} already has valid token`,
				);
				setAuthorizedProviders((prev) => {
					const next = new Set(prev);
					next.add(provider.id);
					return next;
				});
				return;
			}

			// No valid token - start authorization for this provider
			if (provider.use_device_flow && provider.device_auth_url) {
				setDeviceFlowProvider(provider);
			} else {
				oauthService.startAuthorization(provider);
			}
		};

		processAutoConsent();
	}, [pendingAutoConsent]);

	// Populate provider cache when dialog opens with missing providers
	useEffect(() => {
		if (isDialogOpen && missingProviders.length > 0) {
			const cache = new Map<string, IOAuthProvider>();
			for (const provider of missingProviders) {
				cache.set(provider.id, provider);
			}
			setProviderCache(cache);
			setAuthorizedProviders(new Set());
		}
		return () => {
			if (!isDialogOpen) {
				clearProviderCache();
			}
		};
	}, [isDialogOpen, missingProviders]);

	// Listen for OAuth callbacks to mark providers as authorized
	useOAuthCallbackListener((pending, _token) => {
		setAuthorizedProviders((prev) => {
			const next = new Set(prev);
			next.add(pending.providerId);
			return next;
		});
	}, []);

	// Listen for OAuth required events from flow-board
	useEffect(() => {
		const handleOAuthRequired = async (event: Event) => {
			const oauthEvent = event as OAuthRequiredEvent;
			console.log("[OAuthProvider] Event received:", oauthEvent.detail);
			console.log(
				"[OAuthProvider] Missing providers from event:",
				oauthEvent.detail.missingProviders,
			);

			const {
				appId,
				boardId,
				nodeId,
				payload,
				missingProviders: allMissing,
			} = oauthEvent.detail;
			setCurrentAppId(appId);
			setPendingExecution({ appId, boardId, nodeId, payload });

			const consentedIds =
				await oauthConsentStore.getConsentedProviderIds(appId);
			console.log(
				"[OAuthProvider] Consented provider IDs for app:",
				appId,
				consentedIds,
			);

			// Separate providers into categories
			const autoConsentProviders: IOAuthProvider[] = [];
			const needsDialogProviders: IOAuthProvider[] = [];
			const hasTokenNeedsConsent: Set<string> = new Set();

			for (const provider of allMissing) {
				if (consentedIds.has(provider.id)) {
					console.log(
						"[OAuthProvider] Provider has consent, will auto-authorize:",
						provider.id,
					);
					autoConsentProviders.push(provider);
				} else {
					// Check if provider already has a valid token (just needs consent for this app)
					const existingToken = await oauthTokenStore.getToken(provider.id);
					if (existingToken && !oauthTokenStore.isExpired(existingToken)) {
						// Provider has valid token but needs consent for this app
						console.log(
							"[OAuthProvider] Provider has valid token, needs consent for this app:",
							provider.id,
						);
						hasTokenNeedsConsent.add(provider.id);
						needsDialogProviders.push(provider);
					} else {
						console.log(
							"[OAuthProvider] Provider needs dialog (no token):",
							provider.id,
						);
						needsDialogProviders.push(provider);
					}
				}
			}

			// Auto-start authorization for remembered providers (they already have tokens)
			if (autoConsentProviders.length > 0) {
				setPendingAutoConsent(autoConsentProviders);
			}

			// Show dialog for providers that need consent (with or without existing token)
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
	}, []);

	const handleAuthorize = useCallback(async (providerId: string) => {
		console.log("[OAuthProvider] handleAuthorize called for:", providerId);
		const provider = missingProvidersRef.current.find(
			(p) => p.id === providerId,
		);
		if (!provider) {
			console.log("[OAuthProvider] Provider not found in missingProviders");
			return;
		}

		console.log(
			`[OAuthProvider] Provider ${providerId} needs new token, starting auth flow`,
		);
		// Check if provider uses device flow
		if (provider.use_device_flow && provider.device_auth_url) {
			setDeviceFlowProvider(provider);
			return;
		}

		// Standard authorization code flow
		await oauthService.startAuthorization(provider);
	}, []);

	const handleConfirmAll = useCallback(async (rememberConsent: boolean) => {
		const appId = currentAppIdRef.current;
		const providers = missingProvidersRef.current;
		const execution = pendingExecutionRef.current;

		console.log(
			"[OAuthProvider] handleConfirmAll called, rememberConsent:",
			rememberConsent,
		);
		console.log(
			"[OAuthProvider] Current state - appId:",
			appId,
			"providers:",
			providers.length,
			"execution:",
			execution,
		);

		// Save consent for all providers if user wants to remember for future runs
		if (rememberConsent && appId) {
			for (const provider of providers) {
				console.log(
					"[OAuthProvider] Saving consent for provider:",
					provider.id,
				);
				await oauthConsentStore.setConsent(appId, provider.id, provider.scopes);
			}
		}

		setIsDialogOpen(false);
		setMissingProviders([]);
		setAuthorizedProviders(new Set());
		setPreAuthorizedProviders(new Set());

		// Dispatch retry event to re-execute the flow
		// Include skipConsentCheck flag since user just explicitly consented in dialog
		if (execution) {
			console.log("[OAuthProvider] Dispatching retry event for:", execution);
			window.dispatchEvent(
				new CustomEvent("flow:oauth-retry", {
					detail: {
						...execution,
						skipConsentCheck: true,
					},
				}),
			);
			setPendingExecution(null);
		} else {
			console.warn("[OAuthProvider] No pending execution to retry!");
		}
	}, []);

	const handleDeviceFlowSuccess = useCallback((token: IStoredOAuthToken) => {
		setDeviceFlowProvider(null);
		setAuthorizedProviders((prev) => {
			const next = new Set(prev);
			next.add(token.providerId);
			return next;
		});
	}, []);

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
			// Check for OAuth requirements
			const result = await checkOAuthTokens(board, oauthTokenStore);

			if (result.missingProviders.length === 0) {
				// All tokens available, execute directly
				const tokens =
					Object.keys(result.tokens).length > 0 ? result.tokens : undefined;
				return executor(tokens);
			}

			// Missing providers - show consent dialog
			setMissingProviders(result.missingProviders);
			setIsDialogOpen(true);

			throw new Error("OAuth authorization required");
		},
		[],
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
			{deviceFlowProvider && (
				<DeviceFlowDialog
					provider={deviceFlowProvider}
					onSuccess={handleDeviceFlowSuccess}
					onCancel={handleDeviceFlowCancel}
				/>
			)}
		</OAuthExecutionContext.Provider>
	);
}
