import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { type IBit, type IBitMeta, IBitTypes } from "../schema/bit/bit";
import { type IImageEmbeddingModelParameters } from "../schema/bit/bit/image-embedding-model-parameters";
import { type IEmbeddingModelParameters } from "../schema/bit/bit/embedding-model-parameters";
import { type ILlmParameters } from "../schema/bit/bit/llm-parameters";
import { type IVlmParameters } from "../schema/bit/bit/vlm-parameters";
import { type Nullable } from "../schema/auto-import";
import { BitPack } from "./bit-pack";
import { get, set } from "idb-keyval";

export interface IDownloadProgress {
	hash: string;
	max: number;
	downloaded: number;
	path: string;
}

export class Download {
	progress = new Map<string, IDownloadProgress>();
	speed = {
		lastMeasured: 0,
		lastPoints: new Map<string, number>(),
	};

	push(progress: IDownloadProgress) {
		this.progress.set(progress.hash, progress);
	}

	total() {
		const max = Array.from(this.progress.values()).reduce(
			(acc, progress) => acc + progress.max,
			0,
		);
		const downloaded = Array.from(this.progress.values()).reduce(
			(acc, progress) => acc + progress.downloaded,
			0,
		);
		return { max, downloaded };
	}

	files() {
		return this.progress;
	}

	measureSpeed() {
		const now = Date.now();
		const last = this.speed.lastMeasured;
		const time = now - last;
		this.speed.lastMeasured = now;

		const { max, downloaded } = this.total();
		const lastDownloaded = this.speed.lastPoints
			.values()
			.reduce((acc, val) => acc + val, 0);

		const bytesPerSecond = ((downloaded - lastDownloaded) / time) * 1000;

		this.speed.lastMeasured = now;
		this.speed.lastPoints.clear();

		this.progress.forEach((progress) => {
			this.speed.lastPoints.set(progress.hash, progress.downloaded);
		});

		return {
			bytesPerSecond,
			total: downloaded,
			progress: (downloaded / max) * 100,
			max,
		};
	}

	clear() {
		this.progress.clear();
	}
}

export class Bit implements IBit {
	authors: string[] = [];
	created: string = "";
	dependencies: Array<string[]> = [];
	dependency_tree_hash: string = "";
	download_link?: Nullable<string>;
	file_name?: Nullable<string>;
	hash: string = "";
	hub: string = "";
	icon: string = "";
	id: string = "";
	license: string = "";
	meta: { [key: string]: IBitMeta } = {};
	parameters:
		| IImageEmbeddingModelParameters
		| IEmbeddingModelParameters
		| ILlmParameters
		| IVlmParameters
		| object = {};
	repository?: Nullable<string>;
	size?: Nullable<number>;
	type: IBitTypes = IBitTypes.Other;
	updated: string = "";
	version: string = "";

	public toJSON(): string {
		const object = this.toObject();
		return JSON.stringify(object);
	}

	public static fromJson(json: string): Bit {
		const object = JSON.parse(json);
		return Bit.fromObject(object);
	}

	public toObject(): IBit {
		const obj: Record<string, any> = {};
		Object.keys(this).forEach((key) => {
			if (typeof (this as any)[key] !== "function") {
				obj[key] = (this as any)[key];
			}
		});
		return obj as IBit;
	}

	public static fromObject(obj: IBit): Bit {
		const bit = new Bit();

		for (const key of Object.keys(obj)) {
			(bit as any)[key] = obj[key];
		}

		return bit;
	}

	public async fetchDependencies(): Promise<BitPack> {
		const cachedDependencies: IBit[] | undefined = await get(
			this.dependency_tree_hash,
		);

		if (cachedDependencies) {
			const deps: IBit[] = cachedDependencies;
			const pack = new BitPack();
			pack.bits = deps;
			return pack;
		}

		const bits: {
			bits: IBit[];
		} = await invoke("get_pack_from_bit", { bit: this.toObject() });

		if (bits.bits.length > 0) await set(this.dependency_tree_hash, bits.bits);
		const pack = new BitPack();
		pack.bits = bits.bits;
		return pack;
	}

	async fetchSize(): Promise<number> {
		const pack = await this.fetchDependencies();
		return pack.bits.reduce((acc, bit) => acc + (bit.size ?? 0), 0);
	}

	async download(cb?: (progress: Download) => void): Promise<IBit[]> {
		try {
			const dependencies = await this.fetchDependencies();
			const unlistenFn: UnlistenFn[] = [];

			const totalProgress = new Download();

			for (const deps of dependencies.bits) {
				unlistenFn.push(
					await listen(
						`download:${deps.hash}`,
						(event: { payload: IDownloadProgress[] }) => {
							const lastElement = event.payload.pop();
							if (lastElement) totalProgress.push(lastElement);
							if (cb) cb(totalProgress);
						},
					),
				);
			}

			const download: IBit[] = await invoke("download_bit", {
				bit: this.toObject(),
			});

			for (const unlisten of unlistenFn) {
				unlisten();
			}
			return download;
		} catch (error) {
			console.error(error);
			throw error;
		}
	}
}
