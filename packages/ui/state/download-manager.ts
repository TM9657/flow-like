import { create } from "zustand";
import { Bit, type Download, type IBit } from "../lib";
import type { IBackendState } from "./backend-state";

export class DownloadManager {
	private readonly downloads = new Map<string, Download>();

	public async download(backend: IBackendState, bit: IBit): Promise<IBit[]> {
		const pack = Bit.fromObject(bit);
		pack.setBackend(backend);
		const bits = await pack.download((dl) => {
			this.downloads.set(bit.hash, dl);
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
	download: (bit: IBit) => Promise<IBit[]>;
}

export const useDownloadManager = create<IDownloadManager>((set, get) => ({
	manager: new DownloadManager(),
	backend: {} as IBackendState,
	setDownloadBackend: (backend: IBackendState) => set({ backend }),
	download: async (bit: IBit) => {
		const { manager, backend } = get();
		return await manager.download(backend, bit);
	},
}));
