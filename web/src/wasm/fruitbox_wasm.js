/* @ts-self-types="./fruitbox_wasm.d.ts" */

export class WasmMoveList {
    static __wrap(ptr) {
        const obj = Object.create(WasmMoveList.prototype);
        obj.__wbg_ptr = ptr;
        WasmMoveListFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmMoveListFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmmovelist_free(ptr, 0);
    }
    /**
     * @param {number} index
     * @returns {WasmRectangle | undefined}
     */
    get(index) {
        const ret = wasm.wasmmovelist_get(this.__wbg_ptr, index);
        return ret === 0 ? undefined : WasmRectangle.__wrap(ret);
    }
    /**
     * @returns {number}
     */
    get length() {
        const ret = wasm.wasmmovelist_length(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) WasmMoveList.prototype[Symbol.dispose] = WasmMoveList.prototype.free;

export class WasmRectangle {
    static __wrap(ptr) {
        const obj = Object.create(WasmRectangle.prototype);
        obj.__wbg_ptr = ptr;
        WasmRectangleFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmRectangleFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmrectangle_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get bottom() {
        const ret = wasm.wasmrectangle_bottom(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get left() {
        const ret = wasm.wasmrectangle_left(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get right() {
        const ret = wasm.wasmrectangle_right(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get score() {
        const ret = wasm.wasmrectangle_score(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get top() {
        const ret = wasm.wasmrectangle_top(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) WasmRectangle.prototype[Symbol.dispose] = WasmRectangle.prototype.free;

/**
 * @param {Uint8Array} cells
 * @param {number} width
 * @param {number} left
 * @param {number} top
 * @param {number} right
 * @param {number} bottom
 * @returns {Uint8Array}
 */
export function apply_rectangle(cells, width, left, top, right, bottom) {
    const ptr0 = passArray8ToWasm0(cells, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.apply_rectangle(ptr0, len0, width, left, top, right, bottom);
    var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v2;
}

/**
 * @param {number} width
 * @param {number} height
 * @param {number} seed
 * @returns {Uint8Array}
 */
export function create_board_cells(width, height, seed) {
    const ret = wasm.create_board_cells(width, height, seed);
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * @param {Uint8Array} cells
 * @param {number} width
 * @param {number} target
 * @returns {WasmMoveList}
 */
export function find_static_moves(cells, width, target) {
    const ptr0 = passArray8ToWasm0(cells, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.find_static_moves(ptr0, len0, width, target);
    return WasmMoveList.__wrap(ret);
}

/**
 * @param {number} index
 * @param {number} width
 * @param {number} left
 * @param {number} top
 * @param {number} right
 * @param {number} bottom
 * @returns {boolean}
 */
export function is_inside(index, width, left, top, right, bottom) {
    const ret = wasm.is_inside(index, width, left, top, right, bottom);
    return ret !== 0;
}

/**
 * @param {number} start_index
 * @param {number} end_index
 * @param {number} width
 * @returns {WasmRectangle}
 */
export function rectangle_from_cells(start_index, end_index, width) {
    const ret = wasm.rectangle_from_cells(start_index, end_index, width);
    return WasmRectangle.__wrap(ret);
}

/**
 * @param {Uint8Array} cells
 * @param {number} width
 * @param {number} left
 * @param {number} top
 * @param {number} right
 * @param {number} bottom
 * @returns {WasmRectangle}
 */
export function score_rectangle(cells, width, left, top, right, bottom) {
    const ptr0 = passArray8ToWasm0(cells, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.score_rectangle(ptr0, len0, width, left, top, right, bottom);
    return WasmRectangle.__wrap(ret);
}

/**
 * @param {Uint8Array} cells
 * @param {number} width
 * @param {number} left
 * @param {number} top
 * @param {number} right
 * @param {number} bottom
 * @returns {number}
 */
export function sum_rectangle(cells, width, left, top, right, bottom) {
    const ptr0 = passArray8ToWasm0(cells, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.sum_rectangle(ptr0, len0, width, left, top, right, bottom);
    return ret;
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_bbadd78c1bac3a77: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./fruitbox_wasm_bg.js": import0,
    };
}

const WasmMoveListFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmmovelist_free(ptr, 1));
const WasmRectangleFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmrectangle_free(ptr, 1));

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('fruitbox_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
