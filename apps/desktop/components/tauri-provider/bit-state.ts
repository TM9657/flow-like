import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import type {
	IBit,
	IBitPack,
	IBitState,
	IDownloadProgress,
	IIntercomEvent,
	ISettingsProfile,
} from "@tm9657/flow-like-ui";
import type { IBitSearchQuery } from "@tm9657/flow-like-ui/lib/schema/hub/bit-search-query";
import type { TauriBackend } from "../tauri-provider";

export class BitState implements IBitState {
	constructor(private readonly backend: TauriBackend) {}

	async getInstalledBit(bits: IBit[]): Promise<IBit[]> {
		return await invoke("get_installed_bit", {
			bits: bits,
		});
	}
	async downloadBit(
		bit: IBit,
		pack: IBitPack,
		cb?: (progress: IDownloadProgress[]) => void,
	): Promise<IBit[]> {
		const unlistenFn: UnlistenFn[] = [];

		for (const deps of pack.bits) {
			unlistenFn.push(
				await listen(`download:${deps.hash}`, (event) => {
					const payload = event.payload as IIntercomEvent[];
					const downloadProgressEvents = payload.map((item) => item.payload);
					if (cb) cb(downloadProgressEvents);
				}),
			);
		}

		const bits: IBit[] = await invoke("download_bit", {
			bit: bit,
		});

		for (const unlisten of unlistenFn) {
			unlisten();
		}

		return bits;
	}

	async getPackFromBit(bit: IBit): Promise<{ bits: IBit[] }> {
		console.log("Getting pack from bit:", bit);
		const pack = await invoke<{ bits: IBit[] }>("get_pack_from_bit", {
			bit: bit,
		});
		console.log("Pack retrieved:", pack);
		return pack;
	}

	async deleteBit(bit: IBit): Promise<void> {
		throw new Error("Method not implemented.");
	}
	async getBit(id: string, hub?: string): Promise<IBit> {
		return await invoke("get_bit", {
			bit: id,
			hub: hub,
		});
	}
	async addBit(bit: IBit, profile: ISettingsProfile): Promise<void> {
		await invoke("add_bit", {
			bit: bit,
			profile: profile,
		});
	}
	async removeBit(bit: IBit, profile: ISettingsProfile): Promise<void> {
		await invoke("remove_bit", {
			bit: bit,
			profile: profile,
		});
	}
	async getPackSize(bits: IBit[]): Promise<number> {
		const size: number = await invoke("get_bit_size", {
			bits: bits,
		});
		return size;
	}
	async getBitSize(bit: IBit): Promise<number> {
		return await invoke("get_bit_size", {
			bit: bit,
		});
	}
	async searchBits(query: IBitSearchQuery): Promise<IBit[]> {
		return await invoke("search_bits", {
			query,
		});
	}
	async isBitInstalled(bit: IBit): Promise<boolean> {
		return await invoke("is_bit_installed", {
			bit: bit,
		});
	}
	async getProfileBits(): Promise<IBit[]> {
		return await invoke("get_bits_in_current_profile");
	}
}
