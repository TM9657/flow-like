"use client";

import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@tm9657/flow-like-ui";
import { Button } from "@tm9657/flow-like-ui";
import type {
	IDeviceAuthResponse,
	IOAuthProvider,
	IStoredOAuthToken,
} from "@tm9657/flow-like-ui";
import { CheckCircle2, Copy, ExternalLink, Loader2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { oauthService } from "../lib/oauth-service";

interface DeviceFlowDialogProps {
	provider: IOAuthProvider | null;
	onSuccess: (token: IStoredOAuthToken) => void;
	onCancel: () => void;
}

type DeviceFlowState =
	| { status: "idle" }
	| { status: "loading" }
	| { status: "awaiting_user"; deviceAuth: IDeviceAuthResponse }
	| { status: "polling"; deviceAuth: IDeviceAuthResponse }
	| { status: "success" }
	| { status: "error"; message: string };

export function DeviceFlowDialog({
	provider,
	onSuccess,
	onCancel,
}: DeviceFlowDialogProps) {
	const [state, setState] = useState<DeviceFlowState>({ status: "idle" });
	const [copied, setCopied] = useState(false);
	const abortRef = useRef(false);

	useEffect(() => {
		if (!provider) {
			setState({ status: "idle" });
			return;
		}

		abortRef.current = false;
		startDeviceFlow();

		return () => {
			abortRef.current = true;
		};
	}, [provider]);

	async function startDeviceFlow() {
		if (!provider) return;

		setState({ status: "loading" });

		try {
			const deviceAuth = await oauthService.startDeviceAuthorization(provider);
			console.log("[DeviceFlow] Got device auth:", deviceAuth);

			if (abortRef.current) return;

			setState({ status: "awaiting_user", deviceAuth });
		} catch (e) {
			console.error("[DeviceFlow] Failed to start:", e);
			setState({
				status: "error",
				message:
					e instanceof Error
						? e.message
						: "Failed to start device authorization",
			});
		}
	}

	async function startPolling(deviceAuth: IDeviceAuthResponse) {
		if (!provider) return;

		setState({ status: "polling", deviceAuth });

		const scopes = [...provider.scopes];
		const timeoutMs = deviceAuth.expires_in * 1000;
		const startTime = Date.now();
		let interval = (deviceAuth.interval || 5) * 1000;

		while (Date.now() - startTime < timeoutMs) {
			if (abortRef.current) return;

			await new Promise((resolve) => setTimeout(resolve, interval));

			if (abortRef.current) return;

			try {
				const token = await oauthService.pollDeviceAuthorization(
					provider,
					deviceAuth.device_code,
					scopes,
				);

				if (token) {
					setState({ status: "success" });
					toast.success(`Connected to ${provider.name}`);

					// Short delay to show success state
					await new Promise((resolve) => setTimeout(resolve, 500));
					onSuccess(token);
					return;
				}
			} catch (e) {
				if (e instanceof Error) {
					if (e.message.includes("slow_down")) {
						interval += 5000;
						continue;
					}
					if (e.message.includes("expired")) {
						setState({
							status: "error",
							message: "Authorization expired. Please try again.",
						});
						return;
					}
					if (e.message.includes("access_denied")) {
						setState({ status: "error", message: "Authorization denied." });
						return;
					}
				}
				console.error("[DeviceFlow] Poll error:", e);
			}
		}

		setState({
			status: "error",
			message: "Authorization timed out. Please try again.",
		});
	}

	async function copyCode(code: string) {
		try {
			await navigator.clipboard.writeText(code);
			setCopied(true);
			toast.success("Code copied to clipboard");
			setTimeout(() => setCopied(false), 2000);
		} catch {
			toast.error("Failed to copy code");
		}
	}

	async function openVerificationUrl(url: string) {
		try {
			const { open } = await import("@tauri-apps/plugin-shell");
			await open(url);
		} catch {
			window.open(url, "_blank");
		}
	}

	function handleOpenAndPoll() {
		if (state.status !== "awaiting_user") return;

		const { deviceAuth } = state;
		const url =
			deviceAuth.verification_uri_complete || deviceAuth.verification_uri;
		openVerificationUrl(url);
		startPolling(deviceAuth);
	}

	const isOpen = provider !== null;

	return (
		<Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle>Connect to {provider?.name || "Service"}</DialogTitle>
					<DialogDescription>
						{state.status === "loading" && "Starting authorization..."}
						{state.status === "awaiting_user" &&
							"Enter the code below to authorize access"}
						{state.status === "polling" &&
							"Enter the code below in your browser"}
						{state.status === "success" && "Successfully connected!"}
						{state.status === "error" && "Authorization failed"}
					</DialogDescription>
				</DialogHeader>

				<div className="flex flex-col items-center gap-4 py-4">
					{state.status === "loading" && (
						<Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
					)}

					{state.status === "awaiting_user" && (
						<>
							<div className="flex flex-col items-center gap-2">
								<p className="text-sm text-muted-foreground">Your code:</p>
								<button
									type="button"
									onClick={() => copyCode(state.deviceAuth.user_code)}
									className="group flex items-center gap-2 rounded-lg border-2 border-dashed border-primary/50 bg-muted/50 px-6 py-3 font-mono text-2xl font-bold tracking-widest transition-colors hover:border-primary hover:bg-muted"
								>
									{state.deviceAuth.user_code}
									{copied ? (
										<CheckCircle2 className="h-5 w-5 text-green-500" />
									) : (
										<Copy className="h-5 w-5 opacity-50 group-hover:opacity-100" />
									)}
								</button>
							</div>

							<p className="text-center text-sm text-muted-foreground">
								Click the button below to open {provider?.name} and enter this
								code
							</p>

							<Button onClick={handleOpenAndPoll} className="gap-2">
								<ExternalLink className="h-4 w-4" />
								Open {provider?.name} & Authorize
							</Button>
						</>
					)}

					{state.status === "polling" && (
						<div className="flex flex-col items-center gap-4">
							<div className="flex flex-col items-center gap-2">
								<p className="text-sm text-muted-foreground">Your code:</p>
								<button
									type="button"
									onClick={() => copyCode(state.deviceAuth.user_code)}
									className="group flex items-center gap-2 rounded-lg border-2 border-dashed border-primary/50 bg-muted/50 px-6 py-3 font-mono text-2xl font-bold tracking-widest transition-colors hover:border-primary hover:bg-muted"
								>
									{state.deviceAuth.user_code}
									{copied ? (
										<CheckCircle2 className="h-5 w-5 text-green-500" />
									) : (
										<Copy className="h-5 w-5 opacity-50 group-hover:opacity-100" />
									)}
								</button>
							</div>

							<div className="flex items-center gap-2 text-sm text-muted-foreground">
								<Loader2 className="h-4 w-4 animate-spin" />
								<span>Waiting for authorization...</span>
							</div>
						</div>
					)}

					{state.status === "success" && (
						<div className="flex flex-col items-center gap-2">
							<CheckCircle2 className="h-12 w-12 text-green-500" />
							<p className="text-sm font-medium">Connected successfully!</p>
						</div>
					)}

					{state.status === "error" && (
						<div className="flex flex-col items-center gap-3">
							<p className="text-sm text-destructive">{state.message}</p>
							<Button variant="outline" onClick={startDeviceFlow}>
								Try Again
							</Button>
						</div>
					)}
				</div>
			</DialogContent>
		</Dialog>
	);
}
