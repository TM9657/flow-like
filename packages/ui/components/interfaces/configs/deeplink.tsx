"use client";

import { useState } from "react";
import {
	Button,
	Input,
	Label,
	Textarea,
} from "../../ui";
import type { IConfigInterfaceProps } from "../interfaces";
import { Check, Copy, ExternalLink } from "lucide-react";

export function DeeplinkConfig({
	isEditing,
	appId,
	config,
	onConfigUpdate,
}: IConfigInterfaceProps) {
	const [copied, setCopied] = useState(false);

	const setValue = (key: string, value: any) => {
		if (onConfigUpdate) {
			onConfigUpdate({
				...config,
				[key]: value,
			});
		}
	};

	const path = config?.path ?? "";
	const deeplinkUrl = `flow-like://trigger/${appId}/${path}`;
	const exampleWithParams = `${deeplinkUrl}?param1=value1&param2=value2`;

	const copyToClipboard = (text: string) => {
		navigator.clipboard.writeText(text);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	const openDeeplink = () => {
		window.location.href = deeplinkUrl;
	};

	return (
		<div className="w-full space-y-6">
			<div className="space-y-3">
				<Label htmlFor="path">Deep Link Path</Label>
				{isEditing ? (
					<Input
						value={path}
						onChange={(e) => setValue("path", e.target.value)}
						id="path"
						placeholder="my-trigger"
					/>
				) : (
					<div className="flex h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm">
						{path}
					</div>
				)}
				<p className="text-sm text-muted-foreground">
					The path segment that identifies this trigger
				</p>
			</div>

			<div className="space-y-3">
				<Label>Deep Link URL</Label>
				<div className="flex items-center gap-2">
					<div className="flex-1 flex h-10 w-full rounded-md border border-input bg-muted px-3 py-2 text-sm font-mono overflow-x-auto">
						{deeplinkUrl}
					</div>
					<Button
						variant="outline"
						size="icon"
						onClick={() => copyToClipboard(deeplinkUrl)}
						title="Copy to clipboard"
					>
						{copied ? (
							<Check className="h-4 w-4 text-green-500" />
						) : (
							<Copy className="h-4 w-4" />
						)}
					</Button>
					<Button
						variant="outline"
						size="icon"
						onClick={openDeeplink}
						title="Test deep link"
					>
						<ExternalLink className="h-4 w-4" />
					</Button>
				</div>
				<p className="text-sm text-muted-foreground">
					Click the link icon to test the deep link trigger
				</p>
			</div>

			<div className="space-y-3">
				<Label>Usage Examples</Label>
				<div className="space-y-4">
					<div>
						<p className="text-sm font-medium mb-2">Basic Trigger</p>
						<code className="block p-3 bg-muted rounded-md text-sm font-mono break-all">
							{deeplinkUrl}
						</code>
					</div>

					<div>
						<p className="text-sm font-medium mb-2">With Query Parameters</p>
						<code className="block p-3 bg-muted rounded-md text-sm font-mono break-all">
							{exampleWithParams}
						</code>
						<p className="text-xs text-muted-foreground mt-2">
							Query parameters are passed as the event payload
						</p>
					</div>

					<div>
						<p className="text-sm font-medium mb-2">iOS Shortcuts</p>
						<div className="p-3 bg-muted rounded-md text-sm space-y-2">
							<p className="text-muted-foreground">
								1. Add an "Open URLs" action in Shortcuts
							</p>
							<p className="text-muted-foreground">
								2. Set the URL to: <code className="font-mono">{deeplinkUrl}</code>
							</p>
							<p className="text-muted-foreground">
								3. Add query parameters dynamically using Shortcut variables
							</p>
						</div>
					</div>

					<div>
						<p className="text-sm font-medium mb-2">macOS/Linux/Windows</p>
						<div className="p-3 bg-muted rounded-md text-sm space-y-2">
							<p className="text-muted-foreground">From command line:</p>
							<code className="block mt-1 font-mono text-xs">
								# macOS<br />
								open "{deeplinkUrl}"<br />
								<br />
								# Linux<br />
								xdg-open "{deeplinkUrl}"<br />
								<br />
								# Windows<br />
								start "{deeplinkUrl}"
							</code>
						</div>
					</div>
				</div>
			</div>

			<div className="rounded-lg border border-blue-200 bg-blue-50 dark:border-blue-900 dark:bg-blue-950 p-4">
				<h4 className="text-sm font-semibold text-blue-900 dark:text-blue-100 mb-2">
					💡 Use Cases
				</h4>
				<ul className="text-sm text-blue-800 dark:text-blue-200 space-y-1 list-disc list-inside">
					<li>Trigger flows from iOS/Android Shortcuts</li>
					<li>Create launcher icons for quick actions</li>
					<li>Integrate with automation tools (Alfred, Raycast, etc.)</li>
					<li>Build custom URL schemes for your workflows</li>
					<li>Pass dynamic data through query parameters</li>
				</ul>
			</div>
		</div>
	);
}
