"use client";

import { useAuth } from "react-oidc-context";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

const AUTH_CHANNEL = "flow-like-auth";

export default function CallbackPage() {
	const auth = useAuth();
	const router = useRouter();

	useEffect(() => {
		if (auth.isAuthenticated) {
			// Broadcast auth success to other tabs
			try {
				const channel = new BroadcastChannel(AUTH_CHANNEL);
				channel.postMessage({ type: "AUTH_SUCCESS" });
				channel.close();
			} catch {
				// BroadcastChannel not supported, fallback will be handled by storage event
			}
			// Redirect to home page after successful authentication
			router.push("/");
		}
	}, [auth.isAuthenticated, router]);

	if (auth.isLoading) {
		return (
			<div className="flex h-screen items-center justify-center">
				<div className="text-center">
					<div className="mb-4 text-lg">Signing you in...</div>
					<div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent mx-auto" />
				</div>
			</div>
		);
	}

	if (auth.error) {
		return (
			<div className="flex h-screen items-center justify-center">
				<div className="text-center">
					<div className="mb-4 text-lg text-red-500">
						Authentication Error
					</div>
					<div className="text-sm text-muted-foreground">{auth.error.message}</div>
					<button
						type="button"
						onClick={() => router.push("/")}
						className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded"
					>
						Return Home
					</button>
				</div>
			</div>
		);
	}

	return (
		<div className="flex h-screen items-center justify-center">
			<div className="text-center">
				<div className="mb-4 text-lg">Processing authentication...</div>
			</div>
		</div>
	);
}
