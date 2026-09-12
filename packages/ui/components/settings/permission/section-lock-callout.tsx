"use client";

import { KeyRoundIcon, LockIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "../../../lib/utils";
import { Badge } from "../../ui/badge";

export type LockKind = "permission" | "visibility";

/**
 * One vocabulary for "you can't do this yet", used everywhere the settings
 * surface says it.
 *
 * A key is a door this account can open: primary accent, and whatever renders
 * it owes the reader an action. A padlock belongs to someone else: neutral, and
 * the only honest next step is asking an admin. Before this existed the same
 * fact was drawn six different ways — a muted nav row, a plain badge, a blue
 * dashed callout, a grey dot, an amber panel and a destructive panel — and none
 * of them told the reader which of the two situations they were in.
 */
export function lockChrome(kind: LockKind) {
	return kind === "visibility"
		? {
				Icon: KeyRoundIcon,
				container: "border-primary/40 bg-primary/5",
				icon: "text-primary",
				badge: "border-primary/40 text-primary",
			}
		: {
				Icon: LockIcon,
				container: "border-border bg-muted/40",
				icon: "text-muted-foreground",
				badge: "border-border text-muted-foreground",
			};
}

export interface SectionLockCalloutProps {
	kind: LockKind;
	title?: string;
	children: ReactNode;
	action?: ReactNode;
	className?: string;
}

/** Inline callout for a gate that sits next to the thing it gates. */
export function SectionLockCallout({
	kind,
	title,
	children,
	action,
	className,
}: Readonly<SectionLockCalloutProps>) {
	const chrome = lockChrome(kind);
	return (
		<div
			className={cn(
				"flex flex-wrap items-center gap-3 rounded-md border border-dashed px-3 py-2.5 text-xs",
				chrome.container,
				className,
			)}
		>
			<chrome.Icon className={cn("h-3.5 w-3.5 shrink-0", chrome.icon)} />
			<span className="min-w-0 flex-1">
				{title && <span className="font-medium">{title} </span>}
				{children}
			</span>
			{action}
		</div>
	);
}

export interface LockBadgeProps {
	kind: LockKind;
	children: ReactNode;
	className?: string;
}

/** Compact form of the same vocabulary, for a table cell or a card row. */
export function LockBadge({
	kind,
	children,
	className,
}: Readonly<LockBadgeProps>) {
	const chrome = lockChrome(kind);
	return (
		<Badge
			variant="outline"
			className={cn("text-[10px]", chrome.badge, className)}
		>
			<chrome.Icon className="size-3" />
			{children}
		</Badge>
	);
}
