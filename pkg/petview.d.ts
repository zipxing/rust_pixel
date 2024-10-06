/* tslint:disable */
/* eslint-disable */
/**
*/
export class PetviewGame {
  free(): void;
/**
* @returns {PetviewGame}
*/
  static new(): PetviewGame;
/**
* @param {number} dt
*/
  tick(dt: number): void;
/**
* @param {number} t
* @param {Event} e
*/
  key_event(t: number, e: Event): void;
/**
* @param {number} w
* @param {number} h
* @param {Uint8ClampedArray} d
*/
  upload_imgdata(w: number, h: number, d: Uint8ClampedArray): void;
/**
* @param {string} url
* @param {Uint8Array} data
*/
  on_asset_loaded(url: string, data: Uint8Array): void;
/**
* @returns {number}
*/
  get_ratiox(): number;
/**
* @returns {number}
*/
  get_ratioy(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly petviewgame_new: () => number;
  readonly petviewgame_tick: (a: number, b: number) => void;
  readonly petviewgame_key_event: (a: number, b: number, c: number) => void;
  readonly petviewgame_upload_imgdata: (a: number, b: number, c: number, d: number) => void;
  readonly petviewgame_on_asset_loaded: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly petviewgame_get_ratiox: (a: number) => number;
  readonly petviewgame_get_ratioy: (a: number) => number;
  readonly __wbg_petviewgame_free: (a: number, b: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
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
