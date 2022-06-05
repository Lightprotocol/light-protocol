"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Utxo = void 0;
const poseidonHash_1 = require("./utils/poseidonHash");
const randomBN_1 = require("./utils/randomBN");
const toBuffer_1 = require("./utils/toBuffer");
const ethers_1 = require("ethers");
const nacl = require('tweetnacl');
nacl.util = require('tweetnacl-util');
const keypair_1 = require("./utils/keypair");
class Utxo {
    constructor(amount = 0, keypair = new keypair_1.Keypair(), // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding = (0, randomBN_1.randomBN)(), index = null, _commitment = null, // I added null as default if there is an error could be that
    _nullifier = null) {
        // I added null as default if there is an error could be that
        this.amount = ethers_1.BigNumber.from(amount);
        this.blinding = ethers_1.BigNumber.from(blinding);
        this.keypair = keypair;
        this.index = index;
        this._commitment = _commitment;
        this._nullifier = _nullifier;
    }
    /**
     * Returns commitment for this UTXO
     *
     * @returns {BigNumber}
     */
    getCommitment() {
        if (!this._commitment) {
            this._commitment = (0, poseidonHash_1.poseidonHash)([
                this.amount,
                this.keypair.pubkey,
                this.blinding,
            ]);
        }
        return this._commitment;
    }
    /**
     * Returns nullifier for this UTXO
     *
     * @returns {BigNumber}
     */
    getNullifier() {
        if (!this._nullifier) {
            if (this.amount > 0 &&
                (this.index === undefined ||
                    this.index === null ||
                    this.keypair.privkey === undefined ||
                    this.keypair.privkey === null)) {
                throw new Error('Can not compute nullifier without utxo index or private key');
            }
            const signature = this.keypair.privkey
                ? this.keypair.sign(this.getCommitment(), this.index || 0)
                : 0;
            this._nullifier = (0, poseidonHash_1.poseidonHash)([
                this.getCommitment(),
                this.index || 0,
                signature,
            ]);
        }
        return this._nullifier;
    }
    /**
     * Encrypt UTXO to recipient pubkey
     *
     * @returns {string}
     */
    encrypt(nonce, encryptionKeypair, senderThrowAwayKeypair) {
        const bytes_message = Buffer.concat([
            (0, toBuffer_1.toBuffer)(this.blinding, 31),
            (0, toBuffer_1.toBuffer)(this.amount, 8),
        ]);
        // console.log("bytes_message", bytes_message)
        // console.log("nonce", nonce)
        // console.log("encryptionKeypair", encryptionKeypair)
        // console.log("senderThrowAwayKeypair", senderThrowAwayKeypair)
        const ciphertext = nacl.box(bytes_message, nonce, encryptionKeypair.publicKey, senderThrowAwayKeypair.secretKey);
        // console.log("CIPHERTEXT", ciphertext)
        // console.log("CIPHERTEXT TYPE", typeof ciphertext)
        return ciphertext;
    }
    static decrypt(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair, shieldedKeypair, index) {
        const cleartext = nacl.box.open(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair.secretKey);
        if (!cleartext) {
            return [false, null];
        }
        const buf = Buffer.from(cleartext);
        const utxoAmount = ethers_1.BigNumber.from('0x' + buf.slice(31, 39).toString('hex'));
        const utxoBlinding = ethers_1.BigNumber.from('0x' + buf.slice(0, 31).toString('hex'));
        return [
            true,
            new Utxo(utxoAmount, shieldedKeypair, // only recipient can decrypt, has full keypair
            utxoBlinding, index),
        ];
    }
}
exports.Utxo = Utxo;
exports.default = Utxo;
