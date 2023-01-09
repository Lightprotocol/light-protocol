/// <reference types="bn.js" />
/// <reference types="node" />
import { BN } from '@coral-xyz/anchor';
export declare class Keypair {
    /**
     * Initialize a new keypair. Generates a random private key if not defined
     *
     * @param {BN} privkey
     */
    privkey: BN;
    pubkey: BN;
    encryptionPublicKey: Uint8Array;
    poseidon: any;
    burnerSeed: Uint8Array;
    fromBurnerSeed(burnerSeed: Uint8Array, poseidon: any): Keypair;
    constructor(poseidon: any, seed?: String, index?: BN);
    encryptionPublicKeyToBytes(): Buffer;
    fromBytes({ pubkey, encPubkey, privkey, poseidon, burnerSeed }: {
        pubkey: Array<any>;
        encPubkey: Buffer;
        privkey: Array<any>;
        poseidon: any;
        burnerSeed: Uint8Array;
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
