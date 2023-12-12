import {BN} from "@coral-xyz/anchor";

export interface Hasher {
    blakeHash(input: string|Uint8Array, hashLength: number): Uint8Array;
    poseidonHash(input: string[] | BN[]): Uint8Array;
    poseidonHashString(input: string[] | BN[]): string;
    poseidonHashBN(input: string[] | BN[]): BN;
}
export interface HashCreator {
    create(): Hasher;
}

/**
 * Parameters able to instantiate the Wasm hasher
 */
export type InitInput =
    | RequestInfo
    | URL
    | Response
    | BufferSource
    | WebAssembly.Module;

export type WasmInput = {
    /** parameter that describes how to instantiate the non-SIMD enabled Wasm */
    sisd: InitInput;

    /** parameter that describes how to instantiate the SIMD enabled Wasm */
    simd: InitInput;
};

/**
 * Customize how modules are loaded
 */
export interface AccountLoadOptions {
    /**
     * Execute Hash with SIMD instructions. This option is only
     * applicable in a Wasm environment, as native hardware will detect SIMD at
     * runtime. `account.rs` will detect if Wasm SIMD is enabled if this
     * option is not set, so this option is used to override the heuristic.
     */
    simd?: boolean;

    /**
     * Controls how the Wasm module is instantiated. This option is only
     * applicable in browser environments or for users that opt to use the Wasm
     * hasher. If the `wasm` option is given a single instantiation parameter,
     * there is no SIMD check performed.
     */
    wasm?: WasmInput | InitInput;
}
