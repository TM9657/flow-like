"use client";

import { invoke } from "@tauri-apps/api/core";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Badge,
	Breadcrumb,
	BreadcrumbItem,
	BreadcrumbLink,
	BreadcrumbList,
	BreadcrumbPage,
	BreadcrumbSeparator,
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	HoverCard,
	HoverCardContent,
	HoverCardTrigger,
	IAppVisibility,
	type IEvent,
	Input,
	Label,
	ScrollArea,
	Separator,
	Skeleton,
	Switch,
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
	VisibilityIcon,
	toastError,
	useBackend,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { useLiveQuery } from "dexie-react-hooks";
import {
	CableIcon,
	ChartAreaIcon,
	CogIcon,
	CopyIcon,
	CrownIcon,
	DatabaseIcon,
	DownloadIcon,
	EyeIcon,
	EyeOffIcon,
	FolderClosedIcon,
	GlobeIcon,
	LayoutGridIcon,
	LockIcon,
	Maximize2Icon,
	Minimize2Icon,
	PlayCircleIcon,
	SparklesIcon,
	SquarePenIcon,
	UnlockIcon,
	UsersRoundIcon,
	WorkflowIcon,
	ZapIcon,
} from "lucide-react";
import Link from "next/link";
import { usePathname, useSearchParams } from "next/navigation";
import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { appsDB } from "../../../lib/apps-db";
import { EVENT_CONFIG } from "../../../lib/event-config";

const navigationItems = [
	{
		href: "/library/config",
		label: "General",
		icon: SquarePenIcon,
		description: "Basic app information and settings",
	},
	{
		href: "/library/config/configuration",
		label: "Configuration",
		icon: CogIcon,
		description: "App configuration and environment variables",
	},
	{
		href: "/library/config/flows",
		label: "Flows",
		icon: WorkflowIcon,
		description: "Business logic and workflow definitions",
	},
	{
		href: "/library/config/templates",
		label: "Templates",
		icon: CopyIcon,
		description: "Reusable Flow templates",
	},
	{
		href: "/library/config/events",
		label: "Events",
		icon: CableIcon,
		description: "Event handling and triggers",
	},
	{
		href: "/library/config/storage",
		label: "Storage",
		icon: FolderClosedIcon,
		description: "Data storage and file management",
	},
	{
		href: "/library/config/explore",
		label: "Explore Data",
		icon: DatabaseIcon,
		description: "Browse and query your data",
		disabled: true,
	},
	{
		href: "/library/config/team",
		label: "Team",
		icon: UsersRoundIcon,
		description: "Manage team members and permissions",
		visibilities: [
			IAppVisibility.Public,
			IAppVisibility.Prototype,
			IAppVisibility.PublicRequestAccess,
		],
	},
	{
		href: "/library/config/roles",
		label: "Roles",
		icon: CrownIcon,
		description: "Define user roles and access levels",
		visibilities: [
			IAppVisibility.Public,
			IAppVisibility.Prototype,
			IAppVisibility.PublicRequestAccess,
		],
	},
	{
		href: "/library/config/analytics",
		label: "Analytics",
		icon: ChartAreaIcon,
		description: "Performance metrics and insights",
		disabled: true,
	},
	{
		href: "/library/config/endpoints",
		label: "Endpoints",
		icon: GlobeIcon,
		description: "API endpoints and integrations",
		disabled: true,
	},
];

export default function Id({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	const backend = useBackend();
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const online = useLiveQuery(
		() =>
			appsDB.visibility
				.where("appId")
				.equals(id ?? "")
				.first(),
		[id ?? ""],
	) ?? { visibility: IAppVisibility.Offline };
	const currentRoute = usePathname();
	const metadata = useInvoke(
		backend.appState.getAppMeta,
		backend.appState,
		[id ?? ""],
		typeof id === "string",
	);
	const app = useInvoke(
		backend.appState.getApp,
		backend.appState,
		[id ?? ""],
		typeof id === "string",
	);
	const [isMaximized, setIsMaximized] = useState(false);
	const [exportOpen, setExportOpen] = useState(false);
	const [encrypt, setEncrypt] = useState(false);
	const [password, setPassword] = useState("");
	const [confirmPassword, setConfirmPassword] = useState("");
	const [showPassword, setShowPassword] = useState(false);
	const [exporting, setExporting] = useState(false);

	useEffect(() => {
		const saved = localStorage.getItem("exportEncrypted");
		if (saved != null) setEncrypt(saved === "true");
	}, []);

	useEffect(() => {
		localStorage.setItem("exportEncrypted", String(encrypt));
		if (!encrypt) {
			setPassword("");
			setConfirmPassword("");
		}
	}, [encrypt]);

	const strength = useMemo(() => {
		if (!encrypt) return 0;
		let s = 0;
		if (password.length >= 8) s++;
		if (/[A-Z]/.test(password) && /[a-z]/.test(password)) s++;
		if (/\d/.test(password)) s++;
		if (/[^A-Za-z0-9]/.test(password)) s++;
		return s; // 0..4
	}, [password, encrypt]);

	const passValid =
		!encrypt || (password.length >= 8 && password === confirmPassword);

	const handleExport = useCallback(async () => {
		const loader = toast.loading("Exporting app...", {
			description: "This may take a moment, please wait.",
		});
		setExporting(true);
		try {
			await invoke("export_app_to_file", {
				appId: id,
				...(encrypt && password ? { password } : {}),
			});
			toast.success("App exported successfully!", { id: loader });
			setExportOpen(false);
			setPassword("");
			setConfirmPassword("");
		} catch (error) {
			console.error("Export error:", error);
			toast.error("Failed to export app");
		} finally {
			setExporting(false);
			toast.dismiss(loader);
		}
	}, [id, encrypt, password]);

	const events = useInvoke(
		backend.eventState.getEvents,
		backend.eventState,
		[id ?? ""],
		(id ?? "") !== "",
	);

	const usableEvents = useMemo(() => {
		const events = new Set<string>();
		Object.values(EVENT_CONFIG).forEach((config) => {
			const usable = Object.keys(config.useInterfaces);
			for (const eventType of usable) {
				if (config.eventTypes.includes(eventType)) {
					events.add(eventType);
				}
			}
		});
		return events;
	}, [EVENT_CONFIG]);

	async function executeEvent(event: IEvent) {
		if (!id) return;
		const runMeta = await backend.eventState.executeEvent(
			id,
			event.id,
			{
				id: event.node_id,
			},
			false,
			(eventId) => {},
			(events) => {},
		);

		if (!runMeta) {
			toastError(
				"Failed to execute board",
				<PlayCircleIcon className="w-4 h-4" />,
			);
		}
	}

	return (
		<TooltipProvider>
			<main className="flex min-h-screen max-h-screen overflow-hidden flex-col w-full p-6 gap-6">
				{/* Enhanced Breadcrumb - Hidden when maximized */}
				{!isMaximized && (
					<Card className="border-0 shadow-sm bg-gradient-to-r from-background to-muted/20">
						<CardContent className="p-4 flex flex-row items-center justify-between">
							<Breadcrumb>
								<BreadcrumbList>
									<BreadcrumbItem>
										<BreadcrumbLink
											href="/library"
											className="flex items-center gap-1"
										>
											<LayoutGridIcon className="w-3 h-3" />
											Home
										</BreadcrumbLink>
									</BreadcrumbItem>
									<BreadcrumbSeparator />
									<BreadcrumbItem>
										<BreadcrumbPage className="font-medium flex flex-row items-center gap-2">
											{metadata.isFetching ? (
												<Skeleton className="h-4 w-24" />
											) : (
												metadata.data?.name
											)}
											{/* Visibility Badge Overlay */}
											{app.data?.visibility && (
												<div className="bg-gray-600/40 dark:bg-background rounded-full">
													<VisibilityIcon visibility={app.data?.visibility} />
												</div>
											)}
										</BreadcrumbPage>
									</BreadcrumbItem>
								</BreadcrumbList>
							</Breadcrumb>
							{/* Use App */}
							{events.data?.find((event) =>
								usableEvents.has(event.event_type),
							) && (
								<div>
									<Link
										href={`/use?id=${id}&eventId=${
											events.data?.find((event) =>
												usableEvents.has(event.event_type),
											)?.id
										}`}
										className="w-full"
									>
										<Button
											size={"sm"}
											className="flex items-center gap-2 w-full rounded-full px-4"
										>
											<SparklesIcon className="w-4 h-4" />
											<h4 className="text-sm font-medium">Use App</h4>
										</Button>
									</Link>
								</div>
							)}
						</CardContent>
					</Card>
				)}

				{/* Enhanced Layout */}
				<div
					className={`grid w-full items-start gap-6 flex-grow overflow-hidden max-h-full transition-all duration-300 ${
						isMaximized
							? "grid-cols-1"
							: "md:grid-cols-[240px_1fr] lg:grid-cols-[260px_1fr]"
					}`}
				>
					{/* Enhanced Navigation - Hidden when maximized */}
					{!isMaximized && (
						<Card className="h-full flex flex-col flex-grow max-h-full overflow-hidden">
							<CardHeader className="pb-3 pt-3 border-b relative">
								<div className="flex flex-col gap-3">
									<div className="flex items-center gap-2 w-full">
										<div className="relative">
											<Avatar className="w-9 h-9 border border-border/50 shadow-sm transition-all duration-300 group-hover:scale-105">
												<AvatarImage
													className="scale-105 transition-transform duration-300 group-hover:scale-110"
													src={metadata.data?.icon ?? "/app-logo.webp"}
													alt={`${metadata.data?.name ?? id} icon`}
												/>
												<AvatarFallback className="text-xs font-semibold bg-gradient-to-br from-primary/20 to-primary/10">
													{(metadata.data?.name ?? id ?? "Unknown")
														.substring(0, 2)
														.toUpperCase()}
												</AvatarFallback>
											</Avatar>
										</div>
										<div className="flex-1 min-w-0">
											<CardTitle className="text-sm truncate">
												{metadata.isFetching ? (
													<Skeleton className="h-4 w-24" />
												) : (
													metadata.data?.name
												)}
											</CardTitle>
										</div>
									</div>

									{/* Description */}
									{metadata.data?.description && (
										<p className="text-xs text-muted-foreground leading-relaxed line-clamp-2">
											{metadata.data.description}
										</p>
									)}

									{/* Tags and Status */}
									<div className="flex flex-col gap-2">
										{/* Tags */}
										{metadata.data?.tags && metadata.data.tags.length > 0 && (
											<div className="flex flex-wrap gap-1 mb-2">
												{metadata.data.tags.slice(0, 2).map((tag) => (
													<Badge
														key={tag}
														variant="secondary"
														className="text-xs px-2 py-0.5"
													>
														{tag}
													</Badge>
												))}
												{metadata.data.tags.length > 2 && (
													<Tooltip>
														<TooltipTrigger asChild>
															<Badge
																variant="outline"
																className="text-xs px-2 py-0.5"
															>
																+{metadata.data.tags.length - 2}
															</Badge>
														</TooltipTrigger>
														<TooltipContent side="right" className="max-w-xs">
															<div className="space-y-1">
																{metadata.data.tags.slice(2).map((tag) => (
																	<Badge
																		key={tag}
																		variant="secondary"
																		className="text-xs mr-1"
																	>
																		{tag}
																	</Badge>
																))}
															</div>
														</TooltipContent>
													</Tooltip>
												)}
											</div>
										)}
									</div>
								</div>
							</CardHeader>
							<CardContent className="flex-1 p-0 overflow-hidden">
								<ScrollArea className="h-full px-3 flex-1">
									<div className="pt-3">
										<CardTitle className="text-sm font-medium text-muted-foreground mb-3">
											Navigation
										</CardTitle>
									</div>
									<nav
										className="flex flex-col gap-1 pb-4"
										key={id + (online?.visibility ?? "")}
									>
										{navigationItems
											.filter(
												(item) =>
													!item.visibilities ||
													item.visibilities.includes(
														online?.visibility ?? IAppVisibility.Offline,
													),
											)
											.map((item) => {
												const isActive = currentRoute.endsWith(
													item.href.split("/").pop() ?? "",
												);
												const Icon = item.icon;

												if (item.disabled) {
													return (
														<Tooltip key={item.href} delayDuration={300}>
															<TooltipTrigger asChild>
																<div
																	className={`
																		flex items-center gap-3 px-3 py-2 rounded-lg text-sm
																		text-muted-foreground bg-muted/50 opacity-60 cursor-not-allowed
																	`}
																	tabIndex={-1}
																	aria-disabled="true"
																>
																	<Icon className="w-4 h-4 flex-shrink-0" />
																	<span className="truncate">{item.label}</span>
																</div>
															</TooltipTrigger>
															<TooltipContent side="right" className="max-w-xs">
																<p className="font-bold">
																	{item.label} (Coming soon!)
																</p>
																<p className="text-xs mt-1">
																	{item.description}
																</p>
															</TooltipContent>
														</Tooltip>
													);
												}

												return (
													<Tooltip key={item.href} delayDuration={300}>
														<TooltipTrigger asChild>
															<Link
																href={`${item.href}?id=${id}`}
																className={`
                            flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all
                            ${"hover:bg-muted text-muted-foreground hover:text-foreground"}
                        `}
															>
																<Icon className="w-4 h-4 flex-shrink-0" />
																<span className="truncate">{item.label}</span>
															</Link>
														</TooltipTrigger>
														<TooltipContent side="right" className="max-w-xs">
															<p className="font-bold">{item.label}</p>
															<p className="text-xs mt-1">{item.description}</p>
														</TooltipContent>
													</Tooltip>
												);
											})}
										{(online?.visibility ?? IAppVisibility.Private) ===
											IAppVisibility.Offline && (
											<Tooltip key={"export"} delayDuration={300}>
												<Dialog open={exportOpen} onOpenChange={setExportOpen}>
													<TooltipTrigger asChild>
														<DialogTrigger asChild>
															<Button
																variant={"link"}
																className={`
                            flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all justify-start
                            ${"hover:bg-muted text-muted-foreground hover:text-foreground"}
                          `}
															>
																<DownloadIcon className="w-4 h-4 flex-shrink-0" />
																<span className="truncate">Export App</span>
															</Button>
														</DialogTrigger>
													</TooltipTrigger>

													<TooltipContent side="right" className="max-w-xs">
														<p className="font-bold">Export Application</p>
														<p className="text-xs mt-1">
															Export the application to a file for backup or
															sharing.
														</p>
													</TooltipContent>

													<DialogContent className="sm:max-w-[520px]">
														<DialogHeader>
															<DialogTitle>Export Application</DialogTitle>
															<DialogDescription>
																Choose how you want to export your app.
															</DialogDescription>
														</DialogHeader>

														<div className="space-y-4">
															<div className="flex items-center justify-between rounded-lg border p-3">
																<div className="flex items-center gap-3">
																	{encrypt ? (
																		<LockIcon className="w-4 h-4 text-primary" />
																	) : (
																		<UnlockIcon className="w-4 h-4 text-muted-foreground" />
																	)}
																	<div className="min-w-0">
																		<p className="text-sm font-medium">
																			{encrypt
																				? "Encrypted export"
																				: "Unencrypted export"}
																		</p>
																		<p className="text-xs text-muted-foreground">
																			{encrypt
																				? "Protect your export with a password."
																				: "Quick export without encryption."}
																		</p>
																	</div>
																</div>
																<div className="flex items-center gap-2">
																	<span className="text-xs text-muted-foreground">
																		Encrypt
																	</span>
																	<Switch
																		checked={encrypt}
																		onCheckedChange={setEncrypt}
																	/>
																</div>
															</div>

															{encrypt && (
																<div className="space-y-3">
																	<div className="grid gap-2">
																		<Label
																			htmlFor="export-password"
																			className="text-xs"
																		>
																			Password
																		</Label>
																		<div className="relative">
																			<Input
																				id="export-password"
																				type={
																					showPassword ? "text" : "password"
																				}
																				value={password}
																				onChange={(e) =>
																					setPassword(e.target.value)
																				}
																				placeholder="Enter a strong password"
																				autoFocus
																			/>
																			<Button
																				type="button"
																				variant="ghost"
																				size="icon"
																				className="absolute right-1 top-1 h-7 w-7"
																				onClick={() =>
																					setShowPassword((s) => !s)
																				}
																				aria-label={
																					showPassword
																						? "Hide password"
																						: "Show password"
																				}
																			>
																				{showPassword ? (
																					<EyeOffIcon className="w-4 h-4" />
																				) : (
																					<EyeIcon className="w-4 h-4" />
																				)}
																			</Button>
																		</div>
																	</div>

																	<div className="grid gap-2">
																		<Label
																			htmlFor="export-password-confirm"
																			className="text-xs"
																		>
																			Confirm password
																		</Label>
																		<Input
																			id="export-password-confirm"
																			type={showPassword ? "text" : "password"}
																			value={confirmPassword}
																			onChange={(e) =>
																				setConfirmPassword(e.target.value)
																			}
																			placeholder="Re-enter password"
																		/>
																	</div>

																	<div className="flex items-center gap-2">
																		<div className="flex gap-1" aria-hidden>
																			{[0, 1, 2, 3].map((i) => (
																				<span
																					key={i}
																					className={`h-1.5 w-10 rounded ${strength > i ? "bg-green-500" : "bg-muted"}`}
																				/>
																			))}
																		</div>
																		<span className="text-xs text-muted-foreground">
																			{strength <= 1
																				? "Weak"
																				: strength === 2
																					? "Fair"
																					: strength === 3
																						? "Good"
																						: "Strong"}
																		</span>
																	</div>

																	{!passValid && (
																		<p className="text-xs text-destructive">
																			Passwords must match and be at least 8
																			characters.
																		</p>
																	)}
																</div>
															)}
														</div>

														<DialogFooter className="gap-2">
															<Button
																variant="outline"
																onClick={() => setExportOpen(false)}
																disabled={exporting}
															>
																Cancel
															</Button>
															<Button
																onClick={handleExport}
																disabled={exporting || (encrypt && !passValid)}
															>
																{exporting ? "Exporting..." : "Export"}
															</Button>
														</DialogFooter>
													</DialogContent>
												</Dialog>
											</Tooltip>
										)}
									</nav>

									<Separator className="my-4 mx-3" />

									{/* Enhanced Quick Actions */}
									<div className="px-3">
										<div className="flex items-center gap-2 mb-3">
											<ZapIcon className="w-4 h-4 text-primary" />
											<h4 className="text-sm font-medium">Quick Actions</h4>
										</div>
										<div className="flex flex-col gap-2 pb-4">
											{events.data &&
											events.data.filter(
												(event) =>
													event.event_type === "quick_action" && event.active,
											).length > 0 ? (
												events.data
													.filter(
														(event) =>
															event.event_type === "quick_action" &&
															event.active,
													)
													.map((event) => (
														<HoverCard
															key={event.id}
															openDelay={100}
															closeDelay={100}
														>
															<HoverCardTrigger asChild>
																<Button
																	variant="outline"
																	size="sm"
																	className="justify-start gap-2 h-auto py-2 px-3"
																	onClick={async () => {
																		await executeEvent(event);
																	}}
																>
																	<PlayCircleIcon className="w-3 h-3 text-green-600" />
																	<span className="truncate text-xs">
																		{event.name}
																	</span>
																</Button>
															</HoverCardTrigger>
															<HoverCardContent side="right" className="w-80">
																<div className="space-y-2">
																	<div>
																		<h4 className="text-base font-medium">
																			{event.name}
																		</h4>
																		<p className="text-sm text-muted-foreground">
																			{event.description}
																		</p>
																	</div>
																</div>
															</HoverCardContent>
														</HoverCard>
													))
											) : (
												<p className="text-xs text-muted-foreground py-2">
													No quick actions available
												</p>
											)}
										</div>
									</div>
								</ScrollArea>
							</CardContent>
						</Card>
					)}

					{/* Enhanced Content Area with Maximize Button */}
					<Card
						className={`h-full flex flex-col flex-grow overflow-hidden transition-all duration-300 bg-transparent ${
							isMaximized ? "shadow-2xl" : ""
						}`}
					>
						<CardHeader className="pb-0 pt-4 px-4">
							<div className="flex items-center justify-between">
								<div className="flex-1" />
								<Tooltip>
									<TooltipTrigger asChild>
										<Button
											variant="ghost"
											size="sm"
											onClick={() => setIsMaximized(!isMaximized)}
											className="h-8 w-8 p-0"
										>
											{isMaximized ? (
												<Minimize2Icon className="w-4 h-4" />
											) : (
												<Maximize2Icon className="w-4 h-4" />
											)}
										</Button>
									</TooltipTrigger>
									<TooltipContent>
										{isMaximized ? "Minimize" : "Maximize"}
									</TooltipContent>
								</Tooltip>
							</div>
						</CardHeader>
						<CardContent className="flex-1 p-6 pb-0 pt-0 overflow-hidden">
							<Suspense
								fallback={
									<div className="space-y-4">
										<Skeleton className="h-8 w-full" />
										<Skeleton className="h-32 w-full" />
										<Skeleton className="h-24 w-full" />
									</div>
								}
							>
								{children}
							</Suspense>
						</CardContent>
					</Card>
				</div>
			</main>
		</TooltipProvider>
	);
}
