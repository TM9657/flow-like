"use client";
import {
	Badge,
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	type IInvite,
	Separator,
	useBackend,
	useInfiniteInvoke,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { AnimatePresence, motion } from "framer-motion";
import {
	BellIcon,
	Check,
	Clock,
	MailOpen,
	Sparkles,
	UserPlus,
	X,
} from "lucide-react";
import { useCallback } from "react";
import { toast } from "sonner";

const getTimeAgo = (date: string) => {
	const now = new Date();
	const inviteDate = new Date(date);
	const diffInDays = Math.floor(
		(now.getTime() - inviteDate.getTime()) / (1000 * 60 * 60 * 24),
	);

	if (diffInDays === 0) return "Today";
	if (diffInDays === 1) return "Yesterday";
	return `${diffInDays} days ago`;
};

export default function NotificationsPage() {
	const backend = useBackend();
	const {
		data: invitationPages,
		fetchNextPage,
		hasNextPage,
		refetch,
	} = useInfiniteInvoke(backend.teamState.getInvites, backend.teamState, []);
	const invitations: IInvite[] = invitationPages
		? invitationPages.pages.flat()
		: [];

	const handleInviteAction = useCallback(
		async (id: string, action: "accept" | "decline") => {
			try {
				if (action === "accept") {
					await backend.teamState.acceptInvite(id);
				} else {
					await backend.teamState.rejectInvite(id);
				}
				await refetch();
			} catch (error) {
				console.error(`Failed to ${action} invite:`, error);
				toast.error(`Failed to ${action} invite. Please try again later.`);
			}
		},
		[backend, refetch],
	);

	return (
		<main className="flex max-w-screen-xl w-full overflow-hidden flex-col p-6 gap-8 mx-auto flex-1 min-h-0">
			{/* Header Section */}
			<motion.div
				initial={{ opacity: 0, y: -20 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{ duration: 0.5 }}
				className="flex items-center justify-between"
			>
				<div className="flex items-center gap-4">
					<div className="relative">
						<motion.div
							animate={{ rotate: [0, 15, -15, 0] }}
							transition={{
								duration: 2,
								repeat: Number.POSITIVE_INFINITY,
								repeatDelay: 3,
							}}
						>
							<BellIcon className="w-10 h-10 text-primary" />
						</motion.div>
					</div>
					<div>
						<h1 className="text-4xl font-bold text-foreground relative">
							Notifications
						</h1>
						<p className="text-muted-foreground mt-1">
							{invitations.length > 0
								? `You have ${invitations.length} pending invitation${invitations.length > 1 ? "s" : ""}`
								: "All caught up! No new notifications"}
						</p>
					</div>
				</div>

				{invitations.length > 0 && (
					<motion.div
						initial={{ opacity: 0, x: 20 }}
						animate={{ opacity: 1, x: 0 }}
						transition={{ delay: 0.3 }}
						className="flex items-center gap-2 px-4 py-2 bg-primary/10 border border-primary/20 rounded-full"
					>
						<Sparkles className="w-4 h-4 text-primary" />
						<span className="text-sm font-medium text-primary">
							New invitations
						</span>
					</motion.div>
				)}
			</motion.div>

			<Separator className="bg-border" />

			{/* Notifications List */}
			<div className="flex-grow min-h-0 overflow-auto space-y-4 pr-2 py-2">
				<AnimatePresence mode="popLayout">
					{invitations.length === 0 ? (
						<motion.div
							initial={{ opacity: 0, scale: 0.9 }}
							animate={{ opacity: 1, scale: 1 }}
							exit={{ opacity: 0, scale: 0.9 }}
							transition={{ duration: 0.3 }}
						>
							<Card className="border-dashed border-2 border-border bg-muted/30">
								<CardContent className="py-12 text-center">
									<motion.div
										animate={{ y: [0, -10, 0] }}
										transition={{
											duration: 2,
											repeat: Number.POSITIVE_INFINITY,
										}}
										className="mb-4"
									>
										<MailOpen className="w-16 h-16 text-muted-foreground mx-auto" />
									</motion.div>
									<h3 className="text-xl font-semibold text-foreground mb-2">
										All clear!
									</h3>
									<p className="text-muted-foreground">
										No pending invitations at the moment.
									</p>
								</CardContent>
							</Card>
						</motion.div>
					) : (
						<>
							{invitations.map((invite, index) => (
								<InvitationCard
									key={invite.id}
									invite={invite}
									index={index}
									onAction={handleInviteAction}
								/>
							))}

							{hasNextPage && (
								<motion.div
									initial={{ opacity: 0, y: 20 }}
									animate={{ opacity: 1, y: 0 }}
									exit={{ opacity: 0, y: 20 }}
									transition={{ duration: 0.3 }}
									className="flex justify-center mt-4"
								>
									<Button
										variant="outline"
										onClick={() => fetchNextPage()}
										className="w-full max-w-md"
									>
										Load More Invitations
									</Button>
								</motion.div>
							)}
						</>
					)}
				</AnimatePresence>
			</div>
		</main>
	);
}

type InvitationCardProps = {
	invite: IInvite;
	index: number;
	onAction: (id: string, action: "accept" | "decline") => void;
};

function InvitationCard({
	invite,
	index,
	onAction,
}: Readonly<InvitationCardProps>) {
	const backend = useBackend();
	const userLookup = useInvoke(
		backend.userState.lookupUser,
		backend.userState,
		[invite.by_member_id],
	);

	const evaluatedName =
		userLookup.data?.name ??
		userLookup.data?.username ??
		userLookup.data?.email ??
		"Unknown User";

	return (
		<motion.div
			key={invite.id}
			layout
			initial={{ opacity: 0, y: 20, scale: 0.95 }}
			animate={{ opacity: 1, y: 0, scale: 1 }}
			exit={{ opacity: 0, x: -100, scale: 0.95 }}
			transition={{
				duration: 0.3,
				delay: index * 0.1,
				layout: { duration: 0.3 },
			}}
			whileHover={{ y: -2 }}
			className="group"
		>
			<Card className="transition-all duration-300 hover:shadow-xl hover:shadow-primary/10 border-border bg-card/80 backdrop-blur-sm">
				<CardHeader className="pb-3">
					<div className="flex items-start justify-between">
						<div className="flex items-start gap-3">
							<motion.div
								whileHover={{ rotate: 15 }}
								transition={{ duration: 0.2 }}
								className="mt-1 p-2 bg-primary/10 rounded-lg group-hover:bg-primary/20 transition-colors"
							>
								<UserPlus className="w-5 h-5 text-primary" />
							</motion.div>
							<div>
								<CardTitle className="text-xl font-semibold text-foreground group-hover:text-primary transition-colors">
									{invite.name ?? "New Invitation"}
								</CardTitle>
								<div className="flex items-center gap-2 mt-2">
									<span className="text-sm text-muted-foreground">
										Invited by
									</span>
									<Badge variant="secondary" className="font-medium">
										{evaluatedName}
									</Badge>
									<div className="flex items-center gap-1 text-xs text-muted-foreground">
										<Clock className="w-3 h-3" />
										{getTimeAgo(
											new Date(Date.parse(invite.created_at)).toLocaleString(),
										)}
									</div>
								</div>
							</div>
						</div>
						<div className="flex flex-col items-end gap-2">
							<Badge variant="outline" className="text-xs">
								Pending
							</Badge>
						</div>
					</div>
				</CardHeader>

				<CardContent className="pt-0">
					<p className="text-muted-foreground mb-6 leading-relaxed">
						{invite.message}
					</p>

					<div className="flex gap-3">
						<motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
							<Button
								onClick={() => onAction(invite.id, "accept")}
								className="bg-green-600 hover:bg-green-700 text-white shadow-lg shadow-green-600/20 hover:shadow-green-600/30 transition-all"
								size="sm"
							>
								<Check className="w-4 h-4 mr-2" />
								Accept
							</Button>
						</motion.div>

						<motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
							<Button
								onClick={() => onAction(invite.id, "decline")}
								variant="destructive"
								size="sm"
							>
								<X className="w-4 h-4 mr-2" />
								Decline
							</Button>
						</motion.div>
					</div>
				</CardContent>
			</Card>
		</motion.div>
	);
}
