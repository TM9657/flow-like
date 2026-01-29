"use client";

import { ExternalLink, Info, Plus, X } from "lucide-react";
import { useState } from "react";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "../../ui/accordion";
import { Badge } from "../../ui/badge";
import { Button } from "../../ui/button";
import { Input } from "../../ui/input";
import { Label } from "../../ui/label";
import { Switch } from "../../ui/switch";
import type { IConfigInterfaceProps } from "../interfaces";

export function TelegramConfig({
	isEditing,
	config,
	onConfigUpdate,
}: IConfigInterfaceProps) {
	const [newWhitelistId, setNewWhitelistId] = useState("");
	const [newBlacklistId, setNewBlacklistId] = useState("");

	const setValue = (key: string, value: unknown) => {
		if (onConfigUpdate) {
			onConfigUpdate({
				...config,
				[key]: value,
			});
		}
	};

	const botToken = (config?.bot_token as string) ?? "";
	const botName = (config?.bot_name as string) ?? "My Telegram Bot";
	const botDescription = (config?.bot_description as string) ?? "";
	const chatWhitelist: string[] =
		(config?.chat_whitelist as string[]) ?? [];
	const chatBlacklist: string[] =
		(config?.chat_blacklist as string[]) ?? [];
	const respondToMentions = (config?.respond_to_mentions as boolean) ?? true;
	const respondToPrivate = (config?.respond_to_private as boolean) ?? true;
	const commandPrefix = (config?.command_prefix as string) ?? "/";

	const addToWhitelist = () => {
		if (newWhitelistId && !chatWhitelist.includes(newWhitelistId)) {
			setValue("chat_whitelist", [...chatWhitelist, newWhitelistId]);
			setNewWhitelistId("");
		}
	};

	const removeFromWhitelist = (chatId: string) => {
		setValue(
			"chat_whitelist",
			chatWhitelist.filter((id) => id !== chatId),
		);
	};

	const addToBlacklist = () => {
		if (newBlacklistId && !chatBlacklist.includes(newBlacklistId)) {
			setValue("chat_blacklist", [...chatBlacklist, newBlacklistId]);
			setNewBlacklistId("");
		}
	};

	const removeFromBlacklist = (chatId: string) => {
		setValue(
			"chat_blacklist",
			chatBlacklist.filter((id) => id !== chatId),
		);
	};

	return (
		<div className="w-full space-y-6">
			{/* Bot Token */}
			<div className="space-y-3">
				<Label htmlFor="bot_token">Bot Token</Label>
				{isEditing ? (
					<Input
						type="password"
						value={botToken}
						onChange={(e) => setValue("bot_token", e.target.value)}
						id="bot_token"
						placeholder="Enter your Telegram bot token"
						className="font-mono"
					/>
				) : (
					<div className="text-sm text-muted-foreground font-mono">
						{botToken ? "••••••••••••••••" : "Not set"}
					</div>
				)}
				<p className="text-xs text-muted-foreground">
					Get your bot token from{" "}
					<a
						href="https://t.me/BotFather"
						target="_blank"
						rel="noopener noreferrer"
						className="text-primary hover:underline inline-flex items-center gap-1"
					>
						@BotFather <ExternalLink className="h-3 w-3" />
					</a>
				</p>
			</div>

			{/* Bot Identity */}
			<Accordion type="single" collapsible defaultValue="identity">
				<AccordionItem value="identity" className="border rounded-lg px-4">
					<AccordionTrigger className="hover:no-underline">
						<div className="flex items-center gap-2">
							<Info className="h-4 w-4" />
							<span>Bot Identity</span>
						</div>
					</AccordionTrigger>
					<AccordionContent className="space-y-4 pt-2">
						<div className="space-y-2">
							<Label htmlFor="bot_name">Bot Name</Label>
							{isEditing ? (
								<Input
									value={botName}
									onChange={(e) => setValue("bot_name", e.target.value)}
									id="bot_name"
									placeholder="My Telegram Bot"
								/>
							) : (
								<div className="text-sm text-muted-foreground">{botName}</div>
							)}
						</div>

						<div className="space-y-2">
							<Label htmlFor="bot_description">Bot Description</Label>
							{isEditing ? (
								<Input
									value={botDescription}
									onChange={(e) => setValue("bot_description", e.target.value)}
									id="bot_description"
									placeholder="A helpful bot powered by Flow-Like"
								/>
							) : (
								<div className="text-sm text-muted-foreground">
									{botDescription || "Not set"}
								</div>
							)}
						</div>
					</AccordionContent>
				</AccordionItem>
			</Accordion>

			{/* Response Settings */}
			<Accordion type="single" collapsible defaultValue="responses">
				<AccordionItem value="responses" className="border rounded-lg px-4">
					<AccordionTrigger className="hover:no-underline">
						<div className="flex items-center gap-2">
							<Info className="h-4 w-4" />
							<span>Response Settings</span>
						</div>
					</AccordionTrigger>
					<AccordionContent className="space-y-4 pt-2">
						<div className="flex items-center justify-between">
							<div className="space-y-0.5">
								<Label>Respond to Mentions</Label>
								<p className="text-xs text-muted-foreground">
									Bot will respond when mentioned in group chats
								</p>
							</div>
							<Switch
								checked={respondToMentions}
								onCheckedChange={(checked) =>
									setValue("respond_to_mentions", checked)
								}
								disabled={!isEditing}
							/>
						</div>

						<div className="flex items-center justify-between">
							<div className="space-y-0.5">
								<Label>Respond to Private Messages</Label>
								<p className="text-xs text-muted-foreground">
									Bot will respond to direct messages
								</p>
							</div>
							<Switch
								checked={respondToPrivate}
								onCheckedChange={(checked) =>
									setValue("respond_to_private", checked)
								}
								disabled={!isEditing}
							/>
						</div>

						<div className="space-y-2">
							<Label htmlFor="command_prefix">Command Prefix</Label>
							{isEditing ? (
								<Input
									value={commandPrefix}
									onChange={(e) => setValue("command_prefix", e.target.value)}
									id="command_prefix"
									placeholder="/"
									className="w-20"
								/>
							) : (
								<div className="text-sm text-muted-foreground font-mono">
									{commandPrefix}
								</div>
							)}
							<p className="text-xs text-muted-foreground">
								Prefix for bot commands (default: /)
							</p>
						</div>
					</AccordionContent>
				</AccordionItem>
			</Accordion>

			{/* Chat Filters */}
			<Accordion type="single" collapsible>
				<AccordionItem value="filters" className="border rounded-lg px-4">
					<AccordionTrigger className="hover:no-underline">
						<div className="flex items-center gap-2">
							<Info className="h-4 w-4" />
							<span>Chat Filters</span>
							{(chatWhitelist.length > 0 || chatBlacklist.length > 0) && (
								<Badge variant="secondary" className="ml-2">
									{chatWhitelist.length + chatBlacklist.length} configured
								</Badge>
							)}
						</div>
					</AccordionTrigger>
					<AccordionContent className="space-y-4 pt-2">
						{/* Whitelist */}
						<div className="space-y-2">
							<Label>Chat Whitelist</Label>
							<p className="text-xs text-muted-foreground">
								If set, bot will only respond in these chats. Leave empty to
								allow all.
							</p>
							{isEditing && (
								<div className="flex gap-2">
									<Input
										value={newWhitelistId}
										onChange={(e) => setNewWhitelistId(e.target.value)}
										placeholder="Chat ID (e.g., -1001234567890)"
										className="font-mono"
									/>
									<Button
										type="button"
										variant="outline"
										size="icon"
										onClick={addToWhitelist}
									>
										<Plus className="h-4 w-4" />
									</Button>
								</div>
							)}
							<div className="flex flex-wrap gap-2">
								{chatWhitelist.map((chatId) => (
									<Badge
										key={chatId}
										variant="secondary"
										className="font-mono flex items-center gap-1"
									>
										{chatId}
										{isEditing && (
											<button
												type="button"
												onClick={() => removeFromWhitelist(chatId)}
												className="ml-1 hover:text-destructive"
											>
												<X className="h-3 w-3" />
											</button>
										)}
									</Badge>
								))}
								{chatWhitelist.length === 0 && (
									<span className="text-xs text-muted-foreground">
										No whitelist configured (all chats allowed)
									</span>
								)}
							</div>
						</div>

						{/* Blacklist */}
						<div className="space-y-2">
							<Label>Chat Blacklist</Label>
							<p className="text-xs text-muted-foreground">
								Bot will never respond in these chats
							</p>
							{isEditing && (
								<div className="flex gap-2">
									<Input
										value={newBlacklistId}
										onChange={(e) => setNewBlacklistId(e.target.value)}
										placeholder="Chat ID (e.g., -1001234567890)"
										className="font-mono"
									/>
									<Button
										type="button"
										variant="outline"
										size="icon"
										onClick={addToBlacklist}
									>
										<Plus className="h-4 w-4" />
									</Button>
								</div>
							)}
							<div className="flex flex-wrap gap-2">
								{chatBlacklist.map((chatId) => (
									<Badge
										key={chatId}
										variant="destructive"
										className="font-mono flex items-center gap-1"
									>
										{chatId}
										{isEditing && (
											<button
												type="button"
												onClick={() => removeFromBlacklist(chatId)}
												className="ml-1 hover:text-destructive-foreground"
											>
												<X className="h-3 w-3" />
											</button>
										)}
									</Badge>
								))}
								{chatBlacklist.length === 0 && (
									<span className="text-xs text-muted-foreground">
										No blacklist configured
									</span>
								)}
							</div>
						</div>

						<p className="text-xs text-muted-foreground bg-muted p-2 rounded">
							<strong>Tip:</strong> To get a chat ID, forward a message from
							the chat to @userinfobot or use the Telegram API.
						</p>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</div>
	);
}
