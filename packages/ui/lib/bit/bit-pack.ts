import type { IBackendState } from "../../state/backend-state";
import type { IBit } from "../schema/bit/bit";
import type { IBitPack } from "../schema/bit/bit-pack";
import { Bit } from "./bit";

export class BitPack implements IBitPack {
	bits: IBit[] = [];
	backend: IBackendState | undefined

	public setBackend(backend?: IBackendState) {
		this.backend = backend;
		return this
	}

	async get_installed(): Promise<Bit[]> {
		const installed = await this.backend?.getInstalledBit(this.bits);
		if (!installed) {
			throw new Error("No installed bits found");
		}
		return installed.map((bit) => Bit.fromObject(bit).setBackend(this.backend));
	}

	async size(): Promise<number> {
		const size = await this.backend?.getPackSize(this.bits);
		if (!size) {
			throw new Error("No size found");
		}
		return size;
	}

	public static fromJson(json: string): BitPack {
		const object = JSON.parse(json);
		return BitPack.fromObject(object);
	}

	public toJson(): string {
		const object = this.toObject();
		return JSON.stringify(object);
	}

	public static fromObject(obj: IBitPack): BitPack {
		const bitpack = new BitPack();

		for (const key of Object.keys(obj)) {
			(bitpack as any)[key] = obj[key];
		}

		return bitpack;
	}

	public toObject(): IBitPack {
		const obj: Record<string, any> = {};
		Object.keys(this).forEach((key) => {
			if (typeof (this as any)[key] !== "function") {
				obj[key] = (this as any)[key];
			}
		});
		return obj as IBitPack;
	}
}
