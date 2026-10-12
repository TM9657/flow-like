/* tslint:disable */
/* eslint-disable */

export function request_repaint(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly request_repaint: () => void;
    readonly main: (a: number, b: number) => number;
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke___wasm_bindgen_b49e035ff7ca9eff___JsValue__core_e6c6c7d8ff9eb85b___result__Result_____wasm_bindgen_b49e035ff7ca9eff___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke___wasm_bindgen_b49e035ff7ca9eff___sys__JsNullable_wgpu_2645c413d7538e0b___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_e6c6c7d8ff9eb85b___result__Result_____wasm_bindgen_b49e035ff7ca9eff___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke___wasm_bindgen_b49e035ff7ca9eff___sys__JsNullable_wgpu_2645c413d7538e0b___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_e6c6c7d8ff9eb85b___result__Result_____wasm_bindgen_b49e035ff7ca9eff___JsError___true__5: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke___wasm_bindgen_b49e035ff7ca9eff___sys__JsNullable_wgpu_2645c413d7538e0b___backend__webgpu__webgpu_sys__gen_GpuAdapter__GpuAdapter___core_e6c6c7d8ff9eb85b___result__Result_____wasm_bindgen_b49e035ff7ca9eff___JsError___true__6: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke___core_e6c6c7d8ff9eb85b___result__Result_____wasm_bindgen_b49e035ff7ca9eff___JsValue___true_: (a: number, b: number) => [number, number];
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke___js_sys_e521657ab49fdc74___Array______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke___js_sys_e521657ab49fdc74___Array______true__3: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_b49e035ff7ca9eff___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
