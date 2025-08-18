import { type StoreApi, type UseBoundStore, create } from "zustand";
import { Bit, type Download, type IBit } from "../lib";
import type { IBackendState } from "./backend-state";

declare global {
	var __FL_DL_MANAGER__: DownloadManager | undefined;
	var __FL_DL_STORE__: UseBoundStore<StoreApi<IDownloadManager>> | undefined;
}

function getManagerSingleton(): DownloadManager {
	globalThis.__FL_DL_MANAGER__ ??= new DownloadManager();
	return globalThis.__FL_DL_MANAGER__!;
}

type ProgressListener = (dl: Download) => void;

export class DownloadManager {
	private readonly downloads = new Map<string, Download>();
	private readonly listeners = new Map<string, Set<ProgressListener>>();
	private readonly pending = new Map<string, Download>();
	private readonly lastEmit = new Map<string, number>();
	private readonly timers = new Map<string, number>();
	private readonly minIntervalMs = 200;
	private readonly queued = new Set<string>();
	private readonly inFlight = new Map<string, Promise<IBit[]>>();
	private beforeUnloadBound?: () => void;

	constructor() {
		if (typeof window !== "undefined") {
			this.beforeUnloadBound = () => this.cleanupAll();
			window.addEventListener("beforeunload", this.beforeUnloadBound, {
				once: true,
			});
		}
	}

	public onProgress(hash: string, listener: ProgressListener): () => void {
		const set = this.listeners.get(hash) ?? new Set<ProgressListener>();
		set.add(listener);
		this.listeners.set(hash, set);
		const latest = this.pending.get(hash) ?? this.downloads.get(hash);
		if (latest) {
			try {
				listener(latest);
			} catch {}
		}
		return () => {
			const s = this.listeners.get(hash);
			if (!s) return;
			s.delete(listener);
			if (s.size === 0) this.listeners.delete(hash);
		};
	}

	public isQueued(hash: string): boolean {
		return this.queued.has(hash);
	}

	public getLatestPct(hash: string): number | undefined {
		if (this.queued.has(hash)) return 0;
		const dl = this.pending.get(hash) ?? this.downloads.get(hash);
		return dl ? Math.round(dl.progress() * 100) : undefined;
	}

	private notify(hash: string, dl: Download) {
		const set = this.listeners.get(hash);
		if (!set?.size) return;
		for (const l of set) {
			try {
				l(dl);
			} catch {}
		}
	}

	private scheduleNotify(hash: string) {
		if (this.timers.has(hash)) return;
		const now = Date.now();
		const last = this.lastEmit.get(hash) ?? 0;
		const wait = Math.max(0, this.minIntervalMs - (now - last));
		const id = setTimeout(() => {
			this.timers.delete(hash);
			const latest = this.pending.get(hash);
			if (!latest) return;
			this.lastEmit.set(hash, Date.now());
			this.notify(hash, latest);
		}, wait) as unknown as number;
		this.timers.set(hash, id);
	}

	public async download(
		backend: IBackendState,
		bit: IBit,
		cb?: (dl: Download) => void,
	): Promise<IBit[]> {
		const key = bit.hash;

		const existing = this.inFlight.get(key);
		if (existing) {
			const off = cb ? this.onProgress(key, cb) : undefined;
			existing.finally(() => off?.());
			return existing;
		}

		const pack = Bit.fromObject(bit);
		pack.setBackend(backend);

		const off = cb ? this.onProgress(key, cb) : undefined;
		this.queued.add(key);

		const wrappedCb = (dl: Download) => {
			if (this.queued.has(key)) this.queued.delete(key);
			this.downloads.set(key, dl);
			this.pending.set(key, dl);

			const now = Date.now();
			const last = this.lastEmit.get(key) ?? 0;
			if (now - last >= this.minIntervalMs) {
				this.lastEmit.set(key, now);
				this.notify(key, dl);
			} else {
				this.scheduleNotify(key);
			}
		};

		const promise = pack
			.download(wrappedCb)
			.then((bits) => bits)
			.finally(() => {
				off?.();
				this.inFlight.delete(key);
				this.cleanupForKey(key);
			});

		this.inFlight.set(key, promise);
		return promise;
	}

	public async getParents() {
		const bits = [];
		for (const [, dl] of this.downloads) bits.push(dl.parent());
		return bits;
	}

	public async getBits() {
		const bits = [];
		for (const [, dl] of this.downloads) bits.push(...dl.bits());
		return bits;
	}

	public async getTotal(filter?: Set<string>) {
		let total = 0;
		for (const [key, dl] of this.downloads) {
			if (filter && !filter.has(key)) continue;
			total += dl.total().max;
		}
		return total;
	}

	public async getDownloaded(filter?: Set<string>) {
		let downloaded = 0;
		for (const [key, dl] of this.downloads) {
			if (filter && !filter.has(key)) continue;
			downloaded += dl.total().downloaded;
		}
		return downloaded;
	}

	public async getProgress(filter?: Set<string>) {
		const downloaded = await this.getDownloaded(filter);
		const total = await this.getTotal(filter);
		return total > 0 ? (downloaded / total) * 100 : 0;
	}

	private cleanupForKey(key: string) {
		const to = this.timers.get(key);
		if (to != null) clearTimeout(to);
		this.timers.delete(key);
		this.pending.delete(key);
		this.lastEmit.delete(key);
		this.downloads.delete(key);
		this.listeners.delete(key);
		this.queued.delete(key);
	}

	private cleanupAll() {
		for (const key of Array.from(this.timers.keys())) {
			const to = this.timers.get(key);
			if (to != null) clearTimeout(to);
		}
		this.timers.clear();
		this.pending.clear();
		this.lastEmit.clear();
		this.downloads.clear();
		this.listeners.clear();
		this.queued.clear();
		this.inFlight.clear();
		if (typeof window !== "undefined" && this.beforeUnloadBound) {
			window.removeEventListener("beforeunload", this.beforeUnloadBound);
		}
	}
}

interface IDownloadManager {
	manager: DownloadManager;
	backend: IBackendState;
	setDownloadBackend: (backend: IBackendState) => void;
	download: (bit: IBit, cb?: (dl: Download) => void) => Promise<IBit[]>;
	onProgress: (hash: string, cb: (dl: Download) => void) => () => void;
	isQueued: (hash: string) => boolean;
	getLatestPct: (hash: string) => number | undefined;
}

const createStore = () =>
	create<IDownloadManager>((set, get) => ({
		manager: getManagerSingleton(),
		backend: {} as IBackendState,
		setDownloadBackend: (backend: IBackendState) => set({ backend }),
		download: async (bit: IBit, cb?: (dl: Download) => void) => {
			const { manager, backend } = get();
			if (!backend.bitState.downloadBit)
				throw new Error("Backend does not support downloading bits.");
			return await manager.download(backend, bit, cb);
		},
		onProgress: (hash: string, cb: (dl: Download) => void) => {
			const { manager } = get();
			return manager.onProgress(hash, cb);
		},
		isQueued: (hash: string) => {
			const { manager } = get();
			return manager.isQueued(hash);
		},
		getLatestPct: (hash: string) => {
			const { manager } = get();
			return manager.getLatestPct(hash);
		},
	}));

export const useDownloadManager = (globalThis.__FL_DL_STORE__ ??=
	createStore());
