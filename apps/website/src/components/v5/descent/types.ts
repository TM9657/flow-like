export interface DescentApp {
	name: string;
	category: string;
	ui: Record<string, string>;
}

export interface DescentScenario {
	label: string;
	title: string;
	detail: string;
	route: string[];
}

export interface DescentArrivals {
	approved: string;
	briefingReady: string;
	scheduled: string;
	live: string;
	ready: string;
}

export interface DescentData {
	/** Section copy plus the board's titles and pin labels (BOARD_STRING_KEYS). */
	strings: Record<string, string>;
	/** 8 apps, in dock order. */
	apps: DescentApp[];
	/** End states the target windows show when work arrives. */
	arrivals: DescentArrivals;
	/** 3 scenarios. */
	scenarios: DescentScenario[];
	/** App index → scenario index. */
	appFlowIndex: number[];
	/** Scenario index → app index. */
	scenarioAppIndex: number[];
}

export type DescentTheme = "light" | "dark";

export type DescentScenarioEvent =
	/** `duration` is the seconds until "complete". */
	| { type: "start"; scenario: number; duration: number }
	/** `step` indexes `scenarios[scenario].route`: the pill to light. */
	| { type: "hop"; scenario: number; step: number }
	/** `app` is the window that just switched to its end state. */
	| { type: "arrive"; scenario: number; app: number }
	| { type: "complete"; scenario: number };

/** CSS px kept free of 3D content, measured from the stage's top and bottom edges. */
export interface DescentInsets {
	top: number;
	bottom: number;
}

export interface DescentSafeArea {
	/** At rest: under the sub-head, above the scroll cue and the dock. */
	surface: DescentInsets;
	/** Once the cue and the dock have gone and before the copy band shows: above the band's foot. */
	open: DescentInsets;
	/** With the copy band: above it, with air between the picture and the copy. */
	beneath: DescentInsets;
}

export interface DescentOptions {
	reducedMotion: boolean;
	onSelectApp?: (index: number) => void;
	/** Fires when the pointer comes to rest on an app's window (its index), or leaves the windows (-1). */
	onHoverApp?: (index: number) => void;
	/** Fires on the frame the rendered image turns from the porcelain world to the dark operating layer, and back. */
	onTheme?: (theme: DescentTheme) => void;
	onScenarioEvent?: (event: DescentScenarioEvent) => void;
}

export interface DescentScene {
	ready: Promise<void>;
	setProgress(p: number, immediate?: boolean): void;
	setIntro(v: number, immediate?: boolean): void;
	setScenario(index: number): void;
	selectApp(index: number): void;
	/** Rings app `index`'s window while keyboard focus is on its control (-1: none). */
	focusApp(index: number): void;
	setTime(t: number): void;
	setScenarioClock(t: number): void;
	snap(): void;
	render(): void;
	/** Frame the windows (surface) and the board (beneath) inside these insets. */
	setSafeArea(area: DescentSafeArea): void;
	/** Restart the current scenario from its first hop. */
	replayScenario(): void;
	resize(): void;
	pause(): void;
	resume(): void;
	dispose(): void;
	info(): Record<string, unknown>;
}

/** Returns null when WebGL2 or float colour buffers are unavailable. */
export type CreateScene = (
	container: HTMLElement,
	data: DescentData,
	options: DescentOptions,
) => DescentScene | null;
