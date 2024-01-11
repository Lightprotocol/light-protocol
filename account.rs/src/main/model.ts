import { BN } from "@coral-xyz/anchor";
import { Account } from "./wasm/account_wasm";
export { Account } from "./wasm/account_wasm";

export interface LightWasm {
    blakeHash(input: string|Uint8Array, hashLength: number): Uint8Array;
    poseidonHash(input: string[] | BN[]): Uint8Array;
    poseidonHashString(input: string[] | BN[]): string;
    poseidonHashBN(input: string[] | BN[]): BN;
    seedAccount(seed: string): Account;
    aesAccount(aesSecret: Uint8Array): Account;
    privateKeyAccount(privateKey: Uint8Array, encryptionPrivateKey: Uint8Array, aesSecret: Uint8Array): Account;
    publicKeyAccount(publicKey: Uint8Array, encryptionPublicKey: Uint8Array | undefined): Account;
    burnerAccount(seed: string, index: string): Account;
    burnerSeedAccount(seed: string): Account;
    encryptNaclUtxo(public_key: Uint8Array, message: Uint8Array, commitment: Uint8Array): Uint8Array;
}

export interface LightWasmCreator {
    create(): LightWasm;
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
