"use client";

import { useTranslation } from "@flow-like/locales";
import { Plus, X } from "lucide-react";
import type * as React from "react";
import { useState } from "react";
import { cn } from "../../lib/utils";
import { Button } from "./button";
import { Input } from "./input";

export interface AttributeTagsInputProps
	extends Pick<
		React.ComponentProps<"input">,
		"id" | "placeholder" | "maxLength" | "aria-label" | "aria-describedby"
	> {
	value: readonly string[];
	onChange: (next: string[]) => void;
	/** Attributes offered for one-click reuse; the ones already added are left out. */
	suggestions?: readonly string[];
	/** Leads the suggestion chips. Defaults to "Already in use:". */
	suggestionsLabel?: string;
	disabled?: boolean;
	className?: string;
}

/** Free-text attribute tags: type and add, remove chips, or pick a suggestion. */
export function AttributeTagsInput({
	value,
	onChange,
	suggestions = [],
	suggestionsLabel,
	disabled = false,
	className,
	...input
}: Readonly<AttributeTagsInputProps>) {
	const { t } = useTranslation("settings");
	const [draft, setDraft] = useState("");
	const available = suggestions.filter(
		(attribute) => !value.includes(attribute),
	);

	const add = (candidate: string) => {
		const attribute = candidate.trim();
		if (!attribute || value.includes(attribute)) return;
		onChange([...value, attribute]);
		setDraft("");
	};

	return (
		<div className={cn("flex flex-col gap-2", className)}>
			<div className="flex gap-2">
				<Input
					{...input}
					value={draft}
					placeholder={input.placeholder ?? t("egRegioneu", "e.g. region:eu")}
					className="flex-1 font-mono text-xs"
					disabled={disabled}
					onChange={(event) => setDraft(event.target.value)}
					onKeyDown={(event) => {
						if (event.key !== "Enter") return;
						event.preventDefault();
						add(draft);
					}}
				/>
				<Button
					type="button"
					variant="outline"
					size="sm"
					disabled={disabled || !draft.trim()}
					onClick={() => add(draft)}
				>
					{t("add", "Add")}
				</Button>
			</div>
			{value.length > 0 && (
				<div className="flex flex-wrap gap-1.5">
					{value.map((attribute) => (
						<span
							key={attribute}
							className="inline-flex items-center gap-1 rounded-md border bg-muted/50 pl-2 pr-1 py-0.5 font-mono text-xs"
						>
							{attribute}
							<button
								type="button"
								disabled={disabled}
								aria-label={t("removeAttribute", "Remove {{attribute}}", {
									attribute,
								})}
								className="text-muted-foreground hover:text-destructive disabled:pointer-events-none disabled:opacity-50"
								onClick={() =>
									onChange(value.filter((item) => item !== attribute))
								}
							>
								<X className="h-3 w-3" />
							</button>
						</span>
					))}
				</div>
			)}
			{!disabled && available.length > 0 && (
				<div className="flex flex-wrap items-center gap-1.5">
					<span className="text-xs text-muted-foreground">
						{suggestionsLabel ?? t("alreadyInUse", "Already in use:")}
					</span>
					{available.map((attribute) => (
						<button
							key={attribute}
							type="button"
							onClick={() => add(attribute)}
							className="inline-flex items-center gap-1 rounded-md border border-dashed px-2 py-0.5 font-mono text-xs text-muted-foreground hover:border-solid hover:border-primary hover:text-primary"
						>
							<Plus className="h-2.5 w-2.5" />
							{attribute}
						</button>
					))}
				</div>
			)}
		</div>
	);
}
