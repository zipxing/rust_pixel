/* tslint:disable */
/* eslint-disable */
export class SnakeGame {
  private constructor();
  free(): void;
  static new(): SnakeGame;
  tick(dt: number): void;
  key_event(t: number, e: Event): void;
  upload_imgdata(w: number, h: number, d: Uint8ClampedArray): void;
  on_asset_loaded(url: string, data: Uint8Array): void;
  get_ratiox(): number;
  get_ratioy(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly snakegame_new: () => number;
  readonly snakegame_tick: (a: number, b: number) => void;
  readonly snakegame_key_event: (a: number, b: number, c: any) => void;
  readonly snakegame_upload_imgdata: (a: number, b: number, c: number, d: any) => void;
  readonly snakegame_on_asset_loaded: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly snakegame_get_ratiox: (a: number) => number;
  readonly snakegame_get_ratioy: (a: number) => number;
  readonly __wbg_snakegame_free: (a: number, b: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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
