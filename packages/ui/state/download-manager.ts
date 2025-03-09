import { create } from "zustand";
import type { IDownloadProgress } from "../lib/bit/bit";

interface BearState {
	stats: {
		time: number;
		timeString: string;
		speed: number;
		total: number;
		progress: number;
		max: number;
	}[];
	progress: Map<string, IDownloadProgress>;
	pushDownload: (progress: IDownloadProgress) => void;
}

const useBearStore = create<BearState>()((set) => ({
	stats: [],
	progress: new Map(),
	pushDownload: (progress) => {
		set((state) => ({
			...state,
			progress: new Map(state.progress).set(progress.hash, progress),
		}));
	},
}));
