"use client";

import { useTranslation } from "@flow-like/locales";
import {
	BellIcon,
	BlocksIcon,
	ClockIcon,
	KeyIcon,
	LinkIcon,
	LockIcon,
	type LucideIcon,
	ShieldIcon,
	TriangleAlertIcon,
	UserPlusIcon,
	UsersIcon,
} from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useMemo, useState } from "react";
import { RolePermissions } from "../../../lib/permission/role-permission";
import { cn } from "../../../lib/utils";
import { Button } from "../../ui/button";
import { PermissionGate } from "../permission";
import { AppConnectionManagement } from "./app-connection-management";
import { InviteManagement, InviteUserDialog } from "./invite-managment";
import { TeamJoinManagement } from "./join-management";
import {
	CountPill,
	TEAM_ACTION_GRADIENT,
	TeamActionLock,
	type TeamSectionKey,
	useTeamOverview,
} from "./team-shared";
import { TechnicalUserManagement } from "./technical-user-management";
import { UserManagement } from "./user-managements";

interface RailItem {
	key: TeamSectionKey;
	label: string;
	icon: LucideIcon;
	count: number;
	attention?: boolean;
	/** The read behind the count did not land — show a glyph, never a zero. */
	locked?: boolean;
	lockReason?: string;
	/** A refusal carries a lock and names a permission; a failure does neither. */
	lockKind?: "permission" | "error";
}

export function TeamManagementPage() {
	const { t } = useTranslation("settings");
	const searchParams = useSearchParams();
	const appId = searchParams.get("id") ?? "";
	const [section, setSection] = useState<TeamSectionKey>("members");
	const overview = useTeamOverview(appId);
	const { access, unavailable, denied } = overview;

	const teamReason = t(
		"yourRoleCannotReadThisProjectsTeam",
		"Your role cannot read this project's team.",
	);
	const loadFailedReason = t(
		"thisCouldNotBeLoadedRightNow",
		"This could not be loaded right now. Check your connection and try again.",
	);
	// A count can be missing because the role may not have it or because the
	// read failed. Only the first is the reader's role, and only the first
	// deserves a lock and a "ask an admin" sentence.
	const gap = useMemo(
		() => (isDenied: boolean, deniedReason: string) => ({
			lockKind: (isDenied ? "permission" : "error") as "permission" | "error",
			lockReason: isDenied ? deniedReason : loadFailedReason,
		}),
		[loadFailedReason],
	);

	const rail = useMemo<readonly RailItem[][]>(
		() => [
			[
				{
					key: "members",
					label: t("people", "People"),
					icon: UsersIcon,
					count: overview.memberCount,
					locked: unavailable.members,
					...gap(denied.members, teamReason),
				},
				{
					key: "requests",
					label: t("joinRequests", "Join requests"),
					icon: ClockIcon,
					count: overview.joinRequestCount,
					attention: !unavailable.joinRequests && overview.joinRequestCount > 0,
					locked: unavailable.joinRequests,
					...gap(denied.joinRequests, access.adminReason),
				},
				{
					key: "invites",
					label: t("invitesLinks", "Invites & links"),
					icon: LinkIcon,
					count: overview.inviteLinkCount,
					locked: unavailable.inviteLinks,
					...gap(denied.inviteLinks, access.adminReason),
				},
			],
			[
				{
					key: "keys",
					label: t("apiKeys", "API keys"),
					icon: KeyIcon,
					count: overview.apiKeyCount,
					locked: unavailable.apiKeys,
					...gap(denied.apiKeys, access.adminReason),
				},
				{
					key: "connections",
					label: t("connectedApps", "Connected apps"),
					icon: BlocksIcon,
					count: overview.connectedAppCount + overview.pendingAppRequestCount,
					attention:
						!unavailable.connections && overview.pendingAppRequestCount > 0,
					locked: unavailable.connections,
					...gap(denied.connections, teamReason),
				},
			],
		],
		[overview, unavailable, denied, access.adminReason, teamReason, gap, t],
	);

	if (!appId) {
		return (
			<div className="p-10 text-center text-muted-foreground">
				{t("noAppSelected", "No app selected.")}
			</div>
		);
	}

	// "Needs review" sums two sources with different permissions. It only locks
	// when both are denied; with one readable the count is real but partial, so
	// the note has to say what is missing instead of claiming "nothing waiting".
	const needsReviewLocked = unavailable.joinRequests && unavailable.connections;
	const needsReviewParts = [
		unavailable.joinRequests
			? undefined
			: t("countPeople", {
					defaultValue_one: "{{count}} person",
					defaultValue_other: "{{count}} people",
					count: overview.joinRequestCount,
				}),
		unavailable.connections
			? undefined
			: t("countApps", {
					defaultValue_one: "{{count}} App",
					defaultValue_other: "{{count}} Apps",
					count: overview.pendingAppRequestCount,
				}),
	].filter((part): part is string => !!part);
	const needsReviewPartial =
		!needsReviewLocked && (unavailable.joinRequests || unavailable.connections);
	const needsReviewPartialDenied = denied.joinRequests || denied.connections;
	const needsReviewNote =
		overview.needsReviewCount > 0
			? needsReviewParts.join(" · ")
			: needsReviewPartial
				? needsReviewPartialDenied
					? t(
							"someOfThisIsHiddenForYourRole",
							"Some of this is hidden for your role",
						)
					: t("someOfThisCouldNotBeLoaded", "Some of this could not be loaded")
				: t("nothingWaiting", "Nothing waiting");
	const needsReviewGap = gap(
		denied.joinRequests && denied.connections,
		access.adminReason,
	);

	return (
		<div className="flex flex-col gap-5 pb-8">
			<header className="flex flex-wrap items-start justify-between gap-4">
				<div className="min-w-0">
					<h1 className="text-xl font-semibold tracking-tight">
						{t("access", "Access")}
					</h1>
					<p className="mt-0.5 text-sm text-muted-foreground">
						{t("whoAndWhatCanReachThisApp", "Who and what can reach this app.")}
					</p>
				</div>
				<div className="flex shrink-0 items-center gap-2">
					<Button variant="outline" asChild>
						<a href={`/library/config/roles?id=${appId}`}>
							<ShieldIcon className="size-4" />
							{t("roles", "Roles")}
						</a>
					</Button>
					<TeamActionLock
						locked={!access.canAdminister}
						reason={access.adminReason}
					>
						<InviteUserDialog
							appId={appId}
							trigger={
								<Button
									className={TEAM_ACTION_GRADIENT}
									disabled={!access.canAdminister}
								>
									<UserPlusIcon className="size-4" />
									{t("invitePeople", "Invite people")}
								</Button>
							}
						/>
					</TeamActionLock>
				</div>
			</header>

			<div className="grid grid-cols-2 gap-2.5 lg:grid-cols-4">
				<StatTile
					icon={UsersIcon}
					label={t("people", "People")}
					value={`${overview.memberCount}${overview.memberCountExact ? "" : "+"}`}
					note={
						unavailable.roles
							? t("roleSplitUnavailable", "Role split unavailable")
							: t(
									"editorcountCanEditViewercountReadonly",
									"{{editorCount}} can edit · {{viewerCount}} read-only",
									{
										editorCount: overview.editorCount,
										viewerCount: overview.viewerCount,
									},
								)
					}
					locked={unavailable.members}
					{...gap(denied.members, teamReason)}
					onClick={() => setSection("members")}
				/>
				<StatTile
					icon={BellIcon}
					label={t("needsReview", "Needs review")}
					value={overview.needsReviewCount}
					note={needsReviewNote}
					attention={!needsReviewLocked && overview.needsReviewCount > 0}
					locked={needsReviewLocked}
					{...needsReviewGap}
					onClick={() =>
						setSection(
							overview.joinRequestCount > 0 ? "requests" : "connections",
						)
					}
				/>
				<StatTile
					icon={KeyIcon}
					label={t("apiKeys", "API keys")}
					value={overview.apiKeyCount}
					note={
						overview.expiredKeyCount > 0
							? t("expiredkeycountExpired", "{{expiredKeyCount}} expired", {
									expiredKeyCount: overview.expiredKeyCount,
								})
							: t("allValid", "All valid")
					}
					locked={unavailable.apiKeys}
					{...gap(denied.apiKeys, access.adminReason)}
					onClick={() => setSection("keys")}
				/>
				<StatTile
					icon={BlocksIcon}
					label={t("connectedApps", "Connected apps")}
					value={overview.connectedAppCount}
					note={
						overview.pendingAppRequestCount > 0
							? t(
									"pendingapprequestcountAwaitingApproval",
									"{{pendingAppRequestCount}} awaiting approval",
									{ pendingAppRequestCount: overview.pendingAppRequestCount },
								)
							: t("noPendingRequests", "No pending requests")
					}
					locked={unavailable.connections}
					{...gap(denied.connections, teamReason)}
					onClick={() => setSection("connections")}
				/>
			</div>

			<div className="grid items-start gap-6 md:grid-cols-[212px_minmax(0,1fr)]">
				<nav
					aria-label={t("accessSections", "Access sections")}
					className="flex gap-1 overflow-x-auto md:sticky md:top-0 md:flex-col md:overflow-visible"
				>
					{rail.map((group, index) => (
						<div
							key={group[0].key}
							className="flex gap-1 md:flex-col md:gap-0.5"
						>
							<span
								className={cn(
									"hidden px-2.5 pb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground md:block",
									index === 0 ? "pt-0" : "pt-3",
								)}
							>
								{t("machines", {
									defaultValue_zero: "People",
									defaultValue_other: "Machines",
									count: index,
								})}
							</span>
							{group.map((item) => (
								<RailButton
									key={item.key}
									item={item}
									active={section === item.key}
									onSelect={() => setSection(item.key)}
								/>
							))}
						</div>
					))}
				</nav>

				<div className="min-w-0">
					{section === "members" && (
						<PermissionGate
							appId={appId}
							require={[RolePermissions.ReadTeam]}
							feature={t("people", "People")}
							description={t(
								"yourRoleCannotSeeWhoHasAccessToThisProject",
								"Your role cannot see who has access to this project.",
							)}
						>
							<UserManagement appId={appId} />
						</PermissionGate>
					)}
					{section === "requests" && (
						<PermissionGate
							appId={appId}
							require={[RolePermissions.Admin]}
							feature={t("joinRequests", "Join requests")}
							description={t(
								"onlyProjectAdminsCanReviewWhoAsksToJoin",
								"Only project admins can review who asks to join.",
							)}
						>
							<TeamJoinManagement appId={appId} />
						</PermissionGate>
					)}
					{section === "invites" && (
						<PermissionGate
							appId={appId}
							require={[RolePermissions.Admin]}
							feature={t("invitesLinks", "Invites & links")}
							description={t(
								"onlyProjectAdminsCanInvitePeopleOrManageInviteLinks",
								"Only project admins can invite people or manage invite links.",
							)}
						>
							<InviteManagement appId={appId} />
						</PermissionGate>
					)}
					{section === "keys" && (
						<PermissionGate
							appId={appId}
							require={[RolePermissions.Admin]}
							feature={t("apiKeys", "API keys")}
							description={t(
								"onlyProjectAdminsCanSeeOrIssueApiKeys",
								"Only project admins can see or issue API keys.",
							)}
						>
							<TechnicalUserManagement appId={appId} />
						</PermissionGate>
					)}
					{section === "connections" && (
						<PermissionGate
							appId={appId}
							require={[RolePermissions.ReadTeam]}
							feature={t("connectedApps", "Connected apps")}
							description={t(
								"yourRoleCannotSeeWhichAppsAreConnectedToThisOne",
								"Your role cannot see which apps are connected to this one.",
							)}
						>
							<AppConnectionManagement appId={appId} />
						</PermissionGate>
					)}
				</div>
			</div>
		</div>
	);
}

function RailButton({
	item,
	active,
	onSelect,
}: Readonly<{ item: RailItem; active: boolean; onSelect: () => void }>) {
	const Icon = item.icon;
	return (
		<button
			type="button"
			onClick={onSelect}
			aria-current={active}
			title={item.locked ? item.lockReason : undefined}
			className={cn(
				"flex shrink-0 items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
				active
					? "bg-primary/10 font-medium text-primary shadow-[inset_2px_0_0_var(--primary)]"
					: "text-muted-foreground hover:bg-muted hover:text-foreground",
			)}
		>
			<Icon className="size-4 shrink-0" />
			<span className="flex-1 whitespace-nowrap md:whitespace-normal">
				{item.label}
			</span>
			{item.locked ? (
				<GapGlyph
					kind={item.lockKind}
					className="size-3.5 shrink-0 text-muted-foreground"
				/>
			) : (
				<CountPill
					value={item.count}
					tone={item.attention ? "attention" : "neutral"}
				/>
			)}
		</button>
	);
}

/** A lock means "not for your role"; a warning means "this did not load". */
function GapGlyph({
	kind = "permission",
	className,
}: Readonly<{ kind?: "permission" | "error"; className?: string }>) {
	const Glyph = kind === "error" ? TriangleAlertIcon : LockIcon;
	return <Glyph className={className} />;
}

function StatTile({
	icon: Icon,
	label,
	value,
	note,
	attention = false,
	locked = false,
	lockReason,
	lockKind,
	onClick,
}: Readonly<{
	icon: LucideIcon;
	label: string;
	value: string | number;
	note: string;
	attention?: boolean;
	/** The source read did not land — the tile shows a dash and the reason. */
	locked?: boolean;
	lockReason?: string;
	lockKind?: "permission" | "error";
	onClick: () => void;
}>) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"flex flex-col items-start gap-0.5 rounded-xl border px-3.5 py-3 text-left transition-colors",
				attention
					? "border-primary/45 bg-primary/5 hover:border-primary hover:bg-primary/10"
					: "border-border/60 bg-card hover:border-border hover:bg-muted/40",
			)}
		>
			<span
				className={cn(
					"flex items-center gap-1.5 text-xs",
					attention ? "text-primary" : "text-muted-foreground",
				)}
			>
				<Icon className="size-3.5" />
				{label}
			</span>
			<span
				className={cn(
					"flex items-center gap-1.5 text-2xl font-semibold tabular-nums tracking-tight",
					attention && "text-primary",
					locked && "text-muted-foreground",
				)}
			>
				{locked ? (
					<>
						<GapGlyph kind={lockKind} className="size-4" />—
					</>
				) : (
					value
				)}
			</span>
			<span className="text-[11px] text-muted-foreground">
				{locked ? lockReason : note}
			</span>
		</button>
	);
}
