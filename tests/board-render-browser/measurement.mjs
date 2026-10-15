export function percentile(values, fraction) {
	if (values.length === 0) return null;
	const sorted = [...values].sort((a, b) => a - b);
	const position = (sorted.length - 1) * fraction;
	const lower = Math.floor(position);
	return (
		sorted[lower] +
		(sorted[Math.ceil(position)] - sorted[lower]) * (position - lower)
	);
}

export function summarizeFrames(frameTimesMs, refreshIntervalMs) {
	const missedFrames = frameTimesMs.reduce(
		(total, elapsed) =>
			total + Math.max(0, Math.round(elapsed / refreshIntervalMs) - 1),
		0,
	);
	return {
		frameCount: frameTimesMs.length,
		p50Ms: percentile(frameTimesMs, 0.5),
		p95Ms: percentile(frameTimesMs, 0.95),
		p99Ms: percentile(frameTimesMs, 0.99),
		maxMs: frameTimesMs.length ? Math.max(...frameTimesMs) : null,
		refreshIntervalMs,
		missedFrames,
		missedFrameRatio:
			missedFrames / Math.max(1, frameTimesMs.length + missedFrames),
	};
}

// Calibrate on an empty foreground page, before mounting either renderer.
export async function calibrateRefreshRate(durationMs = 1200) {
	const result = await measureScenario({
		name: "refresh-calibration",
		durationMs,
	});
	const intervalMs = percentile(result.frameTimesMs, 0.5);
	return {
		intervalMs,
		estimatedHz: intervalMs ? 1000 / intervalMs : null,
		...result,
	};
}

/**
 * @param {{ name: string, durationMs: number, step?: (elapsedMs: number, deltaMs: number, frameIndex: number) => void, signal?: AbortSignal }} options
 */
export async function measureScenario({ name, durationMs, step, signal }) {
	if (!(durationMs > 0))
		throw new Error("A positive measurement duration is required");
	const frameTimesMs = [];
	const longTasks = [];
	const visibilityChanges = [];
	const initialVisibility = document.visibilityState;
	const onVisibility = () =>
		visibilityChanges.push({
			time: performance.now(),
			state: document.visibilityState,
		});
	document.addEventListener("visibilitychange", onVisibility);
	let observer;
	if (
		typeof PerformanceObserver !== "undefined" &&
		PerformanceObserver.supportedEntryTypes.includes("longtask")
	) {
		observer = new PerformanceObserver((list) => {
			for (const entry of list.getEntries())
				longTasks.push({
					startTime: entry.startTime,
					duration: entry.duration,
				});
		});
		observer.observe({ type: "longtask" });
	}
	let startedAt;
	let endedAt;
	let previous;
	try {
		await new Promise((resolve, reject) => {
			const frame = (now) => {
				try {
					if (startedAt === undefined) startedAt = now;
					const delta = previous === undefined ? 0 : now - previous;
					if (previous !== undefined) frameTimesMs.push(delta);
					previous = now;
					const elapsed = now - startedAt;
					if (elapsed >= durationMs || signal?.aborted) {
						endedAt = now;
						resolve();
						return;
					}
					step?.(elapsed, delta, frameTimesMs.length);
					requestAnimationFrame(frame);
				} catch (error) {
					reject(error);
				}
			};
			requestAnimationFrame(frame);
		});
	} finally {
		if (observer) {
			for (const entry of observer.takeRecords())
				longTasks.push({
					startTime: entry.startTime,
					duration: entry.duration,
				});
			observer.disconnect();
		}
		document.removeEventListener("visibilitychange", onVisibility);
	}
	const interval = globalThis.__boardBenchmarkRefreshIntervalMs ?? 1000 / 60;
	const measuredLongTasks = longTasks.filter(
		(entry) =>
			entry.startTime < endedAt && entry.startTime + entry.duration > startedAt,
	);
	return {
		name,
		requestedDurationMs: durationMs,
		durationMs: endedAt - startedAt,
		startedAt,
		endedAt,
		frameTimesMs,
		...summarizeFrames(frameTimesMs, interval),
		longTaskObserverSupported: Boolean(observer),
		longTasks: measuredLongTasks,
		longTaskTotalMs: measuredLongTasks.reduce(
			(sum, entry) =>
				sum +
				Math.max(
					0,
					Math.min(entry.startTime + entry.duration, endedAt) -
						Math.max(entry.startTime, startedAt),
				),
			0,
		),
		initialVisibility,
		visibilityChanges,
	};
}

// Event dispatch to the next rAF is a responsiveness proxy, not presentation latency.
export function startInputMeasurement(name = "pointer-input") {
	const controller = new AbortController();
	const pointerEventToRafMs = [];
	const pointerDispatchToRafMs = [];
	let pointerEvents = 0;
	const onPointer = (event) => {
		const dispatchedAt = performance.now();
		const timestamp =
			event.timeStamp > performance.timeOrigin
				? event.timeStamp - performance.timeOrigin
				: event.timeStamp;
		pointerEvents++;
		requestAnimationFrame(() => {
			const now = performance.now();
			pointerDispatchToRafMs.push(Math.max(0, now - dispatchedAt));
			pointerEventToRafMs.push(Math.max(0, now - timestamp));
		});
	};
	document.addEventListener("pointermove", onPointer, { capture: true });
	const frames = measureScenario({
		name,
		durationMs: 60000,
		signal: controller.signal,
	});
	return async () => {
		document.removeEventListener("pointermove", onPointer, { capture: true });
		controller.abort();
		const result = await frames;
		return {
			...result,
			pointerEvents,
			pointerEventToRafMs,
			pointerDispatchToRafMs,
			pointerEventToRafP50Ms: percentile(pointerEventToRafMs, 0.5),
			pointerEventToRafP95Ms: percentile(pointerEventToRafMs, 0.95),
			pointerDispatchToRafP95Ms: percentile(pointerDispatchToRafMs, 0.95),
			latencySemantics:
				"Synthetic pointer event timestamp to next requestAnimationFrame callback; this does not measure presented pixels or hardware input latency.",
		};
	};
}
