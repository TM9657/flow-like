/** Hidden tabs and collapsed boards have no opportunity to paint. */
export class CanvasPaintBudget {
	private previousTime: number;
	private previousDrawable: boolean;
	private elapsed = 0;

	constructor(
		now: number,
		drawable: boolean,
		private readonly limitMs = 10_000,
	) {
		this.previousTime = now;
		this.previousDrawable = drawable;
	}

	expired(now: number, drawable: boolean) {
		if (this.previousDrawable && drawable)
			this.elapsed += Math.max(0, now - this.previousTime);
		this.previousTime = now;
		this.previousDrawable = drawable;
		return this.elapsed >= this.limitMs;
	}
}

function isDrawable(canvas: HTMLCanvasElement) {
	const document = canvas.ownerDocument;
	const view = document.defaultView;
	if (!view || document.hidden || !canvas.isConnected) return false;
	const bounds = canvas.getBoundingClientRect();
	if (
		bounds.width <= 0 ||
		bounds.height <= 0 ||
		bounds.right <= 0 ||
		bounds.bottom <= 0 ||
		bounds.left >= view.innerWidth ||
		bounds.top >= view.innerHeight
	)
		return false;
	if (typeof canvas.checkVisibility === "function")
		return canvas.checkVisibility({
			checkOpacity: true,
			checkVisibilityCSS: true,
		});
	const visibility = view.getComputedStyle(canvas).visibility;
	return visibility !== "hidden" && visibility !== "collapse";
}

/** Wait for a configured frame, with a deadline that only runs while drawable. */
export function waitForCanvasPaint({
	canvas,
	renderedFrames,
	initialFrames,
	onDrawable,
}: {
	canvas: HTMLCanvasElement;
	renderedFrames(): number;
	initialFrames: number;
	onDrawable(): void;
}) {
	const document = canvas.ownerDocument;
	const view = document.defaultView;
	if (!view) throw new Error("The native canvas has no browser window.");
	let cancel = () => {};
	const promise = new Promise<void>((resolve, reject) => {
		let finished = false;
		let frame = 0;
		let timer = 0;
		let wasDrawable = false;
		let requiredFrames = initialFrames;
		const budget = new CanvasPaintBudget(view.performance.now(), false);
		const finish = (error?: Error) => {
			if (finished) return;
			finished = true;
			view.cancelAnimationFrame(frame);
			view.clearTimeout(timer);
			document.removeEventListener("visibilitychange", check);
			if (error) reject(error);
			else resolve();
		};
		const check = () => {
			if (finished) return;
			try {
				const drawable = isDrawable(canvas);
				const expired = budget.expired(view.performance.now(), drawable);
				if (drawable && !wasDrawable) {
					// A hidden or collapsed surface may have painted at its old size.
					requiredFrames = Math.max(requiredFrames, renderedFrames());
					onDrawable();
				}
				wasDrawable = drawable;
				if (drawable && renderedFrames() > requiredFrames) return finish();
				if (expired)
					finish(new Error("The native canvas did not paint in time."));
			} catch (error) {
				finish(error instanceof Error ? error : new Error(String(error)));
			}
		};
		const nextFrame = () => {
			check();
			if (!finished) frame = view.requestAnimationFrame(nextFrame);
		};
		const watchdog = () => {
			check();
			if (!finished) timer = view.setTimeout(watchdog, 250);
		};
		cancel = () => finish();
		document.addEventListener("visibilitychange", check);
		frame = view.requestAnimationFrame(nextFrame);
		// A visible renderer that stops scheduling frames must still fail normally.
		timer = view.setTimeout(watchdog, 250);
	});
	return { promise, cancel: () => cancel() };
}
