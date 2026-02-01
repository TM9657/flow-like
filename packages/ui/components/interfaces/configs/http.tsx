"use client";

import { useMemo, useState } from "react";
import {
	Alert,
	AlertDescription,
	AlertTitle,
	Badge,
	Button,
	Input,
	Label,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Switch,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../ui";
import type { IConfigInterfaceProps } from "../interfaces";

export type HttpSink = {
	path: string;
	method: string;
	auth_token?: string | null;
};

const HTTP_METHODS = [
	{ value: "GET", label: "GET", description: "Retrieve data" },
	{ value: "POST", label: "POST", description: "Create or submit data" },
	{ value: "PUT", label: "PUT", description: "Update/replace data" },
	{ value: "PATCH", label: "PATCH", description: "Partially update data" },
	{ value: "DELETE", label: "DELETE", description: "Remove data" },
	{ value: "HEAD", label: "HEAD", description: "Get headers only" },
	{ value: "OPTIONS", label: "OPTIONS", description: "Get supported methods" },
];

export function HttpConfig({
	isEditing,
	appId,
	config,
	onConfigUpdate,
	hub,
}: IConfigInterfaceProps) {
	const [showToken, setShowToken] = useState(false);

	const path = (config?.path as string) || "/webhook";
	const method = (config?.method as string) || "POST";
	const authToken = (config?.auth_token as string | null) || null;

	const setValue = (key: string, value: any) => {
		onConfigUpdate?.({
			...config,
			[key]: value,
		});
	};

	// Compute URLs
	const localUrl = `http://localhost:9657/${appId}${path}`;

	const remoteUrl = useMemo(() => {
		if (!hub?.domain) return null;
		// Use HTTPS for production/staging, HTTP for development
		const protocol = hub.environment === "Development" ? "http" : "https";
		return `${protocol}://${hub.domain}/sink/trigger/http/${appId}${path}`;
	}, [hub?.domain, hub?.environment, appId, path]);

	const supportsRemote = hub?.supported_sinks?.http === true;

	const pathError =
		path && !path.startsWith("/") ? "Path must start with '/'" : null;

	const CurlExample = ({ url, withAuth }: { url: string; withAuth: boolean }) => (
		<pre className="mt-2 overflow-x-auto text-xs bg-muted p-3 rounded-md">
			{withAuth
				? `curl -X ${method} "${url}" \\\n  -H "Authorization: Bearer ${authToken}"`
				: `curl -X ${method} "${url}"`}
		</pre>
	);

	return (
		<div className="w-full space-y-6">
			<div className="space-y-1">
				<h3 className="text-lg font-semibold">HTTP Event Sink</h3>
				<p className="text-sm text-muted-foreground">
					Trigger this event via HTTP requests.
				</p>
			</div>

			{/* Method Selection */}
			<div className="space-y-2">
				<Label htmlFor="http_method">HTTP Method</Label>
				<Select
					value={method}
					onValueChange={(value) => setValue("method", value)}
					disabled={!isEditing}
				>
					<SelectTrigger id="http_method" className="w-full">
						<SelectValue placeholder="Select HTTP method" />
					</SelectTrigger>
					<SelectContent>
						{HTTP_METHODS.map((m) => (
							<SelectItem key={m.value} value={m.value}>
								<div className="flex items-center gap-2">
									<Badge variant="outline" className="font-mono">
										{m.label}
									</Badge>
									<span className="text-muted-foreground text-xs">
										{m.description}
									</span>
								</div>
							</SelectItem>
						))}
					</SelectContent>
				</Select>
				<p className="text-sm text-muted-foreground">
					The HTTP method that will trigger this event.
				</p>
			</div>

			{/* Path */}
			<div className="space-y-2">
				<Label htmlFor="http_path">Path</Label>
				<div className="flex items-center gap-2">
					<div className="flex-shrink-0 text-sm text-muted-foreground">
						/{appId}
					</div>
					<Input
						id="http_path"
						value={path}
						onChange={(e) => setValue("path", e.target.value)}
						placeholder="/webhook"
						disabled={!isEditing}
						className={pathError ? "border-destructive" : ""}
					/>
				</div>
				{pathError && <p className="text-sm text-destructive">{pathError}</p>}
				<p className="text-sm text-muted-foreground">
					The path for this endpoint. Must start with <code>/</code>.
				</p>
			</div>

			{/* URL Preview - Tabs for local/remote */}
			<div className="space-y-2">
				<Label>Endpoint URLs</Label>
				{supportsRemote && remoteUrl ? (
					<Tabs defaultValue="remote" className="w-full">
						<TabsList className="grid w-full grid-cols-2">
							<TabsTrigger value="remote">Remote (Server)</TabsTrigger>
							<TabsTrigger value="local">Local (Desktop)</TabsTrigger>
						</TabsList>
						<TabsContent value="remote" className="space-y-3">
							<div className="relative">
								<div className="flex h-auto min-h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm items-center font-mono break-all">
									<Badge variant="default" className="mr-2 font-mono shrink-0">
										{method}
									</Badge>
									{remoteUrl}
								</div>
								<Button
									type="button"
									variant="ghost"
									size="sm"
									className="absolute right-1 top-1 h-8"
									onClick={() => navigator.clipboard.writeText(remoteUrl)}
								>
									Copy
								</Button>
							</div>
							<p className="text-xs text-muted-foreground">
								Public endpoint for remote/cloud execution. Available 24/7.
							</p>
							<Alert>
								<AlertTitle>Example Request</AlertTitle>
								<AlertDescription>
									<CurlExample url={remoteUrl} withAuth={!!authToken} />
								</AlertDescription>
							</Alert>
						</TabsContent>
						<TabsContent value="local" className="space-y-3">
							<div className="relative">
								<div className="flex h-auto min-h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm items-center font-mono break-all">
									<Badge variant="secondary" className="mr-2 font-mono shrink-0">
										{method}
									</Badge>
									{localUrl}
								</div>
								<Button
									type="button"
									variant="ghost"
									size="sm"
									className="absolute right-1 top-1 h-8"
									onClick={() => navigator.clipboard.writeText(localUrl)}
								>
									Copy
								</Button>
							</div>
							<p className="text-xs text-muted-foreground">
								Local endpoint for desktop app execution. Only available when the app is running.
							</p>
							<Alert>
								<AlertTitle>Example Request</AlertTitle>
								<AlertDescription>
									<CurlExample url={localUrl} withAuth={!!authToken} />
								</AlertDescription>
							</Alert>
						</TabsContent>
					</Tabs>
				) : (
					<div className="space-y-3">
						<div className="relative">
							<div className="flex h-auto min-h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm items-center font-mono break-all">
								<Badge variant="secondary" className="mr-2 font-mono shrink-0">
									{method}
								</Badge>
								{localUrl}
							</div>
							<Button
								type="button"
								variant="ghost"
								size="sm"
								className="absolute right-1 top-1 h-8"
								onClick={() => navigator.clipboard.writeText(localUrl)}
							>
								Copy
							</Button>
						</div>
						<p className="text-xs text-muted-foreground">
							Local endpoint for desktop app execution. Only available when the app is running.
						</p>
						<Alert>
							<AlertTitle>Example Request</AlertTitle>
							<AlertDescription>
								<CurlExample url={localUrl} withAuth={!!authToken} />
							</AlertDescription>
						</Alert>
					</div>
				)}
			</div>

			{/* Authentication */}
			<div className="space-y-4">
				<div className="flex items-center justify-between">
					<div className="space-y-0.5">
						<Label>Authentication</Label>
						<p className="text-sm text-muted-foreground">
							Optional Bearer token to secure this endpoint
						</p>
					</div>
					<Switch
						checked={authToken !== null && authToken !== ""}
						onCheckedChange={(checked) => {
							if (checked) {
								setValue("auth_token", generateToken());
							} else {
								setValue("auth_token", null);
							}
						}}
						disabled={!isEditing}
					/>
				</div>

				{authToken && (
					<div className="space-y-2">
						<div className="flex items-center justify-between">
							<Label htmlFor="http_auth_token">Bearer Token</Label>
							<Button
								type="button"
								variant="ghost"
								size="sm"
								onClick={() => setShowToken(!showToken)}
							>
								{showToken ? "Hide" : "Show"}
							</Button>
						</div>
						<div className="flex gap-2">
							<Input
								id="http_auth_token"
								type={showToken ? "text" : "password"}
								value={authToken}
								onChange={(e) => setValue("auth_token", e.target.value)}
								placeholder="Enter token or generate one"
								disabled={!isEditing}
								className="font-mono text-xs"
							/>
							<Button
								type="button"
								variant="secondary"
								onClick={() => setValue("auth_token", generateToken())}
								disabled={!isEditing}
							>
								Generate
							</Button>
						</div>
						<p className="text-sm text-muted-foreground">
							Include this token as{" "}
							<code>Authorization: Bearer {"{token}"}</code> in your requests.
						</p>
					</div>
				)}
			</div>

			{/* Conflict Warning */}
			{!pathError && (
				<Alert>
					<AlertTitle>Route Conflicts</AlertTitle>
					<AlertDescription>
						If multiple events use the same app ID, path, and method, only the
						most recently registered event will be triggered. The system will
						log warnings if conflicts occur.
					</AlertDescription>
				</Alert>
			)}
		</div>
	);
}

function generateToken(): string {
	const array = new Uint8Array(32);
	crypto.getRandomValues(array);
	return Array.from(array, (byte) => byte.toString(16).padStart(2, "0")).join(
		"",
	);
}
