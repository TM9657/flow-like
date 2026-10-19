"use client";

import { Loader2, RefreshCw } from "lucide-react";
import { useId } from "react";
import { Button } from "../../../ui/button";
import { useSavedForecasts } from "./use-saved-forecast";

export function SavedForecastPicker({
	appId,
	userScoped = false,
	value = "",
	onChange,
	disabled = false,
}: {
	appId: string;
	userScoped?: boolean;
	value?: string;
	onChange: (id: string) => void;
	disabled?: boolean;
}) {
	const id = useId();
	const forecasts = useSavedForecasts(appId, userScoped);
	const items = forecasts.data ?? [];
	return (
		<div className="min-w-0 space-y-2">
			<label htmlFor={id} className="text-xs font-medium">
				Saved prediction
			</label>
			<div className="flex min-w-0 items-center gap-2">
				<select
					id={id}
					value={value}
					disabled={
						disabled || !appId || forecasts.isLoading || forecasts.isError
					}
					onChange={(event) => onChange(event.target.value)}
					className="h-9 min-w-0 flex-1 rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
				>
					<option value="">Choose a saved prediction</option>
					{value && !items.some((item) => item.id === value) && (
						<option value={value}>
							Selected prediction{forecasts.data ? " (unavailable)" : ""}
						</option>
					)}
					{items.map((item) => (
						<option key={item.id} value={item.id}>
							{item.name} · {item.horizon} steps
						</option>
					))}
				</select>
				<Button
					type="button"
					size="icon"
					variant="ghost"
					disabled={disabled || !appId || forecasts.isLoading}
					onClick={forecasts.refetch}
					aria-label="Refresh saved predictions"
				>
					{forecasts.isLoading ? (
						<Loader2 className="size-4 animate-spin" />
					) : (
						<RefreshCw className="size-4" />
					)}
				</Button>
			</div>
			<p
				className="text-xs leading-relaxed text-muted-foreground"
				role={forecasts.isError ? "status" : undefined}
			>
				{!appId
					? "Choose an app first."
					: forecasts.isError
						? "Saved predictions could not load. Check your access and try again."
						: forecasts.isLoading
							? "Loading saved predictions…"
							: items.length === 0
								? "Save a prediction in Data Studio to use it here."
								: userScoped
									? "Personal predictions are available only to their owner."
									: "Viewers need permission to read the prediction's source data."}
			</p>
		</div>
	);
}
