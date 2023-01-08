/// <reference types="bn.js" />
import { BN } from '@project-serum/anchor';
export declare class Keypair {
    /**
     * Initialize a new keypair. Generates a random private key if not defined
     *
     * @param {string} privkey
     */
    privkey: BN;
    pubkey: BN;
    encryptionKey: any;
    poseidon: any;
    constructor(poseidon: any, seed?: string, index?: BN);
    pubKeyToBytes(): void;
    privKeyToBytes(): void;
    encryptionKeyToBytes(): void;
    fromBytes({ pubkey, encPubkey, privkey }: {
        pubkey: Array<any>;
        encPubkey: Array<any>;
        privkey: Array<any>;
    }): void;
    /**
     * Sign a message using keypair private key
     *
     * @param {string|number|BigNumber} commitment a hex string with commitment
     * @param {string|number|BigNumber} merklePath a hex string with merkle path
     * @returns {BigNumber} a hex string with signature
     */
    sign(commitment: any, merklePath: any): any;
}
