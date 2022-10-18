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
      console.log(assets)
        if (assets.length != amounts.length) {
          throw `utxo constructor: asset.length  ${assets.length}!= amount.length ${amounts.length}`;
        }
        console.log("assets", assets)
        while (assets.length < N_ASSETS) {
          assets.push(new anchor.BN(0))
          console.log("pushing asset")

        }
        for (var i= 0; i < N_ASSETS; i++) {
          if (amounts[i] < 0) {
              throw `utxo constructor: amount cannot be negative, amounts[${i}] = ${amounts[i]}`
          }
        }
        console.log("amounts", amounts)

        while (amounts.length < N_ASSETS) {
          amounts.push(0)
          console.log("pushing amount")
        }
        console.log("here1");
        if (keypair == null) {
          keypair = new keypair_1.Keypair(poseidon)

        }
        console.log("here2");

        // I added null as default if there is an error could be that
        this.amounts = amounts.map((x) => new anchor.BN(x));
        console.log("here3");

        //console.log("feeAmount", feeAmount)
        // this.feeAmount = new anchor.BN(feeAmount);
        this.blinding = new anchor.BN(blinding);
        console.log("here4");

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
          console.log("getting this.amounts", this.amounts)
          console.log("getting amountHash", amountHash)

          let assetHash = this.poseidon.F.toString(this.poseidon(this.assets));
          console.log("getting assetHash", assetHash)

          this._commitment = this.poseidon.F.toString(this.poseidon([
              amountHash,
              // this.feeAmount,
              this.keypair.pubkey,
              this.blinding, // is instructionDataHash
              assetHash,
              this.instructionType
          ]));
          console.log("getting commitment: ", this._commitment)


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
            // console.log("getting nf ", this.keypair)
            const signature = this.keypair.privkey
                ? this.keypair.sign(this.getCommitment(), this.index || 0)
                : 0;
            console.log("signature", signature)

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
        // console.log("bytes_message", bytes_message)
        // console.log("nonce", nonce)
        // console.log("encryptionKeypair", encryptionKeypair)
        // console.log("senderThrowAwayKeypair", senderThrowAwayKeypair)
        const ciphertext = nacl.box(bytes_message, nonce, encryptionKeypair.publicKey, senderThrowAwayKeypair.secretKey);
        // console.log("CIPHERTEXT", ciphertext)
        // console.log("CIPHERTEXT TYPE", typeof ciphertext)
        return ciphertext;
    }
    static decrypt(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair, shieldedKeypair, assets = [], POSEIDON, index) {
      console.log("jhere");
        const cleartext = nacl.box.open(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair.secretKey);
        console.log(cleartext);
        if (!cleartext) {
            return [false, null];
        }
        const buf = Buffer.from(cleartext);
        console.log(buf);
        const utxoAmount1 = new anchor.BN(Array.from(buf.slice(31, 39)).reverse());
        const utxoAmount2 = new anchor.BN(Array.from(buf.slice(39, 47)).reverse());

        const utxoBlinding = new anchor.BN( buf.slice(0, 31));
        console.log(assets);
        return [
            true,
            // {
            // utxoAmount1,
            // utxoAmount2,
            // shieldedKeypair,
            // utxoBlinding,
            // index}
            new Utxo(POSEIDON, assets, [utxoAmount1, utxoAmount2], shieldedKeypair,"0", utxoBlinding, index)
            // only recipient can decrypt, has full keypair
            // utxoBlinding, index),
        ];
    }
}
exports.Utxo = Utxo;
exports.default = Utxo;
