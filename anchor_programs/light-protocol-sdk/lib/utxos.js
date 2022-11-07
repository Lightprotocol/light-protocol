"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Utxo = void 0;
const poseidon = require("./utils/poseidonHash");
const randomBN_1 = require("./utils/randomBN");
const toBuffer_1 = require("./utils/toBuffer");
const nacl = require('tweetnacl');
nacl.util = require('tweetnacl-util');
const keypair_1 = require("./utils/keypair");
const constants_1 = require("./constants");
const toFixedHex_1 = require("./utils/toFixedHex");
const anchor = require("@project-serum/anchor")
const toBufferLE = require('bigint-buffer');
// import { BigNumber } from 'ethers';

const N_ASSETS = 3;
class Utxo {
    constructor(poseidon, assets = [0, 0, 0],amounts = [0, 0, 0],/*feeAmount = 0, */keypair = null, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    instructionType = "0", blinding = (0, randomBN_1.randomBN)(), index = null, _commitment = null, // I added null as default if there is an error could be that
    _nullifier = null) {
        if (assets.length != amounts.length) {
          throw `utxo constructor: asset.length  ${assets.length}!= amount.length ${amounts.length}`;
        }
        while (assets.length < N_ASSETS) {
          assets.push(new anchor.BN(0))
        }
        for (var i= 0; i < N_ASSETS; i++) {
          if (amounts[i] < 0) {
              throw `utxo constructor: amount cannot be negative, amounts[${i}] = ${amounts[i]}`
          }
        }

        while (amounts.length < N_ASSETS) {
          amounts.push(0)
        }
        if (keypair == null) {
          keypair = new keypair_1.Keypair(poseidon)

        }

        this.amounts = amounts.map((x) => new anchor.BN(x));
        this.blinding = new anchor.BN(blinding);
        this.keypair = keypair;
        this.index = index;
        this.assets = assets.map((x) => new anchor.BN(x));
        this.instructionType = instructionType;
        this._commitment = _commitment;
        this._nullifier = _nullifier;
        this.poseidon = poseidon;
    }
    /**
     * Returns commitment for this UTXO
     *signature:
     * @returns {BigNumber}
     */
    getCommitment() {
        if (!this._commitment) {
          let amountHash = this.poseidon.F.toString(this.poseidon(this.amounts));

          let assetHash = this.poseidon.F.toString(this.poseidon(this.assets));
          this._commitment = this.poseidon.F.toString(this.poseidon([
              amountHash,
              this.keypair.pubkey,
              this.blinding,
              assetHash,
              this.instructionType
          ]));


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

            this._nullifier = this.poseidon.F.toString(this.poseidon([
                this.getCommitment(),
                this.index || 0,
                signature,
            ]))
        }
        return this._nullifier;
    }
    /**
     * Encrypt UTXO to recipient pubkey
     *
     * @returns {string}
     */
    encrypt(nonce, encryptionKeypair, senderThrowAwayKeypair) {
        console.log(this.amounts[0]);

        const bytes_message = Buffer.concat([
            this.blinding.toBuffer(),
            toBufferLE.toBufferLE(BigInt(this.amounts[0]), 8),
            toBufferLE.toBufferLE(BigInt(this.amounts[1]), 8)
        ]);
        const ciphertext = nacl.box(bytes_message, nonce, encryptionKeypair.publicKey, senderThrowAwayKeypair.secretKey);

        return ciphertext;
    }
    static decrypt(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair, shieldedKeypair, assets = [], POSEIDON, index) {

        const cleartext = nacl.box.open(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair.secretKey);
        if (!cleartext) {
            return [false, null];
        }
        const buf = Buffer.from(cleartext);
        const utxoAmount1 = new anchor.BN(Array.from(buf.slice(31, 39)).reverse());
        const utxoAmount2 = new anchor.BN(Array.from(buf.slice(39, 47)).reverse());

        const utxoBlinding = new anchor.BN( buf.slice(0, 31));

        return [
            true,
            new Utxo(POSEIDON, assets, [utxoAmount1, utxoAmount2], shieldedKeypair,"0", utxoBlinding, index)
        ];
    }
}
exports.Utxo = Utxo;
exports.default = Utxo;
