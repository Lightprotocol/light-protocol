import { BigNumber } from 'ethers';
import { Keypair } from './utils/keypair';
export declare class Utxo {
    /** Initialize a new UTXO - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
     *
     * @param {BigNumber | BigInt | number | string} amount UTXO amount
     * @param {BigNumber | BigInt | number | string} blinding Blinding factor
     */
    amount: number | BigNumber;
    blinding: BigNumber;
    keypair: Keypair;
    index: number | null;
    _commitment: BigNumber | null;
    _nullifier: BigNumber | null;
    constructor(amount?:number[] | BigNumber[], keypair?: Keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding?: BigNumber, index?: number | null, _commitment?: BigNumber | null, // I added null as default if there is an error could be that
    _nullifier?: BigNumber | null);
    /**
     * Returns commitment for this UTXO
     *
     * @returns {BigNumber}
     */
    getCommitment(): BigNumber | null;
    /**
     * Returns nullifier for this UTXO
     *
     * @returns {BigNumber}
     */
    getNullifier(): BigNumber | null;
    /**
     * Encrypt UTXO to recipient pubkey
     *
     * @returns {string}
     */
    encrypt(nonce: Uint8Array, encryptionKeypair: nacl.BoxKeyPair, senderThrowAwayKeypair: nacl.BoxKeyPair): Uint8Array;
    static decrypt(encryptedUtxo: any, nonce: any, senderThrowAwayPubkey: any, recipientEncryptionKeypair: any, shieldedKeypair: any, index: any): (boolean | null)[] | (boolean | Utxo)[];
}
export default Utxo;
