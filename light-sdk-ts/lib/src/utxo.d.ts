/// <reference types="bn.js" />
import { Keypair } from './keypair';
import { BigNumber } from 'ethers';
import { PublicKey } from '@solana/web3.js';
import { BN } from '@project-serum/anchor';
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
    instructionType: BigNumber;
    poolType: BN;
    _commitment: BN | null;
    _nullifier: BN | null;
    constructor({ poseidon, assets, amounts, keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding, poolType, verifierAddress, appData, index }: {
        poseidon: any;
        assets?: PublicKey[] | undefined;
        amounts?: BN[] | undefined;
        keypair: any;
        blinding?: BN | undefined;
        poolType?: BN | undefined;
        verifierAddress?: PublicKey | undefined;
        appData?: never[] | undefined;
        index?: null | undefined;
    });
    toBytes(): Uint8Array;
    fromBytes(bytes: Uint8Array, keypairInAppDataOffset?: number): this;
    /**
     * Returns commitment for this UTXO
     *signature:
     * @returns {BigNumber}
     */
    getCommitment(): BN | null;
    /**
     * Returns nullifier for this UTXO
     *
     * @returns {BigNumber}
     */
    getNullifier(): BN | null;
    /**
     * Encrypt UTXO to recipient pubkey
     *
     * @returns {string}
     */
    encrypt(nonce: any, encryptionKeypair: any, senderThrowAwayKeypair: any): Uint8Array;
    static decrypt(encryptedUtxo: Uint8Array, nonce: Uint8Array, senderThrowAwayPubkey: Uint8Array, recipientEncryptionKeypair: any, shieldedKeypair: any, assets: never[] | undefined, POSEIDON: any, index: any): (boolean | null)[] | (boolean | Utxo)[];
}
