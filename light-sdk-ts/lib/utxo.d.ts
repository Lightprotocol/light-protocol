/// <reference types="bn.js" />
import { Keypair } from "./keypair";
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
export declare const newNonce: () => Uint8Array;
export declare const N_ASSETS = 2;
export declare const N_ASSET_PUBKEYS = 3;
export declare class Utxo {
    /** Initialize a new UTXO - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
     *
     * @param {new BN[]} amounts array UTXO amount
     * @param {new BN | new BN | number | string} blinding Blinding factor
     */
    amounts: BN[];
    assets: PublicKey[];
    assetsCircuit: BN[];
    blinding: BN;
    keypair: Keypair;
    index: number | null;
    appData: Array<any>;
    verifierAddress: BN;
    verifierAddressCircuit: BN;
    instructionType: BN;
    poolType: BN;
    _commitment: BN | null;
    _nullifier: BN | null;
    poseidon: any;
    constructor({ poseidon, assets, amounts, keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding, poolType, verifierAddress, appData, appDataFromBytesFn, index, }: {
        poseidon: any;
        assets?: PublicKey[];
        amounts?: BN[];
        keypair?: Keypair;
        blinding?: BN;
        poolType?: BN;
        verifierAddress?: PublicKey;
        appData?: Array<any>;
        appDataFromBytesFn?: Function;
        index?: any;
    });
    toBytes(): Uint8Array;
    static fromBytes({ poseidon, bytes, keypair, keypairInAppDataOffset, }: {
        poseidon: any;
        bytes: Uint8Array;
        keypair?: Keypair;
        keypairInAppDataOffset?: number;
    }): Utxo;
    /**
     * Returns commitment for this UTXO
     *signature:
     * @returns {BN}
     */
    getCommitment(): BN | null;
    /**
     * Returns nullifier for this UTXO
     *
     * @returns {BN}
     */
    getNullifier(): BN | null;
    /**
     * Encrypt UTXO to recipient pubkey
     *
     * @returns {string}
     */
    encrypt(): Uint8Array;
    static decrypt({ poseidon, encBytes, keypair, }: {
        poseidon: any;
        encBytes: Uint8Array;
        keypair: Keypair;
    }): Utxo | null;
}
