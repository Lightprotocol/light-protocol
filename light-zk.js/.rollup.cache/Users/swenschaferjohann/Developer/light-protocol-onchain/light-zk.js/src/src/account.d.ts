/// <reference types="bn.js" />
/// <reference types="node" />
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
export declare class Account {
    /**
     * Initialize a new shielded account. Generates a random private key if not defined
     *
     * @param {BN} privkey
     * @param {BN} pubkey
     */
    privkey: BN;
    pubkey: BN;
    encryptionKeypair: nacl.BoxKeyPair;
    burnerSeed: Uint8Array;
    poseidonEddsaKeypair?: {
        publicKey?: [Uint8Array, Uint8Array];
        privateKey: Uint8Array;
    };
    eddsa: any;
    aesSecret?: Uint8Array;
    /**
     *
     * @param poseidon required
     * @param seed
     * @param burner
     * @param privateKey
     * @param publicKey
     * @param encryptionPublicKey required for transfers to other users
     */
    constructor({ poseidon, seed, burner, privateKey, publicKey, poseidonEddsaPrivateKey, eddsa, encryptionPublicKey, encryptionPrivateKey, aesSecret, }: {
        poseidon?: any;
        seed?: string;
        burner?: Boolean;
        privateKey?: BN;
        publicKey?: BN;
        poseidonEddsaPrivateKey?: Uint8Array;
        eddsa?: any;
        encryptionPublicKey?: Uint8Array;
        encryptionPrivateKey?: Uint8Array;
        aesSecret?: Uint8Array;
    });
    getEddsaPublicKey(eddsa?: any): Promise<[Uint8Array, Uint8Array]>;
    static getEddsaPrivateKey(seed: string): {
        publicKey: any;
        privateKey: any;
    };
    encryptionPublicKeyToBytes(): Buffer;
    signEddsa(msg: string | Uint8Array, eddsa?: any): Promise<Uint8Array>;
    /**
     * Sign a message using keypair private key
     *
     * @param {string|number|BigNumber} commitment a hex string with commitment
     * @param {string|number|BigNumber} merklePath a hex string with merkle path
     * @returns {BigNumber} a hex string with signature
     */
    sign(poseidon: any, commitment: any, merklePath: any): any;
    /**
     * @description gets domain separated aes secret to execlude the possibility of nonce reuse
     * @note we derive an aes key for every utxo of every merkle tree
     *
     * @param {PublicKey} merkleTreePdaPublicKey
     * @param {number} index the xth transaction for the respective merkle tree
     * @returns {Uint8Array} the blake2b hash of the aesSecret + merkleTreePdaPublicKey.toBase58() + index.toString()
     */
    getAesUtxoViewingKey(merkleTreePdaPublicKey: PublicKey, salt: string): Uint8Array;
    getDomainSeparatedAesSecretKey(domain: string): Uint8Array;
    static createBurner(poseidon: any, seed: String, index: BN): Account;
    static fromBurnerSeed(poseidon: any, burnerSeed: string): Account;
    static fromPrivkey(poseidon: any, privateKey: string, encryptionPrivateKey: string, aesSecret: string): Account;
    getPrivateKeys(): {
        privateKey: string;
        encryptionPrivateKey: string;
        aesSecret: string;
    };
    static fromPubkey(publicKey: string, poseidon: any): Account;
    getPublicKey(): string;
    static getEncryptionKeyPair(seed: String): nacl.BoxKeyPair;
    static generateShieldedPrivateKey(seed: String, poseidon: any): BN;
    static generateAesSecret(seed: String, _domain?: string): Uint8Array;
    static generateShieldedPublicKey(privateKey: BN, poseidon: any): BN;
    static encryptAes(aesSecret: Uint8Array, message: Uint8Array, iv: Uint8Array): Promise<Uint8Array>;
    static decryptAes(aesSecret: Uint8Array, encryptedBytes: Uint8Array): Promise<any>;
    static encryptNacl(publicKey: Uint8Array, message: Uint8Array, signerSecretKey: Uint8Array, nonce?: Uint8Array, returnWithoutNonce?: boolean): Uint8Array;
    decryptNacl(ciphertext: Uint8Array, nonce?: Uint8Array, signerpublicKey?: Uint8Array): Uint8Array | null;
}
//# sourceMappingURL=account.d.ts.map