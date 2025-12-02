import type { IBoard, INode } from "../schema";
import type {
	IOAuthProvider,
	IOAuthToken,
	IOAuthTokenCheckResult,
	IOAuthTokenStore,
	IStoredOAuthToken,
} from "./types";

/**
 * Extract OAuth providers from board nodes (including layers).
 * Deduplicates providers by ID and merges required scopes from all nodes.
 */
export function extractOAuthProvidersFromBoard(
	board: IBoard,
): IOAuthProvider[] {
	const providersMap = new Map<string, IOAuthProvider>();
	const additionalScopesMap = new Map<string, Set<string>>();

	const processNode = (node: INode) => {
		const nodeProviders = (node as any).oauth_providers as
			| IOAuthProvider[]
			| undefined;
		if (nodeProviders && nodeProviders.length > 0) {
			console.log(
				`[OAuth] Node ${node.friendly_name} has providers:`,
				nodeProviders,
			);
			for (const provider of nodeProviders) {
				if (!providersMap.has(provider.id)) {
					providersMap.set(provider.id, { ...provider });
				}
			}
		}

		// Collect required_oauth_scopes from nodes
		const requiredScopes = (node as any).required_oauth_scopes as
			| Record<string, string[] | { values?: string[] }>
			| undefined;
		if (requiredScopes) {
			for (const [providerId, scopes] of Object.entries(requiredScopes)) {
				if (!additionalScopesMap.has(providerId)) {
					additionalScopesMap.set(providerId, new Set());
				}
				const scopeSet = additionalScopesMap.get(providerId)!;
				// Handle both array format and protobuf { values: [] } format
				const scopeArray = Array.isArray(scopes)
					? scopes
					: (scopes?.values ?? []);
				for (const scope of scopeArray) {
					scopeSet.add(scope);
				}
			}
		}
	};

	for (const node of Object.values(board.nodes)) {
		processNode(node);
	}
	for (const layer of Object.values(board.layers)) {
		for (const node of Object.values(layer.nodes)) {
			processNode(node);
		}
	}

	// Merge additional scopes into providers
	const providers: IOAuthProvider[] = [];
	for (const [providerId, provider] of providersMap) {
		const additionalScopes = additionalScopesMap.get(providerId);
		if (additionalScopes && additionalScopes.size > 0) {
			// Merge base scopes with additional scopes from nodes
			const allScopes = new Set([...provider.scopes, ...additionalScopes]);
			provider.merged_scopes = Array.from(allScopes);
			console.log(
				`[OAuth] Provider ${provider.name} merged scopes:`,
				provider.merged_scopes,
			);
		} else {
			provider.merged_scopes = [...provider.scopes];
		}
		providers.push(provider);
	}

	console.log(
		`[OAuth] Found ${providers.length} OAuth providers in board:`,
		providers,
	);
	return providers;
}

/**
 * Convert stored token to backend format.
 */
export function storedTokenToBackendFormat(
	token: IStoredOAuthToken,
): IOAuthToken {
	return {
		access_token: token.access_token,
		refresh_token: token.refresh_token,
		expires_at: token.expires_at
			? Math.floor(token.expires_at / 1000)
			: undefined,
		token_type: token.token_type ?? "Bearer",
	};
}

/**
 * Get OAuth tokens for required providers, checking validity.
 * Returns valid tokens and list of providers that need authorization.
 */
export async function getOAuthTokensForProviders(
	providers: IOAuthProvider[],
	tokenStore: IOAuthTokenStore,
): Promise<IOAuthTokenCheckResult> {
	const tokens: Record<string, IOAuthToken> = {};
	const missingProviders: IOAuthProvider[] = [];

	for (const provider of providers) {
		const storedToken = await tokenStore.getToken(provider.id);
		if (storedToken && !tokenStore.isExpired(storedToken)) {
			tokens[provider.id] = storedTokenToBackendFormat(storedToken);
		} else {
			missingProviders.push(provider);
		}
	}

	return { tokens, missingProviders };
}

/**
 * Check OAuth tokens and return result with missing providers.
 * Does NOT throw - caller decides how to handle missing providers.
 */
export async function checkOAuthTokens(
	board: IBoard,
	tokenStore: IOAuthTokenStore,
): Promise<IOAuthTokenCheckResult & { requiredProviders: IOAuthProvider[] }> {
	const requiredProviders = extractOAuthProvidersFromBoard(board);

	if (requiredProviders.length === 0) {
		return { tokens: {}, missingProviders: [], requiredProviders: [] };
	}

	const result = await getOAuthTokensForProviders(
		requiredProviders,
		tokenStore,
	);
	return { ...result, requiredProviders };
}

/**
 * Check if all required OAuth providers have valid tokens.
 * Throws an error if any are missing.
 */
export async function ensureOAuthTokens(
	board: IBoard,
	tokenStore: IOAuthTokenStore,
): Promise<Record<string, IOAuthToken> | undefined> {
	const requiredProviders = extractOAuthProvidersFromBoard(board);

	if (requiredProviders.length === 0) {
		return undefined;
	}

	const { tokens, missingProviders } = await getOAuthTokensForProviders(
		requiredProviders,
		tokenStore,
	);

	if (missingProviders.length > 0) {
		const missingNames = missingProviders.map((p) => p.name).join(", ");
		throw new Error(
			`Missing OAuth authorization for: ${missingNames}. Please authorize these services first.`,
		);
	}

	return tokens;
}
