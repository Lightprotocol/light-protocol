const { encrypt, decrypt, getEncryptionPublicKey } = require("eth-sig-util");
const solana = require("@solana/web3.js");

const { ethers } = require("ethers");
const { BigNumber } = ethers;
const { poseidonHash, toFixedHex } = require("./utils");

function packEncryptedMessage(encryptedMessage) {
  const nonceBuf = Buffer.from(encryptedMessage.nonce, "base64");
  const ephemPublicKeyBuf = Buffer.from(
    encryptedMessage.ephemPublicKey,
    "base64",
  );
  const ciphertextBuf = Buffer.from(encryptedMessage.ciphertext, "base64");
  const messageBuff = Buffer.concat([
    Buffer.alloc(24 - nonceBuf.length),
    nonceBuf,
    Buffer.alloc(32 - ephemPublicKeyBuf.length),
    ephemPublicKeyBuf,
    ciphertextBuf,
  ]);
  return "0x" + messageBuff.toString("hex");
}

function unpackEncryptedMessage(encryptedMessage) {
  if (encryptedMessage.slice(0, 2) === "0x") {
    encryptedMessage = encryptedMessage.slice(2);
  }
  const messageBuff = Buffer.from(encryptedMessage, "hex");
  const nonceBuf = messageBuff.slice(0, 24);
  const ephemPublicKeyBuf = messageBuff.slice(24, 56);
  const ciphertextBuf = messageBuff.slice(56);
  return {
    version: "x25519-xsalsa20-poly1305",
    nonce: nonceBuf.toString("base64"),
    ephemPublicKey: ephemPublicKeyBuf.toString("base64"),
    ciphertext: ciphertextBuf.toString("base64"),
  };
}

class Keypair {
  /**
   * Initialize a new keypair. Generates a random private key if not defined
   *
   * @param {string} privkey
   */
  constructor(privkey = ethers.Wallet.createRandom().privateKey) {
    this.privkey = privkey;
    this.pubkey = poseidonHash([this.privkey]);
    this.encryptionKey = getEncryptionPublicKey(privkey.slice(2));
  }

  toString() {
    return (
      toFixedHex(this.pubkey) +
      Buffer.from(this.encryptionKey, "base64").toString("hex")
    );
  }

  /**
   * Key address for this keypair, alias to {@link toString}
   *
   * @returns {string}
   */
  address() {
    return this.toString();
  }

  /**
   * Initialize new keypair from address string
   *
   * @param str
   * @returns {Keypair}
   */
  static fromString(str) {
    if (str.length === 130) {
      str = str.slice(2);
    }
    if (str.length !== 128) {
      throw new Error("Invalid key length");
    }
    return Object.assign(new Keypair(), {
      privkey: null,
      pubkey: BigNumber.from("0x" + str.slice(0, 64)),
      encryptionKey: Buffer.from(str.slice(64, 128), "hex").toString("base64"),
    });
  }

  /**
   * Encrypt data using keypair encryption key
   *
   * @param {Buffer} bytes
   * @returns {string} a hex string with encrypted data
   */
  encrypt(bytes) {
    return packEncryptedMessage(
      encrypt(
        this.encryptionKey,
        { data: bytes.toString("base64") },
        "x25519-xsalsa20-poly1305",
      ),
    );
  }

  /**
   * Decrypt data using keypair private key
   *
   * @param {string} data a hex string with data
   * @returns {Buffer}
   */
  decrypt(data) {
    return Buffer.from(
      decrypt(unpackEncryptedMessage(data), this.privkey.slice(2)),
      "base58",
    );
  }
}

module.exports = {
  Keypair,
  packEncryptedMessage,
  unpackEncryptedMessage,
};
// export { Keypair, packEncryptedMessage, unpackEncryptedMessage };
