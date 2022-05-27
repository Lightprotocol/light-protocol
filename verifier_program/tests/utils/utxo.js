// const { ethers } = require("hardhat");
const { ethers } = require("ethers");
const { BigNumber } = ethers;
const nacl = require("tweetnacl");
nacl.util = require("tweetnacl-util");

// const BigNumber = require("bignumber.js");
const { randomBN, poseidonHash, toBuffer, toFixedHex } = require("./utils");
const { Keypair } = require("./keypair");

const randomKeypair = () => nacl.box.keyPair();
class Utxo {
  /** Initialize a new UTXO - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
   *
   * @param {BigNumber | BigInt | number | string} amount UTXO amount
   * @param {BigNumber | BigInt | number | string} blinding Blinding factor
   * @param {number|null} index UTXO index in the merkle tree
   */
  constructor({
    amount = 0,
    keypair = new Keypair(), // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding = randomBN(),
    index = null,
  } = {}) {
    this.amount = BigNumber.from(amount);
    this.blinding = BigNumber.from(blinding);
    this.keypair = keypair;
    this.index = index;
  }

  /**
   * Returns commitment for this UTXO
   *
   * @returns {BigNumber}
   */
  getCommitment() {
    if (!this._commitment) {
      this._commitment = poseidonHash([
        this.amount,
        this.blinding,
        this.keypair.pubkey,
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
      // if (
      //   this.amount > 0 &&
      //   (this.index === undefined ||
      //     this.index === null ||
      //     this.keypair.privkey === undefined ||
      //     this.keypair.privkey === null)
      // ) {
      //   throw new Error("Can not compute nullifier without utxo index");
      // }
      this._nullifier = poseidonHash([
        this.getCommitment(),
        this.index || 0,
        this.keypair.privkey || 0,
      ]);
    }
    return this._nullifier;
  }

  /**
   * Encrypt UTXO to recipient pubkey
   *
   * @returns {string}
   */
  encrypt(nonce, recipientEncryptionPubkey, senderThrowAwayKeypair) {
    const bytes = Buffer.concat([
      toBuffer(this.blinding, 31),
      toBuffer(this.amount, 8),
    ]);

    var ciphertext = nacl.box(
      bytes,
      nonce,
      recipientEncryptionPubkey,
      senderThrowAwayKeypair.secretKey,
    );
    console.log("ciphertext: ", ciphertext);
    return ciphertext;
  }

  static decrypt(
    encryptedUtxo,
    nonce,
    senderThrowAwayPubkey,
    recipientEncryptionKeypair,
    shieldedKeypair,
    index,
  ) {
    var cleartext = nacl.box.open(
      encryptedUtxo,
      nonce,
      senderThrowAwayPubkey,
      recipientEncryptionKeypair.secretKey,
    );

    if (!cleartext) {
      return [false, null];
    }
    console.log(cleartext);
    var buf = Buffer.from(cleartext);
    // var buf = Buffer.from(decipher.output.toHex(), "hex");

    return [
      true,
      new Utxo({
        blinding: BigNumber.from("0x" + buf.slice(0, 31).toString("hex")),
        amount: BigNumber.from("0x" + buf.slice(31, 39).toString("hex")),
        keypair: shieldedKeypair, // only recipient can decrypt, has full keypair
        index,
      }),
    ];
  }

  //   static decryptFromString(ciphertext, privkey, index) {
  //     let buf = Buffer.from(ciphertext, "hex");
  //     var decipher = forge.cipher.createDecipher("AES-CBC", key);
  //     decipher.start({ iv: iv });
  //     decipher.update(forge.util.createBuffer(buf));
  //     var result = decipher.finish();
  //     console.log(
  //       "decr buff from hex:",
  //       Buffer.from(decipher.output.toHex(), "hex"),
  //     );

  //     if (!result) {
  //       return false;
  //     }
  //     buf = Buffer.from(decipher.output.toHex(), "hex");

  //     return new Utxo({
  //       blinding: BigNumber.from("0x" + buf.slice(0, 31).toString("hex")),
  //       amount: BigNumber.from("0x" + buf.slice(31, 39).toString("hex")),
  //       keypair: {
  //         privkey: "0x" + buf.slice(39, 71).toString("hex"),
  //         pubkey: poseidonHash(["0x" + buf.slice(39, 71).toString("hex")]),
  //       },
  //       index,
  //     });
  //   }

  //   /**
  //    * Decrypt a UTXO
  //    *
  //    * @param {string} data hex string with data
  //    * @param {number} index UTXO index in merkle tree
  //    * @returns {Utxo}
  //    */
  //   static decrypt(data, privkey, index) {
  //     var bytes = CryptoJS.AES.decrypt(data, privkey);
  //     var originalText = bytes.toString(CryptoJS.enc.Utf8);
  //     var buf = Buffer.from(JSON.parse(originalText).data); // og buf that was encrypted

  //     // const buf = toBuffer(data, 39);

  //     return new Utxo({
  //       blinding: BigNumber.from("0x" + buf.slice(0, 31).toString("hex")),
  //       amount: BigNumber.from("0x" + buf.slice(31, 39).toString("hex")),
  //       // keypair: BigNumber.from("0x" + buf.slice(39, 71).toString("hex")),
  //       keypair: {
  //         privkey: BigNumber.from("0x" + buf.slice(39, 71).toString("hex")),
  //         pubkey: poseidonHash([
  //           BigNumber.from("0x" + buf.slice(39, 71).toString("hex")),
  //         ]),
  //       },
  //       index,
  //     });
  //   }
  // }

}
module.exports = Utxo;
