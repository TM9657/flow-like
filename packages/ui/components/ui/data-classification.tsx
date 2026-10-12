"use client";

import { useTranslation } from "@flow-like/locales";
import { Fingerprint } from "lucide-react";
import { useId, useMemo } from "react";
import {
	DATA_CLASSIFICATIONS,
	type DataClassification,
	type IDataSensitivity,
	isEmptyDataSensitivity,
	parseDataClassification,
} from "../../lib/data-classification";
import { cn } from "../../lib/utils";
import { Badge } from "./badge";
import { Label } from "./label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./select";
import { Switch } from "./switch";

const BADGE_TONE: Record<DataClassification, string> = {
	public: "border-border bg-muted text-muted-foreground",
	internal: "border-info-line bg-info-bg text-info",
	confidential: "border-warning-line bg-warning-bg text-warning",
	restricted: "border-destructive/40 bg-destructive/10 text-destructive",
};

const DOT_TONE: Record<DataClassification, string> = {
	public: "bg-muted-foreground",
	internal: "bg-info-solid",
	confidential: "bg-warning-solid",
	restricted: "bg-destructive",
};

/** Radix Select reserves the empty string, so "not classified" needs a value of its own. */
const UNCLASSIFIED = "unclassified";

export interface DataClassificationCopy {
	label: string;
	description: string;
}

/** Label and one-line description of every level, plus `unset` for unlabelled data. */
export function useDataClassificationCopy(): Record<
	DataClassification | "unset",
	DataClassificationCopy
> {
	const { t } = useTranslation("common");
	return useMemo(
		() => ({
			unset: {
				label: t("dataClassificationUnset", "Not classified"),
				description: t(
					"dataClassificationUnsetHint",
					"Pick a level so people know how carefully to handle this data.",
				),
			},
			public: {
				label: t("dataClassificationPublic", "Public"),
				description: t(
					"dataClassificationPublicHint",
					"Fine to share with anyone, inside or outside your organization.",
				),
			},
			internal: {
				label: t("dataClassificationInternal", "Internal"),
				description: t(
					"dataClassificationInternalHint",
					"For members of your organization only.",
				),
			},
			confidential: {
				label: t("dataClassificationConfidential", "Confidential"),
				description: t(
					"dataClassificationConfidentialHint",
					"Only for the people who need it. A leak causes harm.",
				),
			},
			restricted: {
				label: t("dataClassificationRestricted", "Restricted"),
				description: t(
					"dataClassificationRestrictedHint",
					"The most sensitive data. A leak causes severe harm.",
				),
			},
		}),
		[t],
	);
}

export function DataClassificationDot({
	classification,
	className,
}: Readonly<{ classification: DataClassification; className?: string }>) {
	return (
		<span
			aria-hidden
			className={cn(
				"inline-block size-2 shrink-0 rounded-full",
				DOT_TONE[classification],
				className,
			)}
		/>
	);
}

export function DataClassificationBadge({
	classification,
	className,
}: Readonly<{ classification: DataClassification; className?: string }>) {
	const copy = useDataClassificationCopy()[classification];
	return (
		<Badge
			variant="outline"
			title={copy.description}
			className={cn(BADGE_TONE[classification], className)}
		>
			{copy.label}
		</Badge>
	);
}

export function PiiBadge({ className }: Readonly<{ className?: string }>) {
	const { t } = useTranslation("common");
	return (
		<Badge
			variant="outline"
			title={t("piiLabel", "Contains personal data")}
			className={cn("bg-background text-foreground", className)}
		>
			<Fingerprint />
			{t("piiBadge", "PII")}
		</Badge>
	);
}

/** The classification and PII badges of a label; renders nothing for an empty one. */
export function DataSensitivityBadges({
	value,
	className,
	badgeClassName,
}: Readonly<{
	value: IDataSensitivity | undefined;
	className?: string;
	badgeClassName?: string;
}>) {
	if (!value || isEmptyDataSensitivity(value)) return null;
	return (
		<span className={cn("inline-flex flex-wrap items-center gap-1", className)}>
			{value.classification && (
				<DataClassificationBadge
					classification={value.classification}
					className={badgeClassName}
				/>
			)}
			{value.pii && <PiiBadge className={badgeClassName} />}
		</span>
	);
}

export interface DataClassificationSelectProps {
	value: DataClassification | undefined;
	onChange: (next: DataClassification | undefined) => void;
	disabled?: boolean;
	id?: string;
	className?: string;
	"aria-label"?: string;
	"aria-describedby"?: string;
}

export function DataClassificationSelect({
	value,
	onChange,
	disabled,
	id,
	className,
	...aria
}: Readonly<DataClassificationSelectProps>) {
	const copy = useDataClassificationCopy();
	return (
		<Select
			value={value ?? UNCLASSIFIED}
			onValueChange={(next) => onChange(parseDataClassification(next))}
			disabled={disabled}
		>
			<SelectTrigger
				id={id}
				className={cn("w-56 max-w-full", className)}
				{...aria}
			>
				<SelectValue />
			</SelectTrigger>
			<SelectContent>
				<SelectItem value={UNCLASSIFIED}>
					<span
						aria-hidden
						className="inline-block size-2 shrink-0 rounded-full border border-muted-foreground/60"
					/>
					{copy.unset.label}
				</SelectItem>
				{DATA_CLASSIFICATIONS.map((level) => (
					<SelectItem key={level} value={level}>
						<DataClassificationDot classification={level} />
						{copy[level].label}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}

/** Classification select and PII switch for any labelled resource. */
export function DataSensitivityFields({
	value,
	onChange,
	disabled,
	className,
}: Readonly<{
	value: IDataSensitivity;
	onChange: (next: IDataSensitivity) => void;
	disabled?: boolean;
	className?: string;
}>) {
	const { t } = useTranslation("common");
	const copy = useDataClassificationCopy();
	const ids = useId();
	return (
		<div className={cn("grid gap-4 sm:grid-cols-2", className)}>
			<div className="flex min-w-0 flex-col gap-1.5">
				<Label htmlFor={`${ids}-classification`}>
					{t("classification", "Classification")}
				</Label>
				<DataClassificationSelect
					id={`${ids}-classification`}
					aria-describedby={`${ids}-classification-hint`}
					value={value.classification}
					onChange={(classification) => onChange({ ...value, classification })}
					disabled={disabled}
					className="w-full"
				/>
				<p
					id={`${ids}-classification-hint`}
					className="text-xs text-muted-foreground"
				>
					{copy[value.classification ?? "unset"].description}
				</p>
			</div>
			<div className="flex min-w-0 items-start justify-between gap-3 rounded-md border px-3 py-2.5">
				<div className="min-w-0 space-y-1">
					<Label htmlFor={`${ids}-pii`}>
						<Fingerprint className="size-3.5 text-muted-foreground" />
						{t("piiLabel", "Contains personal data")}
					</Label>
					<p id={`${ids}-pii-hint`} className="text-xs text-muted-foreground">
						{t(
							"piiHint",
							"Names, contact details, IDs or anything else that identifies a person.",
						)}
					</p>
				</div>
				<Switch
					id={`${ids}-pii`}
					aria-describedby={`${ids}-pii-hint`}
					checked={value.pii === true}
					onCheckedChange={(pii) => onChange({ ...value, pii })}
					disabled={disabled}
				/>
			</div>
		</div>
	);
}
