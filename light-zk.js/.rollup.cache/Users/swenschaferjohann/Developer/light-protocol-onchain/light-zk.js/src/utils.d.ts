/// <reference types="bn.js" />
/// <reference types="node" />
import { BN } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Utxo } from "./utxo";
import { TokenUtxoBalance, Wallet } from "./wallet";
import { TokenData } from "./types";
export declare function hashAndTruncateToCircuit(data: Uint8Array): BN;
export declare function getAssetLookUpId({ connection, asset, }: {
    asset: PublicKey;
    connection: Connection;
}): Promise<any>;
export declare function getAssetIndex(assetPubkey: PublicKey, assetLookupTable: string[]): BN;
export declare function fetchAssetByIdLookUp(assetIndex: BN, assetLookupTable: string[]): PublicKey;
export declare function fetchVerifierByIdLookUp(index: BN, verifierProgramLookupTable: string[]): PublicKey;
export declare const arrToStr: (uint8arr: Uint8Array) => string;
export declare const strToArr: (str: string) => Uint8Array;
export declare function decimalConversion({ tokenCtx, skipDecimalConversions, publicAmountSpl, publicAmountSol, minimumLamports, minimumLamportsAmount, }: {
    tokenCtx: TokenData;
    skipDecimalConversions?: boolean;
    publicAmountSpl?: BN | string | number;
    publicAmountSol?: BN | string | number;
    minimumLamports?: boolean;
    minimumLamportsAmount?: BN;
}): {
    publicAmountSpl: BN | undefined;
    publicAmountSol: BN | undefined;
};
export declare const convertAndComputeDecimals: (amount: BN | string | number, decimals: BN) => BN;
export declare const getUpdatedSpentUtxos: (tokenBalances: Map<string, TokenUtxoBalance>) => Utxo[];
export declare const fetchNullifierAccountInfo: (nullifier: string, connection: Connection) => Promise<anchor.web3.AccountInfo<Buffer> | null>;
export declare const fetchQueuedLeavesAccountInfo: (leftLeaf: Uint8Array, connection: Connection) => Promise<anchor.web3.AccountInfo<Buffer> | null>;
export declare const sleep: (ms: number) => Promise<unknown>;
export type KeyValue = {
    [key: string]: any;
};
/**
 * @description Creates an object of a type defined in accounts[accountName],
 * @description all properties need to be part of obj, if a property is missing an error is thrown.
 * @description The accounts array is part of an anchor idl.
 * @param obj Object properties are picked from.
 * @param accounts Idl accounts array from which accountName is selected.
 * @param accountName Defines which account in accounts to use as type for the output object.
 * @returns
 */
export declare function createAccountObject<T extends KeyValue>(obj: T, accounts: any[], accountName: string): Partial<KeyValue>;
export declare function firstLetterToLower(input: string): string;
export declare function firstLetterToUpper(input: string): string;
/**
 * This function checks if an account in the provided idk object exists with a name
 * ending with 'PublicInputs' and contains a field named 'publicAppVerifier'.
 *
 * @param {Idl} idl - The IDL object to check.
 * @returns {boolean} - Returns true if such an account exists, false otherwise.
 */
export declare function isProgramVerifier(idl: anchor.Idl): boolean;
export declare function initLookUpTable(payer: Wallet, provider: anchor.Provider, extraAccounts?: Array<PublicKey>): Promise<PublicKey>;
export declare function setEnvironment(): void;
export declare enum System {
    MacOsAmd64 = 0,
    MacOsArm64 = 1,
    LinuxAmd64 = 2,
    LinuxArm64 = 3
}
export declare function getSystem(): System;
//# sourceMappingURL=utils.d.ts.map