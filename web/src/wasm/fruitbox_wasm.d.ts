/* tslint:disable */
/* eslint-disable */

export class WasmMoveList {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    get(index: number): WasmRectangle | undefined;
    readonly length: number;
}

export class WasmRectangle {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly bottom: number;
    readonly left: number;
    readonly right: number;
    readonly score: number;
    readonly top: number;
}

export function apply_rectangle(cells: Uint8Array, width: number, left: number, top: number, right: number, bottom: number): Uint8Array;

export function create_board_cells(width: number, height: number, seed: number): Uint8Array;

export function find_static_moves(cells: Uint8Array, width: number, target: number): WasmMoveList;

export function is_inside(index: number, width: number, left: number, top: number, right: number, bottom: number): boolean;

export function rectangle_from_cells(start_index: number, end_index: number, width: number): WasmRectangle;

export function score_rectangle(cells: Uint8Array, width: number, left: number, top: number, right: number, bottom: number): WasmRectangle;

export function sum_rectangle(cells: Uint8Array, width: number, left: number, top: number, right: number, bottom: number): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmmovelist_free: (a: number, b: number) => void;
    readonly __wbg_wasmrectangle_free: (a: number, b: number) => void;
    readonly apply_rectangle: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly create_board_cells: (a: number, b: number, c: number) => [number, number];
    readonly find_static_moves: (a: number, b: number, c: number, d: number) => number;
    readonly is_inside: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly rectangle_from_cells: (a: number, b: number, c: number) => number;
    readonly score_rectangle: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly sum_rectangle: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly wasmmovelist_get: (a: number, b: number) => number;
    readonly wasmmovelist_length: (a: number) => number;
    readonly wasmrectangle_bottom: (a: number) => number;
    readonly wasmrectangle_left: (a: number) => number;
    readonly wasmrectangle_right: (a: number) => number;
    readonly wasmrectangle_score: (a: number) => number;
    readonly wasmrectangle_top: (a: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
