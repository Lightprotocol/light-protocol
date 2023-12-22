import type {
    HashCreator,
    AccountLoadOptions,
    Hasher,
    InitInput,
    WasmInput,
} from "./model.js";

import init, {
    Account as AccountWasm,
    blake2 as blake2Wasm,
    blake2str as blake2strWasm,
    poseidon as poseidonWasm,
} from "./wasm/account_wasm";
import simdInit, {
    Account as AccountSimd,
    blake2 as blake2Simd,
    blake2str as blake2strSimd,
    poseidon as poseidonSimd,
} from "./wasm-simd/account_wasm_simd";
import {BN} from "@coral-xyz/anchor";

function stringify(input: string[] | BN[]): string[] {
    if (input.length > 0 && input[0] instanceof BN) {
        return (input as BN[]).map((item) => item.toString(10));
    } else {
        return input as string[];
    }
}

let wasmInit: (() => InitInput) | undefined = undefined;
export const setWasmInit = (arg: () => InitInput) => {
    wasmInit = arg;
};

let wasmSimdInit: (() => InitInput) | undefined = undefined;
export const setWasmSimdInit = (arg: () => InitInput) => {
    wasmSimdInit = arg;
};

const isWasmInput = (
    x?: AccountLoadOptions["wasm"]
): x is WasmInput | undefined =>
    x === undefined || (typeof x === "object" && "simd" in x);

/**
 * account.rs implemented in Web assembly.
 */
export class WasmHasher {
    static async loadModule(
        options?: Partial<AccountLoadOptions>
    ): Promise<HashCreator> {
        if (isWasmInput(options?.wasm)) {
            const useSimd = options?.simd ?? hasSimd();
            if (!useSimd) {
                return await loadWasm(options?.wasm?.sisd);
            } else {
                return await loadWasmSimd(options?.wasm?.simd);
            }
        } else {
            return await loadWasm(options?.wasm);
        }
    }

    static async loadAccount(
        options?: Partial<AccountLoadOptions>
    ): Promise<Hasher> {
        const module = await WasmHasher.loadModule(options);
        return module.create();
    }

    static resetModule() {
        sisdMemory = undefined;
        simdMemory = undefined;
    }


    static async getInstance(): Promise<Hasher> {
        return (await WasmHasher.loadModule()).create();
    }
}

interface HashStrategy {
    blake2str(input: string, hash_length: number): Uint8Array;
    blake2(input: Uint8Array, hash_length: number): Uint8Array;
    poseidon(inputs: Array<any>): Uint8Array;
    account(seed: string): AccountWasm;
}


function wasmAccount(hasher: HashStrategy): HashCreator {
    const WasmHasher = class implements Hasher {
        blakeHash(input: string | Uint8Array, hashLength: number): Uint8Array {
            if (typeof input === 'string') {
                return hasher.blake2str(input, hashLength);
            }
            else {
                return hasher.blake2(input, hashLength);
            }
        }

        poseidonHash(input: string[] | []): Uint8Array {
            return hasher.poseidon(stringify(input));
        }

        poseidonHashBN(input: string[] | []): BN {
            return new BN(this.poseidonHash(input));
        }

        poseidonHashString(input: string[] | []): string {
            const bn = new BN(this.poseidonHash(input));
            return bn.toString();
        }

        account(seed: string): AccountWasm {
            return hasher.account(seed);
        }
    };

    return {
        create: () => new WasmHasher(),
    };
}

let sisdMemory: Promise<HashCreator> | undefined;
let simdMemory: Promise<HashCreator> | undefined;
const loadWasmSimd = async (module?: InitInput) => {
    if (simdMemory === undefined) {
        simdMemory = simdInit(module ?? wasmSimdInit?.()).then((x) => {
            return wasmAccount({
                blake2str: blake2strSimd,
                blake2: blake2Simd,
                poseidon: poseidonSimd,
                account: AccountSimd.new
            });
        });
    }
    return await simdMemory;
};

const loadWasm = async (module?: InitInput) => {
    if (sisdMemory === undefined) {
        sisdMemory = init(module ?? wasmInit?.()).then((x) => {
            // grow by 1 page to hold key, results, and data hashing
            return wasmAccount({
                blake2str: blake2strWasm,
                blake2: blake2Wasm,
                poseidon: poseidonWasm,
                account: AccountWasm.new
            });
        });
    }
    return await sisdMemory;
};

// Extracted from the compiled file of:
// https://github.com/GoogleChromeLabs/wasm-feature-detect/blob/40269813c83f7e9ff370afc92cde3cc0456c557e/src/detectors/simd/module.wat
//
// Changes:
//  - Validation is cached so it needs to only run once
//  - There's no need to mark as async
let simdEnabled: boolean | undefined;
export const hasSimd = () =>
    simdEnabled ??
    (simdEnabled = WebAssembly.validate(
        new Uint8Array([
            0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10,
            1, 8, 0, 65, 0, 253, 15, 253, 98, 11,
        ])
    ));
