"use client";

import * as SliderPrimitive from "@radix-ui/react-slider";
import { useId } from "react";
import { Button } from "../../../ui/button";
import type { forecastBacktestRange } from "./forecast-backtest";
import { formatForecastTime } from "./forecast-data";

export function ForecastBacktestControls({
	range,
	offset,
	horizon,
	disabled,
	reason,
	running,
	onChange,
	onCommit,
}: {
	range: ReturnType<typeof forecastBacktestRange>;
	offset: number;
	horizon: number;
	disabled: boolean;
	reason?: string | null;
	running: boolean;
	onChange: (offset: number) => void;
	onCommit: (offset: number) => void;
}) {
	const labelId = useId();
	const descriptionId = useId();
	const time = (timestamp?: number) =>
		timestamp === undefined
			? "Unavailable"
			: `${formatForecastTime(timestamp, true)} UTC`;
	return (
		<section
			className="space-y-4 rounded-xl border bg-card p-4"
			aria-label="Historical forecast cutoff"
		>
			<div className="flex flex-wrap items-start justify-between gap-3">
				<div className="space-y-1">
					<p id={labelId} className="text-xs font-medium">
						Holdout forecast as of
					</p>
					<p className="text-sm font-semibold tabular-nums" aria-live="polite">
						{time(range.cutoff)}
					</p>
				</div>
				<Button
					type="button"
					size="sm"
					variant="outline"
					disabled={
						disabled || !range.available || (range.valid && offset === 0)
					}
					onClick={() => {
						onChange(0);
						onCommit(0);
					}}
				>
					Latest window
				</Button>
			</div>
			<SliderPrimitive.Root
				className="relative flex h-5 w-full touch-none select-none items-center data-[disabled]:opacity-50"
				min={0}
				max={Math.max(1, range.maxOffset)}
				step={1}
				value={[
					Math.max(0, Math.min(range.maxOffset, range.maxOffset - offset)),
				]}
				disabled={disabled || !range.available || range.maxOffset === 0}
				onValueChange={([value]) => onChange(range.maxOffset - value)}
				onValueCommit={([value]) => onCommit(range.maxOffset - value)}
			>
				<SliderPrimitive.Track className="relative h-1.5 grow overflow-hidden rounded-full bg-muted">
					<SliderPrimitive.Range className="absolute h-full bg-orange-500" />
				</SliderPrimitive.Track>
				<SliderPrimitive.Thumb
					aria-labelledby={labelId}
					aria-describedby={descriptionId}
					aria-valuetext={
						range.valid
							? `${time(range.cutoff)}, evaluating the next ${horizon} steps`
							: "Choose a valid historical cutoff"
					}
					className="block size-5 rounded-full border-2 border-orange-500 bg-background shadow-sm focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-orange-500/30"
				/>
			</SliderPrimitive.Root>
			<div className="flex flex-wrap justify-between gap-x-6 gap-y-1 text-[10px] text-muted-foreground">
				<span>Earlier · {time(range.oldest)}</span>
				<span>Latest · {time(range.latest)}</span>
			</div>
			<div
				id={descriptionId}
				className="space-y-1 text-xs leading-relaxed text-muted-foreground"
			>
				{range.valid && (
					<p>
						Compare {time(range.start)} to {time(range.end)} · {horizon} steps.
					</p>
				)}
				<p>
					{running
						? "Evaluating the selected window…"
						: reason ||
							(!range.available
								? "Run a forecast with enough history for the evaluation window and at least two earlier observations."
								: !range.valid
									? "The selected cutoff is outside the available history. Move the slider or choose Latest window."
									: "Move the cutoff and release to rerun the holdout. It uses history through that date; known future inputs keep their configured availability.")}
				</p>
				<p>Outlook continues from the latest observation.</p>
				{range.seriesCount > 1 && (
					<p>
						Each series uses the same step offset from its own end. Dates above
						are for the selected series.
					</p>
				)}
			</div>
		</section>
	);
}
