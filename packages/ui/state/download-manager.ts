import { create, type StoreApi, type UseBoundStore } from "zustand";
import { Bit, type Download, type IBit } from "../lib";
import type { IBackendState } from "./backend-state";

// Ensure a single DownloadManager instance across navigations/HMR
declare global {
	// eslint-disable-next-line no-var
	var __FL_DL_MANAGER__: DownloadManager | undefined;
	// eslint-disable-next-line no-var
	var __FL_DL_STORE__: UseBoundStore<StoreApi<IDownloadManager>> | undefined;
}

function getManagerSingleton(): DownloadManager {
	globalThis.__FL_DL_MANAGER__ ??= new DownloadManager();
	return globalThis.__FL_DL_MANAGER__;
}

export class DownloadManager {
	private readonly downloads = new Map<string, Download>();

	public async download(
		backend: IBackendState,
		bit: IBit,
		cb?: (dl: Download) => void,
	): Promise<IBit[]> {
		const pack = Bit.fromObject(bit);
		pack.setBackend(backend);
		const bits = await pack.download((dl) => {
			this.downloads.set(bit.hash, dl);
			if (cb) {
				cb(dl);
			}
		});
		this.downloads.delete(bit.hash);
		return bits;
	}

	public async getParents() {
		const bits = [];
		for (const [_, dl] of this.downloads) {
			bits.push(dl.parent());
		}
		return bits;
	}

	public async getBits() {
		const bits = [];
		for (const [_, dl] of this.downloads) {
			bits.push(...dl.bits());
		}
		return bits;
	}

	public async getTotal(filter?: Set<string>) {
		let total = 0;
		for (const [key, dl] of this.downloads) {
			if (filter && !filter.has(key)) {
				continue;
			}
			total += dl.total().max;
		}
		return total;
	}

	public async getDownloaded(filter?: Set<string>) {
		let downloaded = 0;
		for (const [key, dl] of this.downloads) {
			if (filter && !filter.has(key)) {
				continue;
			}
			downloaded += dl.total().downloaded;
		}
		return downloaded;
	}

	public async getProgress(filter?: Set<string>) {
		const downloaded = await this.getDownloaded(filter);
		const total = await this.getTotal(filter);
		return total > 0 ? (downloaded / total) * 100 : 0;
	}

	public async getSpeed(filter?: Set<string>) {
		const status = {
			bytesPerSecond: 0,
			total: 0,
			progress: 0,
			max: 0,
		};

		for (const [key, dl] of this.downloads) {
			if (filter && !filter.has(key)) {
				continue;
			}
			const current = dl.measureSpeed();
			status.bytesPerSecond += current.bytesPerSecond;
			status.total += current.total;
			status.progress += current.progress;
			status.max += current.max;
		}
		status.progress = status.max > 0 ? (status.total / status.max) * 100 : 0;
		return status;
	}
}

interface IDownloadManager {
	manager: DownloadManager;
	backend: IBackendState;
	setDownloadBackend: (backend: IBackendState) => void;
	download: (bit: IBit, cb?: (dl: Download) => void) => Promise<IBit[]>;
}

const createStore = () =>
	create<IDownloadManager>((set, get) => ({
		manager: getManagerSingleton(),
		backend: {} as IBackendState,
		setDownloadBackend: (backend: IBackendState) => set({ backend }),
		download: async (bit: IBit, cb?: (dl: Download) => void) => {
			const { manager, backend } = get();

			// Check if the backend actually has functions to download bits
			if (!backend.bitState.downloadBit) {
				throw new Error("Backend does not support downloading bits.");
			}

			return await manager.download(backend, bit, cb);
		},
	}));

export const useDownloadManager = (globalThis.__FL_DL_STORE__ ??= createStore());
