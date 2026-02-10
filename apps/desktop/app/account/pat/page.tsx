"use client";

import {
	Button,
	Calendar,
	Card,
	Dialog,
	DialogContent,
	Input,
	Label,
	Popover,
	Select,
	Separator,
	Tooltip,
	TooltipContent,
	TooltipTrigger,
	useBackend,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { cn } from "@tm9657/flow-like-ui/lib/utils";
import { format } from "date-fns";
import {
	CalendarIcon,
	CheckIcon,
	CopyIcon,
	KeyRoundIcon,
	PlusIcon,
	ShieldCheckIcon,
	Trash2Icon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

interface PAT {
	id: string;
	name: string;
	created_at: string;
	valid_until: string | null;
	permission: number;
}

const permissionLevels = [
	{ value: 1, label: "Read Only", description: "View access only" },
	{ value: 2, label: "Read & Write", description: "View and modify access" },
	{ value: 4, label: "Admin", description: "Full administrative access" },
];

const PatManagementPage = () => {
	const backend = useBackend();
	const pats = useInvoke(backend.userState.getPATs, backend.userState, []);
	const [loading, setLoading] = useState(true);
	const [creating, setCreating] = useState(false);
	const [showCreateDialog, setShowCreateDialog] = useState(false);
	const [showTokenDialog, setShowTokenDialog] = useState(false);
	const [newToken, setNewToken] = useState<string>("");
	const [newTokenPermission, setNewTokenPermission] = useState<number>(1);
	const [copiedId, setCopiedId] = useState<string | null>(null);

	// Form state
	const [tokenName, setTokenName] = useState("");
	const [expirationDate, setExpirationDate] = useState<Date | undefined>(
		undefined,
	);
	const [selectedPermission, setSelectedPermission] = useState<number>(1);

	const handleCreateToken = async () => {
		if (!tokenName.trim()) {
			toast.error("Please enter a token name");
			return;
		}

		if (!backend) {
			toast.error("Backend not available");
			return;
		}

		try {
			setCreating(true);
			const result = await backend.userState.createPAT(
				tokenName,
				expirationDate,
				selectedPermission,
			);

			setNewToken(result.pat);
			setNewTokenPermission(result.permission);
			setShowCreateDialog(false);
			setShowTokenDialog(true);

			// Reset form
			setTokenName("");
			setExpirationDate(undefined);
			setSelectedPermission(1);

			// Reload PATs list
			await pats.refetch();
			toast.success("Token created successfully!");
		} catch (error) {
			toast.error("Failed to create token");
			console.error(error);
		} finally {
			setCreating(false);
		}
	};

	const handleDeleteToken = async (id: string, name: string) => {
		if (
			!confirm(
				`Are you sure you want to delete the token "${name}"? This action cannot be undone.`,
			)
		) {
			return;
		}

		if (!backend) {
			toast.error("Backend not available");
			return;
		}

		try {
			await backend.userState.deletePAT(id);
			toast.success("Token deleted successfully");
			await pats.refetch();
		} catch (error) {
			toast.error("Failed to delete token");
			console.error(error);
		}
	};

	const copyToClipboard = (text: string, id?: string) => {
		navigator.clipboard.writeText(text);
		toast.success("Copied to clipboard!");
		if (id) {
			setCopiedId(id);
			setTimeout(() => setCopiedId(null), 2000);
		}
	};

	const formatDate = (dateString: string) => {
		return format(new Date(dateString), "MMM dd, yyyy");
	};

	const getPermissionLabel = (permission: number) => {
		return (
			permissionLevels.find((p) => p.value === permission)?.label || "Unknown"
		);
	};

	const isExpired = (validUntil: string | null) => {
		if (!validUntil) return false;
		return new Date(validUntil) < new Date();
	};

	return (
		<div className="container mx-auto px-4 py-8 max-w-6xl flex flex-col h-full flex-grow overflow-hidden">
			{/* Header Section */}
			<div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-primary/10 via-primary/5 to-background border border-primary/20 p-6 mb-6 shadow-lg">
				<div className="absolute inset-0 bg-grid-white/5 [mask-image:radial-gradient(white,transparent_85%)]" />
				<div className="relative z-10">
					<div className="flex items-center gap-3 mb-2">
						<div className="p-2 rounded-xl bg-primary/10 backdrop-blur-sm">
							<KeyRoundIcon className="h-6 w-6 text-primary" />
						</div>
						<div>
							<h1 className="text-3xl font-bold bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text text-transparent">
								Personal Access Tokens
							</h1>
						</div>
					</div>
					<p className="text-muted-foreground text-sm max-w-3xl">
						Manage your personal access tokens to integrate with external
						applications and services securely.
					</p>
				</div>
			</div>

			{/* Create Token Card */}
			<Card className="p-4 mb-6 border-2 hover:border-primary/50 transition-colors">
				<div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
					<div className="flex items-start gap-3">
						<div className="p-2 rounded-lg bg-primary/10">
							<PlusIcon className="h-4 w-4 text-primary" />
						</div>
						<div className="flex-1">
							<h2 className="text-lg font-semibold mb-0.5">
								Generate New Token
							</h2>
							<p className="text-xs text-muted-foreground">
								Create a new personal access token with custom permissions and
								expiration date.
							</p>
						</div>
					</div>
					<Button
						onClick={() => setShowCreateDialog(true)}
						size="lg"
						className="w-full md:w-auto shadow-lg hover:shadow-xl transition-all"
					>
						<PlusIcon className="h-4 w-4 mr-2" />
						Create Token
					</Button>
				</div>
			</Card>

			{/* Tokens List */}
			<div className="space-y-4 overflow-y-auto flex-grow flex flex-col pr-2">
				<div className="flex items-center justify-between mb-4 sticky top-0 bg-background z-10 pb-2">
					<h2 className="text-2xl font-semibold">Your Tokens</h2>
					<div className="text-sm text-muted-foreground">
						{pats.data?.length} {pats.data?.length === 1 ? "token" : "tokens"}
					</div>
				</div>

				{pats.isLoading ? (
					<div className="grid gap-4">
						{[1, 2, 3].map((i) => (
							<Card key={i} className="p-6 animate-pulse">
								<div className="space-y-3">
									<div className="h-5 bg-muted rounded w-1/3" />
									<div className="h-4 bg-muted rounded w-1/2" />
								</div>
							</Card>
						))}
					</div>
				) : pats.data?.length === 0 ? (
					<Card className="p-12 text-center border-dashed border-2">
						<div className="flex flex-col items-center gap-4">
							<div className="p-4 rounded-full bg-muted">
								<KeyRoundIcon className="h-8 w-8 text-muted-foreground" />
							</div>
							<div>
								<h3 className="text-lg font-semibold mb-2">No tokens yet</h3>
								<p className="text-muted-foreground max-w-md">
									Get started by creating your first personal access token to
									integrate with external services.
								</p>
							</div>
							<Button
								onClick={() => setShowCreateDialog(true)}
								className="mt-2"
							>
								<PlusIcon className="h-4 w-4 mr-2" />
								Create Your First Token
							</Button>
						</div>
					</Card>
				) : (
					<div className="grid gap-4 pb-4">
						{pats.data?.map((pat) => {
							const expired = isExpired(pat.valid_until);
							return (
								<Card
									key={pat.id}
									className={cn(
										"p-6 transition-all hover:shadow-md",
										expired && "opacity-60 border-destructive/50",
									)}
								>
									<div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
										<div className="flex-1 space-y-3">
											<div className="flex items-start gap-3">
												<div
													className={cn(
														"p-2 rounded-lg mt-0.5",
														expired ? "bg-destructive/10" : "bg-primary/10",
													)}
												>
													<KeyRoundIcon
														className={cn(
															"h-4 w-4",
															expired ? "text-destructive" : "text-primary",
														)}
													/>
												</div>
												<div className="flex-1 min-w-0">
													<div className="flex items-center gap-2 mb-1">
														<h3 className="text-lg font-semibold truncate">
															{pat.name}
														</h3>
														{expired && (
															<span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-destructive/10 text-destructive">
																Expired
															</span>
														)}
													</div>
													<div className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-muted-foreground">
														<div className="flex items-center gap-1">
															<ShieldCheckIcon className="h-3.5 w-3.5" />
															<span>{getPermissionLabel(pat.permission)}</span>
														</div>
														<div className="flex items-center gap-1">
															<CalendarIcon className="h-3.5 w-3.5" />
															<span>Created {formatDate(pat.created_at)}</span>
														</div>
														{pat.valid_until && (
															<div className="flex items-center gap-1">
																<CalendarIcon className="h-3.5 w-3.5" />
																<span>
																	Expires {formatDate(pat.valid_until)}
																</span>
															</div>
														)}
														{!pat.valid_until && (
															<span className="text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary">
																Never expires
															</span>
														)}
													</div>
												</div>
											</div>
										</div>

										<div className="flex items-center gap-2 md:flex-shrink-0">
											<Tooltip>
												<TooltipTrigger asChild>
													<Button
														variant="outline"
														size="sm"
														onClick={() => copyToClipboard(pat.id, pat.id)}
														className="gap-2"
													>
														{copiedId === pat.id ? (
															<CheckIcon className="h-4 w-4 text-green-500" />
														) : (
															<CopyIcon className="h-4 w-4" />
														)}
														<span className="hidden sm:inline">
															{copiedId === pat.id ? "Copied!" : "Copy ID"}
														</span>
													</Button>
												</TooltipTrigger>
												<TooltipContent>Copy token ID</TooltipContent>
											</Tooltip>

											<Tooltip>
												<TooltipTrigger asChild>
													<Button
														variant="destructive"
														size="sm"
														onClick={() => handleDeleteToken(pat.id, pat.name)}
													>
														<Trash2Icon className="h-4 w-4" />
													</Button>
												</TooltipTrigger>
												<TooltipContent>Delete token</TooltipContent>
											</Tooltip>
										</div>
									</div>
								</Card>
							);
						})}
					</div>
				)}
			</div>

			{/* Create Token Dialog */}
			<Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
				<DialogContent className="!w-[90vw] !max-w-6xl max-h-[90vh] overflow-y-auto">
					<div className="space-y-6">
						<div>
							<h2 className="text-2xl font-bold mb-2">Create New Token</h2>
							<p className="text-muted-foreground">
								Configure your new personal access token. The token value will
								be shown only once after creation.
							</p>
						</div>

						<Separator />

						<div className="space-y-5">
							<div className="space-y-2">
								<Label htmlFor="token-name" className="text-sm font-medium">
									Token Name *
								</Label>
								<Input
									id="token-name"
									placeholder="e.g., GitHub Actions, Production API"
									value={tokenName}
									onChange={(e) => setTokenName(e.target.value)}
									className="w-full"
								/>
								<p className="text-xs text-muted-foreground">
									Choose a descriptive name to identify this token&apos;s
									purpose.
								</p>
							</div>

							<div className="space-y-2">
								<Label className="text-sm font-medium">
									Permission Level *
								</Label>
								<Select
									value={selectedPermission.toString()}
									onValueChange={(value) =>
										setSelectedPermission(Number.parseInt(value))
									}
								>
									<option value="" disabled>
										Select permission level
									</option>
									{permissionLevels.map((level) => (
										<option key={level.value} value={level.value.toString()}>
											{level.label} - {level.description}
										</option>
									))}
								</Select>
								<p className="text-xs text-muted-foreground">
									Choose the access level for this token.
								</p>
							</div>

							<div className="space-y-2">
								<Label className="text-sm font-medium">
									Expiration Date (Optional)
								</Label>
								<Popover>
									<button
										type="button"
										className={cn(
											"w-full flex items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
											!expirationDate && "text-muted-foreground",
										)}
									>
										<span>
											{expirationDate
												? format(expirationDate, "PPP")
												: "No expiration"}
										</span>
										<CalendarIcon className="h-4 w-4 opacity-50" />
									</button>
									<div className="z-50 mt-2 flex flex-row justify-center">
										<Card className="p-3">
											<Calendar
												mode="single"
												selected={expirationDate}
												onSelect={setExpirationDate}
												disabled={(date) => date < new Date()}
												initialFocus
											/>
											{expirationDate && (
												<div className="mt-2 pt-2 border-t">
													<Button
														variant="ghost"
														size="sm"
														onClick={() => setExpirationDate(undefined)}
														className="w-full"
													>
														Clear Date
													</Button>
												</div>
											)}
										</Card>
									</div>
								</Popover>
								<p className="text-xs text-muted-foreground">
									Leave empty for a token that never expires.
								</p>
							</div>
						</div>

						<Separator />

						<div className="flex flex-col-reverse sm:flex-row gap-3 justify-end">
							<Button
								variant="outline"
								onClick={() => {
									setShowCreateDialog(false);
									setTokenName("");
									setExpirationDate(undefined);
									setSelectedPermission(1);
								}}
								disabled={creating}
							>
								Cancel
							</Button>
							<Button
								onClick={handleCreateToken}
								disabled={creating || !tokenName.trim()}
								className="shadow-lg"
							>
								{creating ? "Creating..." : "Create Token"}
							</Button>
						</div>
					</div>
				</DialogContent>
			</Dialog>

			{/* Token Display Dialog */}
			<Dialog open={showTokenDialog} onOpenChange={setShowTokenDialog}>
				<DialogContent className="max-w-2xl">
					<div className="space-y-6">
						<div className="flex items-start gap-4">
							<div className="p-3 rounded-xl bg-green-500/10">
								<CheckIcon className="h-6 w-6 text-green-500" />
							</div>
							<div className="flex-1">
								<h2 className="text-2xl font-bold mb-2">
									Token Created Successfully!
								</h2>
								<p className="text-muted-foreground">
									Your personal access token has been generated. Make sure to
									copy it now as you won&apos;t be able to see it again.
								</p>
							</div>
						</div>

						<Separator />

						<div className="space-y-4">
							<div className="p-4 rounded-lg bg-muted/50 border-2 border-dashed border-primary/30">
								<Label className="text-xs font-medium text-muted-foreground mb-2 block">
									YOUR TOKEN
								</Label>
								<div className="flex items-center gap-2 mb-3">
									<code className="flex-1 p-3 rounded bg-background border font-mono text-sm break-all select-all">
										{newToken}
									</code>
								</div>
								<Button
									variant="secondary"
									size="sm"
									onClick={() => copyToClipboard(newToken)}
									className="w-full"
								>
									<CopyIcon className="h-4 w-4 mr-2" />
									Copy Token to Clipboard
								</Button>
							</div>

							<div className="p-4 rounded-lg bg-amber-500/10 border border-amber-500/20">
								<div className="flex gap-3">
									<div className="mt-0.5">
										<ShieldCheckIcon className="h-5 w-5 text-amber-600 dark:text-amber-400" />
									</div>
									<div className="flex-1 space-y-1">
										<h4 className="font-semibold text-sm text-amber-600 dark:text-amber-400">
											Important Security Notice
										</h4>
										<ul className="text-xs text-muted-foreground space-y-1 list-disc list-inside">
											<li>Store this token in a secure location</li>
											<li>Never share it in public repositories or channels</li>
											<li>
												Treat it like a password - it has{" "}
												{getPermissionLabel(newTokenPermission)} access
											</li>
											<li>
												If compromised, delete it immediately and create a new
												one
											</li>
										</ul>
									</div>
								</div>
							</div>
						</div>

						<Separator />

						<div className="flex justify-end">
							<Button
								onClick={() => {
									setShowTokenDialog(false);
									setNewToken("");
								}}
								className="shadow-lg"
							>
								I&apos;ve Saved My Token
							</Button>
						</div>
					</div>
				</DialogContent>
			</Dialog>
		</div>
	);
};

export default PatManagementPage;
