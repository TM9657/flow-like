import { invoke } from "@tauri-apps/api/core";
import { Bit } from "./bit";
import { type IBitPack } from "../schema/bit/bit-pack";
import { type IBit } from "../schema/bit/bit";


export class BitPack implements IBitPack {
    bits: IBit[] = [];

    async get_installed(): Promise<Bit[]> {
        return await invoke("get_installed_bit", { bits: this.bits })
    }

    async size(): Promise<number> {
        return await invoke("get_pack_size", { bits: this.bits })
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