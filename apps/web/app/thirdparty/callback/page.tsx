"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useEffect, useState } from "react";

function ThirdpartyCallbackContent() {
	const searchParams = useSearchParams();
	const router = useRouter();
	const [error, setError] = useState<string | null>(null);
	const [processing, setProcessing] = useState(true);

	useEffect(() => {
		const handleCallback = async () => {
			try {
				const code = searchParams.get("code");
				const state = searchParams.get("state");
				const errorParam = searchParams.get("error");
				const errorDescription = searchParams.get("error_description");

				// Also check for implicit flow tokens in hash (handled by Next.js differently)
				const accessToken = searchParams.get("access_token");
				const idToken = searchParams.get("id_token");

				if (errorParam) {
					setError(
						`Authorization failed: ${errorDescription || errorParam}`,
					);
					setProcessing(false);
					return;
				}

				if (!state) {
					setError("Missing state parameter in OAuth callback");
					setProcessing(false);
					return;
				}

				// Dispatch a custom event with the OAuth callback data
				// This will be picked up by the OAuth callback handler
				const callbackEvent = new CustomEvent("thirdparty-oauth-callback", {
					detail: {
						url: window.location.href,
						code,
						state,
						access_token: accessToken,
						id_token: idToken,
						token_type: searchParams.get("token_type"),
						expires_in: searchParams.get("expires_in"),
						scope: searchParams.get("scope"),
					},
				});
				window.dispatchEvent(callbackEvent);

				// Also store in sessionStorage for components that mount after redirect
				sessionStorage.setItem(
					"oauth-callback-pending",
					JSON.stringify({
						url: window.location.href,
						code,
						state,
						access_token: accessToken,
						id_token: idToken,
						token_type: searchParams.get("token_type"),
						expires_in: searchParams.get("expires_in"),
						scope: searchParams.get("scope"),
						timestamp: Date.now(),
					}),
				);

				// Redirect back to the flow page after a short delay
				// The OAuth handler will pick up the pending callback from sessionStorage
				setTimeout(() => {
					router.push("/flow");
				}, 100);
			} catch (err) {
				setError(
					`Failed to process callback: ${err instanceof Error ? err.message : String(err)}`,
				);
				setProcessing(false);
			}
		};

		handleCallback();
	}, [searchParams, router]);

	if (error) {
		return (
			<div className="flex h-screen items-center justify-center">
				<div className="text-center">
					<div className="mb-4 text-lg text-destructive">
						Authentication Error
					</div>
					<div className="text-sm text-muted-foreground">{error}</div>
					<button
						type="button"
						onClick={() => router.push("/flow")}
						className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded"
					>
						Return to Flow
					</button>
				</div>
			</div>
		);
	}

	if (processing) {
		return (
			<div className="flex h-screen items-center justify-center">
				<div className="text-center">
					<div className="mb-4 text-lg">Processing authentication...</div>
					<div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent mx-auto" />
				</div>
			</div>
		);
	}

	return null;
}

export default function ThirdpartyCallbackPage() {
	return (
		<Suspense
			fallback={
				<div className="flex h-screen items-center justify-center">
					<div className="text-center">
						<div className="mb-4 text-lg">Loading...</div>
						<div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent mx-auto" />
					</div>
				</div>
			}
		>
			<ThirdpartyCallbackContent />
		</Suspense>
	);
}
