"use client";

import { useDebounce } from "@uidotdev/usehooks";
import {
	Clock,
	Copy,
	ExternalLink,
	Link,
	LinkIcon,
	Mail,
	MailIcon,
	MoreVertical,
	Plus,
	RefreshCw,
	Settings,
	Trash2,
	User,
	UserCheck,
	UserPlus,
	UserPlus2Icon,
	UserX,
	Users,
	UsersIcon,
} from "lucide-react";
import { useCallback, useState } from "react";
import { toast } from "sonner";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	EmptyState,
	Input,
	Label,
	Separator,
	Textarea,
	useBackend,
	useHub,
	useInvoke,
} from "../../../";

export function InviteManagement({ appId }: Readonly<{ appId: string }>) {
	const backend = useBackend();
	const links = useInvoke(backend.teamState.getInviteLinks, backend.teamState, [
		appId,
	]);
	const [message, setMessage] = useState("");
	const [invitee, setInvitee] = useState("");
	const inviteeSearch = useDebounce(invitee, 500);
	const [showInviteDialog, setShowInviteDialog] = useState(false);
	const [showCreateLinkDialog, setShowCreateLinkDialog] = useState(false);
	const [newLinkName, setNewLinkName] = useState("");
	const [newLinkMaxUses, setNewLinkMaxUses] = useState<string>("");
	const { hub } = useHub();

	const userSearch = useInvoke(
		backend.userState.searchUsers,
		backend.userState,
		[inviteeSearch],
		inviteeSearch.length > 0,
	);

	const copyInviteLink = (token: string) => {
		navigator.clipboard.writeText(token);
		toast.success("Invite link copied to clipboard!");
	};

	const createInviteLink = useCallback(async () => {
		let maxUses: number | undefined = Number.parseInt(newLinkMaxUses);
		if (isNaN(maxUses) || maxUses <= 0) {
			maxUses = -1; // Allow unlimited uses if not specified
		}

		await backend.teamState.createInviteLink(appId, newLinkName, maxUses);
		setNewLinkName("");
		setNewLinkMaxUses("");
		setShowCreateLinkDialog(false);
		toast.success("New invite link created!");
		await links.refetch();
	}, [appId, newLinkName, newLinkMaxUses, backend]);

	const deleteInviteLink = useCallback(
		async (id: string) => {
			await backend.teamState.removeInviteLink(appId, id);
			await links.refetch();
		},
		[backend, links.refetch, appId],
	);

	return (
		<div className="grid gap-6 md:grid-cols-2 w-full pr-2">
			{/* Direct Invite Card - moved to top */}
			<Card className="md:col-span-2">
				<CardHeader>
					<CardTitle className="flex items-center gap-2">
						<UserPlus className="w-5 h-5" />
						Direct Invite
					</CardTitle>
					<CardDescription>Send a direct invitation to a user</CardDescription>
				</CardHeader>
				<CardContent>
					<Dialog open={showInviteDialog} onOpenChange={setShowInviteDialog}>
						<DialogTrigger asChild>
							<Button className="w-full bg-linear-to-r from-primary to-tertiary hover:from-primary/50 hover:to-tertiary/50">
								<UserPlus2Icon className="w-4 h-4 mr-2" />
								Invite User
							</Button>
						</DialogTrigger>
						<DialogContent className="sm:max-w-md">
							<DialogHeader className="space-y-3">
								<div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
									<UserPlus2Icon className="h-6 w-6 text-primary" />
								</div>
								<DialogTitle className="text-center text-xl">
									Invite New Member
								</DialogTitle>
								<DialogDescription className="text-center">
									Search for users and send them an invitation to join your team
								</DialogDescription>
							</DialogHeader>

							<div className="space-y-6 py-4">
								<div className="space-y-2">
									<Label
										htmlFor="usernameOrEmail"
										className="text-sm font-medium"
									>
										Username or Email
									</Label>
									<div className="relative">
										<Input
											id="usernameOrEmail"
											placeholder="Search by username or email..."
											value={invitee}
											onChange={(e) => setInvitee(e.target.value)}
											className="pl-10"
										/>
										<User className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
									</div>
								</div>

								<div className="space-y-2">
									<Label
										htmlFor="inviteMessage"
										className="text-sm font-medium"
									>
										Personal Message
									</Label>
									<Textarea
										id="inviteMessage"
										placeholder="Add a personal message to your invitation (optional)"
										value={message}
										onChange={(e) => setMessage(e.target.value)}
										className="min-h-[80px] resize-none"
									/>
								</div>

								{/* Search Results */}
								{inviteeSearch.length > 0 && (
									<div className="space-y-3">
										<Separator />

										{userSearch.isFetching && (
											<div className="flex items-center justify-center gap-2 py-8 text-muted-foreground">
												<RefreshCw className="h-4 w-4 animate-spin" />
												<span className="text-sm">Searching for users...</span>
											</div>
										)}

										{!userSearch.isFetching &&
											userSearch.data &&
											userSearch.data.length > 0 && (
												<div className="space-y-2">
													<h4 className="text-sm font-medium text-foreground">
														Search Results
													</h4>
													<div className="max-h-48 space-y-2 overflow-y-auto pr-2">
														{userSearch.data.map((user) => (
															<div
																key={user.id}
																className="group flex items-center justify-between rounded-lg border bg-card p-3 transition-colors hover:bg-accent/50"
															>
																<div className="flex items-center gap-3">
																	<Avatar className="h-9 w-9">
																		<AvatarImage
																			src={user.avatar_url}
																			alt={
																				user.name ?? user.username ?? user.email
																			}
																		/>
																		<AvatarFallback className="bg-primary/10 text-primary">
																			{(
																				user.name ??
																				user.username ??
																				user.email
																			)
																				?.charAt(0)
																				.toUpperCase()}
																		</AvatarFallback>
																	</Avatar>
																	<div className="min-w-0 flex-1">
																		<p className="truncate text-sm font-medium">
																			{user.name ?? user.username ?? user.email}
																		</p>
																		{user.username &&
																			user.email &&
																			user.name && (
																				<p className="truncate text-xs text-muted-foreground">
																					@{user.username}
																				</p>
																			)}
																	</div>
																</div>
																<Button
																	size="sm"
																	onClick={async () => {
																		try {
																			await backend.teamState.inviteUser(
																				appId,
																				user.id,
																				message,
																			);
																			toast.success(
																				`Invitation sent to ${user.name ?? user.username ?? user.email}!`,
																			);
																			setShowInviteDialog(false);
																			setInvitee("");
																			setMessage("");
																		} catch (error) {
																			console.error(error);
																			toast.error(
																				`Failed to send invite. Please try again.`,
																			);
																		}
																	}}
																	className="h-8 gap-1.5 text-xs"
																>
																	<Mail className="h-3 w-3" />
																	Invite
																</Button>
															</div>
														))}
													</div>
												</div>
											)}

										{!userSearch.isFetching &&
											inviteeSearch.length > 0 &&
											(!userSearch.data || userSearch.data.length === 0) && (
												<div className="flex flex-col items-center gap-2 py-8 text-center">
													<div className="flex h-12 w-12 items-center justify-center rounded-full bg-muted">
														<UserX className="h-6 w-6 text-muted-foreground" />
													</div>
													<div className="space-y-1">
														<p className="text-sm font-medium">
															No users found
														</p>
														<p className="text-xs text-muted-foreground">
															Try searching with a different username or email
														</p>
													</div>
												</div>
											)}
									</div>
								)}

								{inviteeSearch.length === 0 && (
									<div className="flex flex-col items-center gap-2 py-6 text-center text-muted-foreground">
										<Users className="h-8 w-8" />
										<p className="text-sm">Start typing to search for users</p>
									</div>
								)}
							</div>
						</DialogContent>
					</Dialog>
				</CardContent>
			</Card>

			{/* Invite Links Management - moved to bottom */}
			<Card className="md:col-span-2">
				<CardHeader>
					<div className="flex items-center justify-between">
						<div>
							<CardTitle className="flex items-center gap-2">
								<Link className="w-5 h-5" />
								Invite Links
							</CardTitle>
							<CardDescription>
								Create and manage invite links for your team
							</CardDescription>
						</div>
						<Dialog
							open={showCreateLinkDialog}
							onOpenChange={setShowCreateLinkDialog}
						>
							<DialogTrigger asChild>
								<Button>
									<Plus className="w-4 h-4 mr-2" />
									Create Link
								</Button>
							</DialogTrigger>
							<DialogContent className="sm:max-w-md">
								<DialogHeader className="space-y-3">
									<div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
										<Link className="h-6 w-6 text-primary" />
									</div>
									<DialogTitle className="text-center text-xl">
										Create Invite Link
									</DialogTitle>
									<DialogDescription className="text-center">
										Generate a shareable link with optional usage limits for
										your team
									</DialogDescription>
								</DialogHeader>

								<div className="space-y-6 py-4">
									<div className="space-y-2">
										<Label htmlFor="linkName" className="text-sm font-medium">
											Link Name
										</Label>
										<div className="relative">
											<Input
												id="linkName"
												placeholder="e.g., Marketing Team, Beta Users"
												value={newLinkName}
												onChange={(e) => setNewLinkName(e.target.value)}
												className="pl-10"
											/>
											<Settings className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
										</div>
									</div>

									<div className="space-y-2">
										<Label htmlFor="maxUses" className="text-sm font-medium">
											Maximum Uses
										</Label>
										<div className="relative">
											<Input
												id="maxUses"
												type="number"
												placeholder="Leave empty for unlimited uses"
												value={newLinkMaxUses}
												onChange={(e) => setNewLinkMaxUses(e.target.value)}
												className="pl-10"
											/>
											<Users className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
										</div>
										<p className="text-xs text-muted-foreground">
											Set a limit on how many people can use this link. Leave
											empty for unlimited access.
										</p>
									</div>
								</div>

								<DialogFooter className="gap-2 sm:gap-0">
									<Button
										variant="outline"
										onClick={() => {
											setShowCreateLinkDialog(false);
											setNewLinkName("");
											setNewLinkMaxUses("");
										}}
									>
										Cancel
									</Button>
									<Button
										onClick={createInviteLink}
										disabled={!newLinkName.trim()}
									>
										<Plus className="w-4 h-4 mr-2" />
										Create Link
									</Button>
								</DialogFooter>
							</DialogContent>
						</Dialog>
					</div>
				</CardHeader>
				<CardContent className="space-y-4">
					{(links.data?.length ?? 0) === 0 && (
						<EmptyState
							className="w-full flex grow min-w-fill flex-col max-w-full"
							title="No Invite Links"
							description="Create Invite Links to share your project"
							icons={[UsersIcon, LinkIcon, MailIcon]}
						/>
					)}
					{(links.data?.length ?? 0) !== 0 &&
						links.data?.map((link) => (
							<div key={link.id} className="border rounded-lg p-4 space-y-3">
								<div className="flex items-start justify-between">
									<div className="space-y-1">
										<h4 className="font-medium">{link.name}</h4>
										<div className="flex items-center gap-4 text-sm text-muted-foreground">
											<span className="flex items-center gap-1">
												<UserCheck className="w-4 h-4" />
												{link.count_joined} joined
											</span>
											{(link.max_uses ?? 0) > 0 && (
												<span>Max: {link.max_uses}</span>
											)}
											<span className="flex items-center gap-1">
												<Clock className="w-4 h-4" />
												{new Date(
													Date.parse(link.created_at),
												).toLocaleDateString()}
											</span>
										</div>
									</div>
									<DropdownMenu>
										<DropdownMenuTrigger asChild>
											<Button variant="ghost" size="sm">
												<MoreVertical className="w-4 h-4" />
											</Button>
										</DropdownMenuTrigger>
										<DropdownMenuContent align="end">
											<DropdownMenuItem
												onClick={() =>
													copyInviteLink(
														`https://${hub?.app ?? "app.flow-like.com"}/join?appId=${appId}&token=${link.token}`,
													)
												}
											>
												<ExternalLink className="w-4 h-4 mr-2" />
												Copy Web Link
											</DropdownMenuItem>
											<DropdownMenuItem
												onClick={() =>
													copyInviteLink(
														`flow-like://join?appId=${appId}&token=${link.token}`,
													)
												}
											>
												<Copy className="w-4 h-4 mr-2" />
												Copy Desktop Link
											</DropdownMenuItem>
											<AlertDialog>
												<AlertDialogTrigger asChild>
													<DropdownMenuItem
														onSelect={(e) => e.preventDefault()}
													>
														<Trash2 className="w-4 h-4 mr-2" />
														Delete
													</DropdownMenuItem>
												</AlertDialogTrigger>
												<AlertDialogContent>
													<AlertDialogHeader>
														<AlertDialogTitle>
															Delete Invite Link
														</AlertDialogTitle>
														<AlertDialogDescription>
															Are you sure you want to delete &quot;{link.name}
															&quot;? This action cannot be undone and the link
															will no longer work.
														</AlertDialogDescription>
													</AlertDialogHeader>
													<AlertDialogFooter>
														<AlertDialogCancel>Cancel</AlertDialogCancel>
														<AlertDialogAction
															onClick={() => deleteInviteLink(link.id)}
														>
															Delete
														</AlertDialogAction>
													</AlertDialogFooter>
												</AlertDialogContent>
											</AlertDialog>
										</DropdownMenuContent>
									</DropdownMenu>
								</div>
								<div className="flex gap-2">
									<Input
										value={`https://${hub?.app ?? "app.flow-like.com"}/join?appId=${appId}&token=${link.token}`}
										readOnly
										className="font-mono text-sm"
									/>
									<Button
										onClick={() =>
											copyInviteLink(
												`https://${hub?.app ?? "app.flow-like.com"}/join?appId=${appId}&token=${link.token}`,
											)
										}
										variant="outline"
										size="sm"
									>
										<Copy className="w-4 h-4" />
									</Button>
								</div>
								{link.max_uses > 0 && (
									<div className="flex items-center gap-2">
										<div className="flex-1 bg-muted rounded-full h-2">
											<div
												className="bg-primary h-2 rounded-full transition-all"
												style={{
													width: `${Math.min((link.count_joined / link.max_uses) * 100, 100)}%`,
												}}
											/>
										</div>
										<span className="text-xs text-muted-foreground">
											{link.count_joined}/{link.max_uses}
										</span>
									</div>
								)}
							</div>
						))}
				</CardContent>
			</Card>
		</div>
	);
}
