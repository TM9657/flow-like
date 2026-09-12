"use client";

import { useTranslation } from "@flow-like/locales";
import { ChevronDownIcon } from "lucide-react";
import { useMemo } from "react";
import {
	APPEARANCE_COLORS,
	APPEARANCE_EFFECTS,
	type AppearanceColorKey,
	type AppearanceEffectId,
	type AppearanceMode,
	type AppearanceState,
	contrastRatio,
} from "../../../lib/appearance/appearance-theme";
import { cn } from "../../../lib/utils";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../../ui/collapsible";
import { Label } from "../../ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../ui/select";
import { Slider } from "../../ui/slider";
import { Switch } from "../../ui/switch";

const FONT_STACKS = [
	'"Inter", ui-sans-serif, system-ui, sans-serif',
	'"Inter Tight", "Inter", ui-sans-serif, sans-serif',
	'"JetBrains Mono", ui-monospace, monospace',
	'Georgia, "Times New Roman", serif',
];

export function AppearanceRail({
	state,
	mode,
	onChange,
}: Readonly<{
	state: AppearanceState;
	mode: AppearanceMode;
	onChange: (next: AppearanceState) => void;
}>) {
	const { t } = useTranslation("flow");

	const colorLabels: Record<AppearanceColorKey, string> = useMemo(
		() => ({
			background: t("appearanceColorBackground", "Page background"),
			foreground: t("appearanceColorForeground", "Text"),
			card: t("appearanceColorCard", "Surface"),
			border: t("appearanceColorBorder", "Borders"),
			mutedForeground: t("appearanceColorMuted", "Secondary text"),
			primary: t("appearanceColorPrimary", "Accent"),
			primaryForeground: t(
				"appearanceColorPrimaryForeground",
				"Text on accent",
			),
		}),
		[t],
	);

	const effectCopy: Record<
		AppearanceEffectId,
		{ label: string; hint: string }
	> = useMemo(
		() => ({
			glass: {
				label: t("appearanceFxGlass", "Frosted surfaces"),
				hint: "backdrop-filter: blur(16px)",
			},
			elevation: {
				label: t("appearanceFxElevation", "Elevation"),
				hint: "box-shadow: 0 10px 28px -16px",
			},
			bevel: {
				label: t("appearanceFxBevel", "Inner highlight"),
				hint: "inset 0 1px 0 white / 8 %",
			},
			hairline: {
				label: t("appearanceFxHairline", "Accent hairline"),
				hint: "border-color: accent / 32 %",
			},
			wash: {
				label: t("appearanceFxWash", "Gradient wash"),
				hint: "radial-gradient on the app root",
			},
			grain: {
				label: t("appearanceFxGrain", "Film grain"),
				hint: "SVG turbulence, 5 % opacity",
			},
			glow: {
				label: t("appearanceFxGlow", "Accent glow"),
				hint: "glow under primary buttons",
			},
			lift: {
				label: t("appearanceFxLift", "Lift on hover"),
				hint: "translateY(-2px)",
			},
			press: {
				label: t("appearanceFxPress", "Press feedback"),
				hint: "active: scale(.972)",
			},
			entrance: {
				label: t("appearanceFxEntrance", "Entrance rise"),
				hint: "fade + 10px, staggered 60 ms",
			},
			sheen: {
				label: t("appearanceFxSheen", "Button sheen"),
				hint: "light sweeps the primary button",
			},
			aurora: {
				label: t("appearanceFxAurora", "Aurora drift"),
				hint: "blurred gradients, 22 s loop",
			},
			focus: {
				label: t("appearanceFxFocus", "Focus pulse"),
				hint: "ring expands on focus",
			},
			shimmer: {
				label: t("appearanceFxShimmer", "Loading shimmer"),
				hint: "placeholders sweep",
			},
			rows: {
				label: t("appearanceFxRows", "Row hover"),
				hint: "table rows tint on hover",
			},
		}),
		[t],
	);

	const palette = state.palette[mode];
	const surface = APPEARANCE_EFFECTS.filter((fx) => fx.group === "surface");
	const motion = APPEARANCE_EFFECTS.filter((fx) => fx.group === "motion");
	const countOn = (ids: AppearanceEffectId[]) =>
		ids.filter((id) => state.effects[id]).length;

	const setColor = (key: AppearanceColorKey, value: string) =>
		onChange({
			...state,
			palette: { ...state.palette, [mode]: { ...palette, [key]: value } },
		});

	const setEffect = (id: AppearanceEffectId, value: boolean) =>
		onChange({ ...state, effects: { ...state.effects, [id]: value } });

	return (
		<div className="flex flex-col">
			<RailGroup
				title={t("appearancePalette", "Palette")}
				meta={
					mode === "dark"
						? t("appearanceModeDark", "dark")
						: t("appearanceModeLight", "light")
				}
			>
				<div className="flex flex-col gap-2.5">
					{APPEARANCE_COLORS.map((field) => {
						const ratio = field.against
							? contrastRatio(palette[field.key], palette[field.against])
							: null;
						return (
							<div
								key={field.key}
								className="grid grid-cols-[22px_minmax(0,1fr)_auto] items-center gap-2.5"
							>
								<input
									type="color"
									id={`appearance-${field.key}`}
									value={palette[field.key]}
									onChange={(event) => setColor(field.key, event.target.value)}
									aria-label={colorLabels[field.key]}
									className="size-[22px] cursor-pointer appearance-none rounded-md border bg-transparent p-0 [&::-moz-color-swatch]:rounded [&::-moz-color-swatch]:border-0 [&::-webkit-color-swatch-wrapper]:p-0.5 [&::-webkit-color-swatch]:rounded [&::-webkit-color-swatch]:border-0"
								/>
								<Label
									htmlFor={`appearance-${field.key}`}
									className="flex min-w-0 flex-col items-start gap-0 font-normal"
								>
									<span className="text-xs">{colorLabels[field.key]}</span>
									<code className="truncate font-mono text-[10px] text-muted-foreground">
										{field.token}
									</code>
								</Label>
								{ratio !== null && (
									<span
										className={cn(
											"rounded border px-1.5 py-0.5 font-mono text-[10px] tabular-nums",
											ratio >= 4.5
												? "border-emerald-500/40 text-emerald-500"
												: ratio >= 3
													? "text-muted-foreground"
													: "border-amber-500/40 text-amber-500",
										)}
										title={t(
											"appearanceContrastTitle",
											"Contrast against {{token}}",
											{ token: field.against },
										)}
									>
										{ratio.toFixed(1)}:1
									</span>
								)}
							</div>
						);
					})}
				</div>
			</RailGroup>

			<RailGroup
				title={t("appearanceShapeAndType", "Shape & type")}
				meta="--radius"
			>
				<div className="flex flex-col gap-4">
					<SliderRow
						id="appearance-radius"
						label={t("appearanceRadius", "Corner radius")}
						value={`${state.radius}px`}
						min={0}
						max={24}
						step={1}
						current={state.radius}
						onValue={(value) => onChange({ ...state, radius: value })}
					/>
					<SliderRow
						id="appearance-spacing"
						label={t("appearanceDensity", "Spacing step")}
						value={`${state.spacing}px`}
						min={3}
						max={6}
						step={0.5}
						current={state.spacing}
						onValue={(value) => onChange({ ...state, spacing: value })}
					/>
					<div className="flex flex-col gap-2">
						<Label htmlFor="appearance-font" className="text-xs">
							{t("appearanceTypeface", "Typeface")}
						</Label>
						<Select
							value={state.fontSans}
							onValueChange={(value) => onChange({ ...state, fontSans: value })}
						>
							<SelectTrigger id="appearance-font" size="sm" className="text-xs">
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{FONT_STACKS.map((stack) => (
									<SelectItem key={stack} value={stack} className="text-xs">
										<span style={{ fontFamily: stack }}>
											{stack.split(",")[0].replaceAll('"', "")}
										</span>
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>
				</div>
			</RailGroup>

			<RailGroup
				title={t("appearanceSurfaces", "Surfaces")}
				meta={t("appearanceCountOn", "{{count}} on", {
					count: countOn(surface.map((fx) => fx.id)),
				})}
			>
				<EffectList
					ids={surface.map((fx) => fx.id)}
					copy={effectCopy}
					effects={state.effects}
					onToggle={setEffect}
				/>
			</RailGroup>

			<RailGroup
				title={t("appearanceMotion", "Motion")}
				meta={t("appearanceCountOn", "{{count}} on", {
					count: countOn(motion.map((fx) => fx.id)),
				})}
			>
				<div className="flex flex-col gap-4">
					<EffectList
						ids={motion.map((fx) => fx.id)}
						copy={effectCopy}
						effects={state.effects}
						onToggle={setEffect}
					/>
					<SliderRow
						id="appearance-pace"
						label={t("appearancePace", "Pace")}
						value={`${state.motion.toFixed(2)}×`}
						min={0.5}
						max={2}
						step={0.05}
						current={state.motion}
						onValue={(value) => onChange({ ...state, motion: value })}
					/>
					<p className="text-[11px] text-muted-foreground leading-relaxed">
						{t(
							"appearanceMotionNote",
							"Pace scales every duration this sheet writes. All of it is skipped for visitors whose system asks for reduced motion.",
						)}
					</p>
				</div>
			</RailGroup>
		</div>
	);
}

function RailGroup({
	title,
	meta,
	children,
}: Readonly<{ title: string; meta?: string; children: React.ReactNode }>) {
	return (
		<Collapsible defaultOpen className="border-b">
			<CollapsibleTrigger className="group flex w-full items-baseline gap-2 px-3.5 py-3 text-left">
				<h2 className="font-semibold text-[11px] uppercase tracking-wider">
					{title}
				</h2>
				{meta && (
					<span className="font-mono text-[10px] text-muted-foreground">
						{meta}
					</span>
				)}
				<span className="flex-1" />
				<ChevronDownIcon className="size-3.5 text-muted-foreground transition-transform group-data-[state=closed]:-rotate-90" />
			</CollapsibleTrigger>
			<CollapsibleContent className="px-3.5 pb-4">
				{children}
			</CollapsibleContent>
		</Collapsible>
	);
}

function SliderRow({
	id,
	label,
	value,
	min,
	max,
	step,
	current,
	onValue,
}: Readonly<{
	id: string;
	label: string;
	value: string;
	min: number;
	max: number;
	step: number;
	current: number;
	onValue: (value: number) => void;
}>) {
	return (
		<div className="flex flex-col gap-2">
			<div className="flex items-center justify-between">
				<Label htmlFor={id} className="text-xs">
					{label}
				</Label>
				<span className="font-mono text-[11px] text-muted-foreground tabular-nums">
					{value}
				</span>
			</div>
			<Slider
				id={id}
				min={min}
				max={max}
				step={step}
				value={[current]}
				onValueChange={([next]) => onValue(next ?? current)}
			/>
		</div>
	);
}

function EffectList({
	ids,
	copy,
	effects,
	onToggle,
}: Readonly<{
	ids: AppearanceEffectId[];
	copy: Record<AppearanceEffectId, { label: string; hint: string }>;
	effects: Record<AppearanceEffectId, boolean>;
	onToggle: (id: AppearanceEffectId, value: boolean) => void;
}>) {
	return (
		<div className="flex flex-col gap-0.5">
			{ids.map((id) => (
				<Label
					key={id}
					htmlFor={`appearance-fx-${id}`}
					className="grid grid-cols-[30px_minmax(0,1fr)] items-start gap-2.5 rounded-lg p-1.5 font-normal hover:bg-muted/60"
				>
					<Switch
						id={`appearance-fx-${id}`}
						checked={effects[id]}
						onCheckedChange={(value) => onToggle(id, value)}
					/>
					<span className="flex min-w-0 flex-col gap-0.5">
						<span className="text-xs">{copy[id].label}</span>
						<span className="font-mono text-[10px] text-muted-foreground leading-snug">
							{copy[id].hint}
						</span>
					</span>
				</Label>
			))}
		</div>
	);
}
