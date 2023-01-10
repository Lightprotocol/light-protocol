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
    encPrivateKey?: Uint8Array;
    poseidon: any;
    burnerSeed: Uint8Array;
    constructor({ poseidon, seed, burner, privateKey, publicKey }: {
        poseidon: any;
        seed?: string;
        burner?: Boolean;
        privateKey?: BN;
        publicKey?: BN;
    });
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
    static createBurner(poseidon: any, seed: String, index: BN): Keypair;
    static fromBurnerSeed(poseidon: any, burnerSeed: Uint8Array): Keypair;
    static fromPrivkey(poseidon: any, privateKey: Uint8Array): Keypair;
    static fromPubkey(poseidon: any, publicKey: Uint8Array): Keypair;
    static getEncryptionKeyPair(seed: String): nacl.BoxKeyPair;
    static generateShieldedKeyPrivateKey(seed: String): BN;
    static generateShieldedKeyPublicKey(privateKey: BN, poseidon: any): BN;
}
