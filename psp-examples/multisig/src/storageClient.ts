import { Account, CONSTANT_SECRET_AUTHKEY } from "@lightprotocol/zk.js";
import nacl from "tweetnacl";
const { blake2b } = require("@noble/hashes/blake2b");
const { encrypt, decrypt } = require("ethereum-cryptography/aes");
const b2params24 = { dkLen: 24 };
const b2params16 = { dkLen: 16 };

const bs58 = require("bs58");

// TODO:
// add encrypt to primitive, nonces are H(base_nonce||pubkey), H(base_nonce||pubkey), H(base_nonce||pubkey), etc.
// if only one recipient use nonce directly
// fields [base_nonce], [encryptedAes1,..., encryptedAesN ], [aesCiphertext],
// standardized, the first 32 bytes are the pubkey,
// encrypt to aes

export class StorageUtils {
  account: Account;

  constructor(account: Account) {
    this.account = account;
  }

  /**
   * @description 1. Generate random Aes key
   * @description 2. encrypt aes key to every account
   * @description 3. encrypt message to aes key
   * @param {Account[]} recipients recipients the message is encrypted to with a shared aes key
   * @param {Uint8Array} message which will be encrypted with aes
   * @returns {Uint8Array} with the layout [baseNonce(32), aesKeyCipherTexts(48 * x), aesNonce(16), aesCipherText]
   */
  static async encryptTo(
    recipientPublicKeys: Uint8Array[],
    message: Uint8Array,
    baseNonce?: Uint8Array,
    aesSecretKey?: Uint8Array
  ) {
    if (!aesSecretKey) {
      aesSecretKey = nacl.randomBytes(32);
    }

    if (!baseNonce) {
      baseNonce = nacl.randomBytes(32);
    }
    let i = 0;
    let encryptedAesKeys: Uint8Array[] = [];
    for (const [index, publicKey] of recipientPublicKeys.entries()) {
      let nonce = blake2b
        .create(b2params24)
        .update(bs58.encode(baseNonce) + index.toString())
        .digest();

      encryptedAesKeys.push(
        Account.encryptNacl(
          publicKey,
          aesSecretKey,
          CONSTANT_SECRET_AUTHKEY,
          false,
          nonce,
          true
        )
      );
      i = index;
    }

    let iv = blake2b
      .create(b2params16)
      .update(bs58.encode(baseNonce) + (i + 1).toString())
      .digest();

    const ciphertext = await encrypt(
      message,
      aesSecretKey,
      iv,
      "aes-256-cbc",
      true
    );
    const ciphertextWithIV = new Uint8Array([...iv, ...ciphertext]);

    return Uint8Array.from([
      ...baseNonce,
      ...encryptedAesKeys.map((x) => Array.from(x)).flat(),
      ...ciphertextWithIV,
    ]);
  }

  /**
   * @description Decrypts a byte array with the layout [baseNonce(32), aesKeyCipherTexts(48 * x), aesNonce(16), aesCipherText]
   * @param {Account[]} recipients recipients the message is encrypted to with a shared aes key
   * @param {Uint8Array} message
   */
  static async decryptMultipleRecipients(
    account: Account,
    ciphertext: Uint8Array
  ): Promise<Uint8Array> {
    const baseNonce = ciphertext.slice(0, 32);
    const ciphertextPublicKeys = ciphertext.slice(32);
    let recipientCount = 0;
    const publicKeyCiphertextLength = 48;
    let encryptedAesKey: Uint8Array | undefined;

    // Iterate over ciphertext until we find aes iv to determine the number of users the aes secret key is encrypted to
    for (
      var i = 0;
      i < ciphertextPublicKeys.length / publicKeyCiphertextLength;
      i++
    ) {
      const nonce = blake2b
        .create(b2params24)
        .update(bs58.encode(baseNonce) + i.toString())
        .digest();
      const nonce16 = blake2b
        .create(b2params16)
        .update(bs58.encode(baseNonce) + i.toString())
        .digest();

      const encryptedAesKeyCandidate = ciphertextPublicKeys.slice(
        i * publicKeyCiphertextLength,
        (i + 1) * publicKeyCiphertextLength
      );
      const decryptedAesKeyCandidate = await account.decryptNacl(
        encryptedAesKeyCandidate,
        nonce,
        nacl.box.keyPair.fromSecretKey(CONSTANT_SECRET_AUTHKEY).publicKey
      );
      if (decryptedAesKeyCandidate.value) {
        encryptedAesKey = decryptedAesKeyCandidate.value;
      }
      // check whether we found the aes iv which means we tried to decrypt all publickeys
      if (
        nonce16.toString() ===
        ciphertextPublicKeys
          .slice(
            i * publicKeyCiphertextLength,
            i * publicKeyCiphertextLength + 16
          )
          .toString()
      ) {
        break;
      }
      recipientCount++;
    }

    if (!encryptedAesKey) {
      throw new Error(
        "Failed to decrypt the AES key with the provided secret key"
      );
    }

    const startAesCiphertext = recipientCount * publicKeyCiphertextLength;
    const ciphertextAes = ciphertextPublicKeys.slice(startAesCiphertext);
    const iv16 = ciphertextAes.slice(0, 16);

    return await decrypt(
      ciphertextAes,
      encryptedAesKey,
      iv16,
      "aes-256-cbc",
      true
    );
  }
}
