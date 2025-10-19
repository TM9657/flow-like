"use client";

import { useState } from "react";
import {
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
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
	TextEditor,
	Textarea,
} from "../../ui";
import type { IConfigInterfaceProps } from "../interfaces";
import { Check, Copy, ExternalLink, Play } from "lucide-react";

export function ApiConfig({
	isEditing,
	appId,
	boardId,
	config,
	nodeId,
	node,
	onConfigUpdate,
}: IConfigInterfaceProps) {
	const [copiedCode, setCopiedCode] = useState<string | null>(null);
	const [testPayload, setTestPayload] = useState("{}");
	const [isTesting, setIsTesting] = useState(false);
	const [testResult, setTestResult] = useState<string | null>(null);

	const setValue = (key: string, value: any) => {
		if (onConfigUpdate) {
			onConfigUpdate({
				...config,
				[key]: value,
			});
		}
	};

	const method = config?.method ?? "GET";
	const path = config?.path ?? "/webhook";
	const publicEndpoint = config?.public_endpoint ?? false;
	const authToken = config?.auth_token ?? "";
	const endpoint = `http://localhost:9657/${appId}${path}`;

	const copyToClipboard = (text: string, id: string) => {
		navigator.clipboard.writeText(text);
		setCopiedCode(id);
		setTimeout(() => setCopiedCode(null), 2000);
	};

	const handleTestRequest = async () => {
		setIsTesting(true);
		setTestResult(null);

		try {
			const headers: Record<string, string> = {
				"Content-Type": "application/json",
			};

			if (!publicEndpoint && authToken) {
				headers.Authorization = `Bearer ${authToken}`;
			}

			const options: RequestInit = {
				method,
				headers,
			};

			if (method !== "GET" && testPayload) {
				options.body = testPayload;
			}

			const response = await fetch(endpoint, options);
			const text = await response.text();

			setTestResult(
				`Status: ${response.status}\n\n${text || "No response body"}`,
			);
		} catch (error) {
			setTestResult(`Error: ${error instanceof Error ? error.message : String(error)}`);
		} finally {
			setIsTesting(false);
		}
	};

	const generateCodeExample = (lang: string) => {
		const authHeader = !publicEndpoint && authToken
			? `Authorization: Bearer ${authToken}`
			: null;

		switch (lang) {
			case "curl":
				return `\`\`\`bash
curl -X ${method} "${endpoint}"${authHeader ? ` \\\n  -H "${authHeader}"` : ""}${method !== "GET" ? ' \\\n  -H "Content-Type: application/json" \\\n  -d \'{"key": "value"}\'' : ""}
\`\`\``;

			case "python":
				return `\`\`\`python
import requests

${authHeader ? `headers = {
    "Authorization": "${authToken}",
    "Content-Type": "application/json"
}
` : ""}${method !== "GET" ? `payload = {"key": "value"}
` : ""}
response = requests.${method.toLowerCase()}(
    "${endpoint}"${authHeader ? ",\n    headers=headers" : ""}${method !== "GET" ? ",\n    json=payload" : ""}
)
print(response.status_code, response.json())
\`\`\``;

			case "typescript":
				return `\`\`\`typescript
const response = await fetch("${endpoint}", {
  method: "${method}",${authHeader ? `\n  headers: {\n    "Authorization": "Bearer ${authToken}",\n    "Content-Type": "application/json"\n  },` : ""}${method !== "GET" ? '\n  body: JSON.stringify({"key": "value"})' : ""}
});

const data = await response.json();
console.log(data);
\`\`\``;

			case "rust":
				return `\`\`\`rust
use reqwest;

let client = reqwest::Client::new();
let response = client.${method.toLowerCase()}("${endpoint}")${authHeader ? `\n    .header("Authorization", "Bearer ${authToken}")` : ""}${method !== "GET" ? '\n    .json(&serde_json::json!({"key": "value"}))' : ""}
    .send()
    .await?;

println!("Status: {}", response.status());
let body = response.text().await?;
println!("Body: {}", body);
\`\`\``;

			default:
				return "";
		}
	};

	return (
		<div className="w-full space-y-6">
			<div className="space-y-3">
				<Label htmlFor="path">Path</Label>
				{isEditing ? (
					<input
						type="text"
						value={path}
						onChange={(e) => setValue("path", e.target.value)}
						id="path"
						placeholder="/webhook"
						className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
					/>
				) : (
					<div className="flex h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm">
						{path}
					</div>
				)}
				<p className="text-sm text-muted-foreground">
					API endpoint path (must start with /)
				</p>
			</div>

			<div className="space-y-3">
				<Label htmlFor="method">HTTP Method</Label>
				{isEditing ? (
					<Select value={method} onValueChange={(value) => setValue("method", value)}>
						<SelectTrigger id="method">
							<SelectValue placeholder="Select method" />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="GET">GET</SelectItem>
							<SelectItem value="POST">POST</SelectItem>
							<SelectItem value="PUT">PUT</SelectItem>
							<SelectItem value="PATCH">PATCH</SelectItem>
							<SelectItem value="DELETE">DELETE</SelectItem>
						</SelectContent>
					</Select>
				) : (
					<div className="flex h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm">
						{method}
					</div>
				)}
				<p className="text-sm text-muted-foreground">
					HTTP method for the API endpoint
				</p>
			</div>

			<div className="space-y-4">
				<div className="flex items-center space-x-2">
					{isEditing ? (
						<Switch
							id="public_endpoint"
							checked={publicEndpoint}
							onCheckedChange={(checked) =>
								setValue("public_endpoint", checked)
							}
						/>
					) : (
						<div
							className={`h-5 w-9 rounded-full ${publicEndpoint ? "bg-primary" : "bg-muted"} flex items-center ${publicEndpoint ? "justify-end" : "justify-start"} px-0.5`}
						>
							<div className="h-4 w-4 rounded-full bg-white" />
						</div>
					)}
					<Label htmlFor="public_endpoint">Public Endpoint</Label>
					{!isEditing && (
						<span className="text-sm text-muted-foreground">
							{publicEndpoint ? "Enabled" : "Disabled"}
						</span>
					)}
				</div>
				<p className="text-sm text-muted-foreground">
					Allow access without authentication (use with caution)
				</p>
			</div>

			{!publicEndpoint && (
				<div className="space-y-3">
					<Label htmlFor="auth_token">Authentication Token</Label>
					{isEditing ? (
						<input
							type="password"
							value={authToken}
							onChange={(e) => setValue("auth_token", e.target.value)}
							id="auth_token"
							placeholder="Bearer token for authentication"
							className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
						/>
					) : (
						<div className="flex h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm">
							{authToken ? "••••••••••••" : "No token set"}
						</div>
					)}
					<p className="text-sm text-muted-foreground">
						Required Bearer token for API requests
					</p>
				</div>
			)}

			<div className="space-y-3 pt-4 border-t">
				<div className="flex items-center justify-between">
					<Label>API Endpoint</Label>
					<Button
						variant="outline"
						size="sm"
						onClick={() => copyToClipboard(endpoint, "endpoint")}
					>
						{copiedCode === "endpoint" ? (
							<Check className="h-4 w-4" />
						) : (
							<Copy className="h-4 w-4" />
						)}
					</Button>
				</div>
				<div className="p-3 bg-muted rounded-md font-mono text-sm break-all">
					{endpoint}
				</div>
				<p className="text-sm text-muted-foreground">
					This endpoint is only accessible locally. To expose it to the internet,
					consider using{" "}
					<a
						href="https://ngrok.com"
						target="_blank"
						rel="noopener noreferrer"
						className="text-primary hover:underline inline-flex items-center gap-1"
					>
						ngrok
						<ExternalLink className="h-3 w-3" />
					</a>{" "}
					or similar tunneling services.
				</p>
			</div>

			<div className="space-y-3">
				<Label>Code Examples</Label>
				<Tabs defaultValue="curl" className="w-full">
					<TabsList className="grid w-full grid-cols-4">
						<TabsTrigger value="curl">cURL</TabsTrigger>
						<TabsTrigger value="python">Python</TabsTrigger>
						<TabsTrigger value="typescript">TypeScript</TabsTrigger>
						<TabsTrigger value="rust">Rust</TabsTrigger>
					</TabsList>
					{["curl", "python", "typescript", "rust"].map((lang) => (
						<TabsContent key={lang} value={lang} className="space-y-2">
							<div className="relative">
								<div className="prose prose-sm max-w-none dark:prose-invert">
									<TextEditor
										initialContent={generateCodeExample(lang)}
										isMarkdown={true}
									/>
								</div>
							</div>
						</TabsContent>
					))}
				</Tabs>
			</div>

			<div className="space-y-3 pt-4 border-t">
				<Label>Test Endpoint</Label>
				<Dialog>
					<DialogTrigger asChild>
						<Button variant="outline" className="w-full">
							<Play className="h-4 w-4 mr-2" />
							Test API Request
						</Button>
					</DialogTrigger>
					<DialogContent className="max-w-2xl">
						<DialogHeader>
							<DialogTitle>Test API Request</DialogTitle>
							<DialogDescription>
								Send a test request to {endpoint}
							</DialogDescription>
						</DialogHeader>
						<div className="space-y-4">
							{method !== "GET" && (
								<div className="space-y-2">
									<Label>Request Body (JSON)</Label>
									<Textarea
										value={testPayload}
										onChange={(e) => setTestPayload(e.target.value)}
										placeholder='{"key": "value"}'
										className="font-mono text-sm"
										rows={6}
									/>
								</div>
							)}
							<Button
								onClick={handleTestRequest}
								disabled={isTesting}
								className="w-full"
							>
								{isTesting ? "Sending..." : "Send Request"}
							</Button>
							{testResult && (
								<div className="space-y-2">
									<Label>Response</Label>
									<pre className="p-4 bg-muted rounded-md text-sm overflow-x-auto whitespace-pre-wrap">
										{testResult}
									</pre>
								</div>
							)}
						</div>
					</DialogContent>
				</Dialog>
			</div>
		</div>
	);
}
