import { ethers } from 'ethers';
declare function packEncryptedMessage(encryptedMessage: any): string;
declare function unpackEncryptedMessage(encryptedMessage: any): {
    version: string;
    nonce: string;
    ephemPublicKey: string;
    ciphertext: string;
};
export declare class Keypair {
    /**
     * Initialize a new keypair. Generates a random private key if not defined
     *
     * @param {string} privkey
     */
    privkey: string;
    pubkey: any;
    encryptionKey: any;
    constructor(privkey?: string);
    toString(): string;
    /**
     * Key address for this keypair, alias to {@link toString}
     *
     * @returns {string}
     */
    address(): string;
    /**
     * Initialize new keypair from address string
     *
     * @param str
     * @returns {Keypair}
     */
    static fromString(str: string): never;
    /**
     * Sign a message using keypair private key
     *
     * @param {string|number|BigNumber} commitment a hex string with commitment
     * @param {string|number|BigNumber} merklePath a hex string with merkle path
     * @returns {BigNumber} a hex string with signature
     */
    sign(commitment: any, merklePath: any): ethers.BigNumber;
    /**
     * Encrypt data using keypair encryption key
     *
     * @param {Buffer} bytes
     * @returns {string} a hex string with encrypted data
     */
    encrypt(bytes: any): string;
    /**
     * Decrypt data using keypair private key
     *
     * @param {string} data a hex string with data
     * @returns {Buffer}
     */
    decrypt(data: any): void;
}
export { packEncryptedMessage, unpackEncryptedMessage };
