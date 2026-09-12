"use client";

import { useTranslation } from "@flow-like/locales";
import { KeyRoundIcon, LockIcon } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";
import type {
	INavigationItemState,
	SectionLock,
} from "../../../lib/config-nav";
import { cn } from "../../../lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../ui/tooltip";

/**
 * A locked section has to answer "can I do something about this?" before the
 * user clicks. A key means yes — the door is theirs to open, and the row stays
 * in the foreground with a primary accent. A lock means no — someone else
 * holds it, so the row recedes and the click only explains.
 *
 * Keeping both in the nav is deliberate: hiding them reads as "this app has no
 * Team", which sends people looking for a feature that is right there.
 */
export function lockGlyph(kind: SectionLock["kind"]) {
	return kind === "visibility" ? KeyRoundIcon : LockIcon;
}

const ROW_BASE =
	"w-full flex items-center gap-3 px-3 rounded-lg text-sm text-left transition-all";

const VARIANT_PADDING = {
	sidebar: "py-2",
	mobile: "min-h-11",
} as const;

export interface ConfigNavRowProps {
	item: INavigationItemState;
	appId: string;
	variant: keyof typeof VARIANT_PADDING;
	active?: boolean;
	/** Tooltips only make sense where there is a pointer. */
	withTooltip?: boolean;
	/** Rendered right-aligned on an unlocked row, e.g. the publication dot. */
	trailing?: ReactNode;
	onNavigate?: () => void;
	onLockClick: (item: INavigationItemState) => void;
}

export function ConfigNavRow({
	item,
	appId,
	variant,
	active,
	withTooltip,
	trailing,
	onNavigate,
	onLockClick,
}: Readonly<ConfigNavRowProps>) {
	const { t } = useTranslation("common");
	const Icon = item.icon;
	const padding = VARIANT_PADDING[variant];

	if (item.disabled) {
		const row = (
			<div
				className={cn(
					ROW_BASE,
					padding,
					"text-muted-foreground bg-muted/50 opacity-60 cursor-not-allowed",
				)}
				tabIndex={-1}
				aria-disabled="true"
			>
				<Icon className="w-4 h-4 shrink-0" />
				<span className="truncate">
					{variant === "mobile"
						? t("labelSoon", "{{label}} (soon)", { label: item.label })
						: item.label}
				</span>
			</div>
		);
		return withTooltip ? (
			<LockTooltip
				title={t("labelComingSoon", "{{label}} (Coming soon!)", {
					label: item.label,
				})}
				body={item.description}
			>
				{row}
			</LockTooltip>
		) : (
			row
		);
	}

	if (item.lock) {
		const openable = item.lock.kind === "visibility";
		const Glyph = lockGlyph(item.lock.kind);
		const row = (
			<button
				type="button"
				className={cn(
					ROW_BASE,
					padding,
					openable
						? "text-foreground/80 hover:bg-primary/10 hover:text-primary"
						: "text-muted-foreground/70 hover:bg-muted/60 cursor-default",
				)}
				onClick={() => onLockClick(item)}
			>
				<Icon className="w-4 h-4 shrink-0" />
				<span className="truncate">{item.label}</span>
				<span
					className={cn(
						"ml-auto grid place-items-center rounded-md shrink-0 size-5",
						openable
							? "bg-primary/10 text-primary"
							: "text-muted-foreground/70",
					)}
				>
					<Glyph className="size-3" />
				</span>
			</button>
		);
		return withTooltip ? (
			<LockTooltip
				title={
					openable
						? t("labelUnlockable", "{{label}} — you can unlock this", {
								label: item.label,
							})
						: t("labelLocked", "{{label}} (locked)", { label: item.label })
				}
				body={item.lock.reason}
			>
				{row}
			</LockTooltip>
		) : (
			row
		);
	}

	const row = (
		<Link
			href={`${item.href}?id=${appId}`}
			className={cn(
				ROW_BASE,
				padding,
				active
					? "bg-primary/10 text-primary font-medium"
					: "text-muted-foreground hover:bg-muted hover:text-foreground",
			)}
			onClick={onNavigate}
		>
			<Icon className="w-4 h-4 shrink-0" />
			<span className="truncate">{item.label}</span>
			{trailing}
		</Link>
	);
	return withTooltip ? (
		<LockTooltip title={item.label} body={item.description}>
			{row}
		</LockTooltip>
	) : (
		row
	);
}

function LockTooltip({
	title,
	body,
	children,
}: Readonly<{ title: string; body: string; children: ReactNode }>) {
	return (
		<Tooltip delayDuration={300}>
			<TooltipTrigger asChild>{children}</TooltipTrigger>
			<TooltipContent side="right" className="max-w-xs">
				<p className="font-bold">{title}</p>
				<p className="text-xs mt-1">{body}</p>
			</TooltipContent>
		</Tooltip>
	);
}
