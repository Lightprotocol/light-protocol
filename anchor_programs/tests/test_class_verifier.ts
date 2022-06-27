import { poseidonHash } from "./utils/poseidonHash";
import { randomBN } from "./utils/randomBN";
import { toBuffer } from "./utils/toBuffer";

import { BigNumber } from 'ethers';
const nacl = require("tweetnacl");
nacl.util = require("tweetnacl-util");

import { Keypair } from "./utils/keypair";

class Utxo {
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


  constructor(
    amount: number | BigNumber = 0,
    keypair: Keypair = new Keypair(), // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding: BigNumber = randomBN(),
    index: number | null = null,
    _commitment: BigNumber | null = null, // I added null as default if there is an error could be that
    _nullifier: BigNumber | null = null,) { // I added null as default if there is an error could be that
    this.amount = BigNumber.from(amount);
    this.blinding = BigNumber.from(blinding);
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
  getCommitment(): BigNumber | null {
    if (!this._commitment) {
      this._commitment = poseidonHash([
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
  getNullifier(): BigNumber | null {
    if (!this._nullifier) {
      if (
        this.amount > 0 &&
        (this.index === undefined ||
          this.index === null ||
          this.keypair.privkey === undefined ||
          this.keypair.privkey === null)
      ) {
        throw new Error(
          "Can not compute nullifier without utxo index or private key",
        );
      }

      const signature = this.keypair.privkey
        ? this.keypair.sign(this.getCommitment(), this.index || 0)
        : 0;
      this._nullifier = poseidonHash([
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
  encrypt(nonce: Uint8Array, encryptionKeypair: nacl.BoxKeyPair, senderThrowAwayKeypair: nacl.BoxKeyPair) {

    const bytes_message: Uint8Array = Buffer.concat([
      toBuffer(this.blinding, 31),
      toBuffer(this.amount, 8),
    ]);

    console.log("bytes_message", bytes_message)
    console.log("nonce", nonce)
    console.log("encryptionKeypair", encryptionKeypair)
    console.log("senderThrowAwayKeypair", senderThrowAwayKeypair)

    const ciphertext: Uint8Array = nacl.box(
      bytes_message,
      nonce,
      encryptionKeypair.publicKey,
      senderThrowAwayKeypair.secretKey,
    );
    console.log("CIPHERTEXT", ciphertext)
    console.log("CIPHERTEXT TYPE", typeof ciphertext)
    return ciphertext;
  }

  static decrypt(
    encryptedUtxo: any,
    nonce: any,
    senderThrowAwayPubkey: any,
    recipientEncryptionKeypair: any,
    shieldedKeypair: any,
    index: any,
  ) {
    const cleartext = nacl.box.open(
      encryptedUtxo,
      nonce,
      senderThrowAwayPubkey,
      recipientEncryptionKeypair.secretKey,
    );

    if (!cleartext) {
      return [false, null];
    }
    const buf = Buffer.from(cleartext);
    const utxoAmount = BigNumber.from("0x" + buf.slice(31, 39).toString("hex"));
    const utxoBlinding = BigNumber.from("0x" + buf.slice(0, 31).toString("hex"));
    return [
      true,
      new Utxo(
        utxoAmount,
        shieldedKeypair, // only recipient can decrypt, has full keypair
        utxoBlinding,
        index,
      ),
    ];
  }

}
export default Utxo;
