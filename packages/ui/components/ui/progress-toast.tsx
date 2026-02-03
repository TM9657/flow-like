"use client";

import { toast } from "sonner";
import { Progress } from "./progress";

export interface ProgressToastData {
	id: string;
	message: string;
	progress?: number;
	done: boolean;
	success: boolean;
}

const activeProgressToasts = new Map<string, ProgressToastData>();

function ProgressBarDescription({ progress }: { progress?: number }) {
	const isIndeterminate = progress === undefined;
	const clampedProgress =
		progress !== undefined ? Math.max(0, Math.min(100, progress)) : 0;

	return (
		<div className="flex items-center gap-2 w-full mt-1">
			{isIndeterminate ? (
				<div className="relative h-1.5 w-full overflow-hidden rounded-full bg-primary/20">
					<div className="absolute h-full w-1/3 animate-[indeterminate_1.5s_ease-in-out_infinite] rounded-full bg-primary" />
				</div>
			) : (
				<>
					<Progress value={clampedProgress} className="h-1.5 flex-1" />
					<span className="text-xs text-muted-foreground w-8 text-right">
						{clampedProgress}%
					</span>
				</>
			)}
		</div>
	);
}

export function showProgressToast(data: ProgressToastData): void {
	const { id, message, progress, done, success } = data;

	if (done) {
		activeProgressToasts.delete(id);
		if (success) {
			toast.success(message, { id, duration: 3000 });
		} else {
			toast.error(message, { id, duration: 3000 });
		}
	} else {
		activeProgressToasts.set(id, data);
		toast.loading(message, {
			id,
			duration: Number.POSITIVE_INFINITY,
			description: <ProgressBarDescription progress={progress} />,
		});
	}
}

export function finishAllProgressToasts(success = true): void {
	for (const [id, data] of activeProgressToasts) {
		if (success) {
			toast.success(data.message, { id, duration: 3000 });
		} else {
			toast.dismiss(id);
		}
	}
	activeProgressToasts.clear();
}

export function getActiveProgressToasts(): Map<string, ProgressToastData> {
	return activeProgressToasts;
}
