import type { ExcalidrawScene } from "./excalidraw-events";

function isRecord(value: unknown): value is Record<string, unknown> {
	return value !== null && typeof value === "object" && !Array.isArray(value);
}

/** Accept a .excalidraw document or a scene supplied by a workflow. */
export function parseExcalidrawScene(value: unknown): ExcalidrawScene {
	const data =
		typeof value === "string" && value.trim() ? JSON.parse(value) : value;
	if (data === undefined || data === null || data === "") {
		return {
			type: "excalidraw",
			version: 2,
			source: "flow-like",
			elements: [],
			appState: {},
			files: {},
		};
	}
	if (
		!isRecord(data) ||
		!Array.isArray(data.elements) ||
		!data.elements.every(isRecord)
	) {
		throw new Error("An Excalidraw scene must contain an elements array.");
	}
	if (data.type !== undefined && data.type !== "excalidraw") {
		throw new Error("This document is not an Excalidraw scene.");
	}
	if (
		(data.appState !== undefined && !isRecord(data.appState)) ||
		(data.files !== undefined &&
			(!isRecord(data.files) || !Object.values(data.files).every(isRecord)))
	) {
		throw new Error("Excalidraw appState and files must be objects.");
	}
	return {
		type: "excalidraw",
		version:
			typeof data.version === "number" && Number.isFinite(data.version)
				? data.version
				: 2,
		source: typeof data.source === "string" ? data.source : "flow-like",
		elements: data.elements,
		appState: (data.appState ?? {}) as ExcalidrawScene["appState"],
		files: (data.files ?? {}) as ExcalidrawScene["files"],
	};
}

/** Editor selection, viewport, tool preferences and theme do not edit a drawing. */
export function excalidrawContentKey(scene: ExcalidrawScene): string {
	return JSON.stringify({
		elements: scene.elements.map(
			({ version, versionNonce, updated, ...content }) => content,
		),
		files: scene.files,
		background: scene.appState.viewBackgroundColor,
		gridSize: scene.appState.gridSize,
		gridStep: scene.appState.gridStep,
		gridModeEnabled: scene.appState.gridModeEnabled,
	});
}

export interface ExcalidrawSaveOptions {
	readOnly: boolean;
	autoSave: boolean;
	autoSaveDelayMs: number;
	autoSaveMinChanges: number;
}

export function positiveNumber(
	value: unknown,
	fallback: number,
	minimum = 1,
): number {
	return typeof value === "number" && Number.isFinite(value) && value > 0
		? Math.max(minimum, Math.floor(value))
		: fallback;
}

interface SessionCallbacks {
	onDirty: (dirty: boolean) => void;
	onBusy: (busy: boolean) => void;
	onSave: (
		event: "save" | "autoSave",
		scene: ExcalidrawScene,
		changeCount: number,
	) => Promise<void>;
	onError: (event: "save" | "autoSave", error: unknown) => void;
}

/** Coordinates an idle timer and one save at a time for the current document. */
export class ExcalidrawSession {
	private scene?: ExcalidrawScene;
	private currentKey = "";
	private savedKey = "";
	private generation = 0;
	private changes = 0;
	private lastChange = 0;
	private dirty = false;
	private busy = false;
	private timer?: ReturnType<typeof setTimeout>;
	private options: ExcalidrawSaveOptions = {
		readOnly: false,
		autoSave: false,
		autoSaveDelayMs: 1500,
		autoSaveMinChanges: 1,
	};

	constructor(private callbacks: SessionCallbacks) {}

	configure(options: ExcalidrawSaveOptions) {
		this.options = options;
		this.schedule();
	}

	reset(scene?: ExcalidrawScene) {
		this.cancelTimer();
		this.generation++;
		this.scene = scene;
		this.currentKey = scene ? excalidrawContentKey(scene) : "";
		this.savedKey = this.currentKey;
		this.changes = 0;
		this.busy = false;
		this.callbacks.onBusy(false);
		this.setDirty(false);
	}

	change(scene: ExcalidrawScene): boolean {
		if (this.options.readOnly || !this.scene) return false;
		const key = excalidrawContentKey(scene);
		if (key === this.currentKey) return false;
		this.scene = scene;
		this.currentKey = key;
		this.changes++;
		this.lastChange = Date.now();
		this.setDirty(key !== this.savedKey);
		this.schedule();
		return true;
	}

	async save(event: "save" | "autoSave" = "save") {
		if (!this.scene || this.busy || this.options.readOnly) return;
		if (
			event === "autoSave" &&
			(!this.options.autoSave ||
				!this.dirty ||
				this.changes < this.options.autoSaveMinChanges)
		)
			return;
		this.cancelTimer();
		const generation = this.generation;
		const scene = this.scene;
		const key = this.currentKey;
		const count = this.changes;
		this.busy = true;
		this.callbacks.onBusy(true);
		let succeeded = false;
		try {
			await this.callbacks.onSave(event, scene, count);
			if (generation !== this.generation) return;
			succeeded = true;
			this.savedKey = key;
			this.changes = Math.max(0, this.changes - count);
			this.setDirty(this.currentKey !== this.savedKey);
		} catch (error) {
			if (generation === this.generation) this.callbacks.onError(event, error);
		} finally {
			if (generation === this.generation) {
				this.busy = false;
				this.callbacks.onBusy(false);
				// A failed snapshot is retried on another edit or explicit Save.
				if (succeeded || this.changes > count) this.schedule();
			}
		}
	}

	dispose() {
		this.cancelTimer();
		this.generation++;
		this.scene = undefined;
	}

	private setDirty(dirty: boolean) {
		if (dirty === this.dirty) return;
		this.dirty = dirty;
		this.callbacks.onDirty(dirty);
	}

	private cancelTimer() {
		if (this.timer !== undefined) clearTimeout(this.timer);
		this.timer = undefined;
	}

	private schedule() {
		this.cancelTimer();
		if (
			this.options.readOnly ||
			!this.options.autoSave ||
			this.busy ||
			!this.dirty ||
			this.changes < this.options.autoSaveMinChanges
		)
			return;
		const remaining = Math.max(
			0,
			this.options.autoSaveDelayMs - (Date.now() - this.lastChange),
		);
		this.timer = setTimeout(() => {
			void this.save("autoSave");
		}, remaining);
	}
}
