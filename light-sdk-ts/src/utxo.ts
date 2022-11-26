import { Keypair } from './keypair'
import { BigNumber } from 'ethers'
const nacl = require('tweetnacl')
nacl.util = require('tweetnacl-util')
const crypto = require('crypto');
const randomBN = (nbytes = 31) => new anchor.BN(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;
const anchor = require("@project-serum/anchor")
import {toBufferLE} from 'bigint-buffer';
import { LogLevel } from '@ethersproject/logger';

const N_ASSETS = 3;
export class Utxo {
  /** Initialize a new UTXO - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
   *
   * @param {BigNumber} amount UTXO amount
   * @param {BigNumber | BigInt | number | string} blinding Blinding factor
   */

  amount: BigNumber
  blinding: BigNumber
  keypair: Keypair
  index: number | null
  // _commitment: BigNumber | null
  // _nullifier: BigNumber | null

  constructor(
    poseidon,
    assets = [0, 0, 0],
    amounts = [0, 0, 0],
    keypair = null, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    instructionType = "0",
    blinding = randomBN(),
    index = null,
    _commitment = null,
    _nullifier = null
  ) {
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
      keypair = new Keypair(poseidon)
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
      console.log("at least asset missing in encrypted bytes");

      // TODO: add asset to encrypted bytes
      // TODO: if other stuff is missing
      const bytes_message = new Uint8Array([
          ...this.blinding.toBuffer(),
          ...toBufferLE(BigInt(this.amounts[0]), 8),
          ...toBufferLE(BigInt(this.amounts[1]), 8)
      ]);
      console.log("bytes_message ", bytes_message);
      console.log("encryptionKeypair ", encryptionKeypair);
      console.log("senderThrowAwayKeypair ", senderThrowAwayKeypair);
      
      
      const ciphertext = nacl.box(bytes_message, nonce, encryptionKeypair.PublicKey, senderThrowAwayKeypair.secretKey);

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
