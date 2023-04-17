const nacl = require("tweetnacl");
const { encrypt, decrypt } = require("ethereum-cryptography/aes");

import { BN } from "@coral-xyz/anchor";
import {
  AccountError,
  AccountErrorCode,
  TransactionParametersErrorCode,
  UtxoErrorCode,
} from "./index";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };
const ffjavascript = require("ffjavascript");
// @ts-ignore:
import { buildEddsa } from "circomlibjs";
import { PublicKey } from "@solana/web3.js";
// TODO: add fromPubkeyString()
export class Account {
  /**
   * Initialize a new shielded account. Generates a random private key if not defined
   *
   * @param {BN} privkey
   * @param {BN} pubkey
   */
  privkey: BN;
  pubkey: BN;
  encryptionKeypair: nacl.BoxKeyPair;
  burnerSeed: Uint8Array;
  // keypair for eddsa poseidon signatures
  poseidonEddsaKeypair?: {
    publicKey?: [Uint8Array, Uint8Array];
    privateKey: Uint8Array;
  };
  eddsa: any;
  aesSecret?: Uint8Array;

  /**
   *
   * @param poseidon required
   * @param seed
   * @param burner
   * @param privateKey
   * @param publicKey
   * @param encryptionPublicKey required for transfers to other users
   */
  constructor({
    poseidon,
    seed = new BN(nacl.randomBytes(32)).toString("hex"),
    burner = false,
    privateKey,
    publicKey,
    poseidonEddsaPrivateKey,
    eddsa,
    encryptionPublicKey,
    encryptionPrivateKey,
    aesSecret,
  }: {
    poseidon?: any;
    seed?: string;
    burner?: Boolean;
    privateKey?: BN;
    publicKey?: BN;
    poseidonEddsaPrivateKey?: Uint8Array;
    eddsa?: any;
    encryptionPublicKey?: Uint8Array;
    encryptionPrivateKey?: Uint8Array;
    aesSecret?: Uint8Array;
  }) {
    if (!poseidon) {
      throw new AccountError(
        TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
        "constructor",
      );
    }

    this.burnerSeed = new Uint8Array();
    // creates a burner utxo by using the index for domain separation
    if (burner) {
      if (!seed) {
        throw new AccountError(
          AccountErrorCode.SEED_UNDEFINED,
          "constructor",
          "seed is required to create a burner account",
        );
      }
      if (seed.length < 32) {
        throw new AccountError(
          AccountErrorCode.INVALID_SEED_SIZE,
          "constructor",
          "seed too short length less than 32",
        );
      }
      // burnerSeed can be shared since hash cannot be inverted - only share this for app utxos
      // sharing the burnerSeed saves 32 bytes in onchain data if it is require to share both
      // the encryption and private key of a utxo
      this.burnerSeed = new BN(seed, "hex").toBuffer("be", 32);
      this.privkey = Account.generateShieldedPrivateKey(seed);
      this.encryptionKeypair = Account.getEncryptionKeyPair(seed);
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      this.poseidonEddsaKeypair = Account.getEddsaPrivateKey(
        this.burnerSeed.toString(),
      );
      this.aesSecret = Account.generateAesSecret(this.burnerSeed.toString());
    } else if (privateKey) {
      if (!encryptionPrivateKey) {
        throw new AccountError(
          AccountErrorCode.ENCRYPTION_PRIVATE_KEY_UNDEFINED,
          "constructor",
        );
      }

      if (!aesSecret) {
        throw new AccountError(
          AccountErrorCode.AES_SECRET_UNDEFINED,
          "constructor",
        );
      }

      this.aesSecret = aesSecret;

      this.privkey = privateKey;
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      if (poseidonEddsaPrivateKey) {
        this.poseidonEddsaKeypair = { privateKey: poseidonEddsaPrivateKey };
      }
      this.encryptionKeypair =
        nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
    } else if (publicKey) {
      this.pubkey = publicKey;
      this.privkey = new BN("0");
      this.encryptionKeypair = {
        publicKey: encryptionPublicKey ? encryptionPublicKey : new Uint8Array(),
        secretKey: new Uint8Array(),
      };
    } else {
      if (!seed) {
        throw new AccountError(
          AccountErrorCode.SEED_UNDEFINED,
          "constructor",
          "seed is required to create an account",
        );
      }
      if (seed.length < 32) {
        throw new AccountError(
          AccountErrorCode.INVALID_SEED_SIZE,
          "constructor",
          "seed too short length less than 32",
        );
      }
      this.encryptionKeypair = Account.getEncryptionKeyPair(seed);
      this.privkey = Account.generateShieldedPrivateKey(seed);
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      this.poseidonEddsaKeypair = Account.getEddsaPrivateKey(seed);
      this.aesSecret = Account.generateAesSecret(seed);
    }
    this.eddsa = eddsa;
  }

  async getEddsaPublicKey(eddsa?: any): Promise<[Uint8Array, Uint8Array]> {
    if (!this.poseidonEddsaKeypair) {
      throw new AccountError(
        AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
        "getEddsaPublicKey",
        "poseidonEddsaKeypair.privateKey undefined",
      );
    }

    if (!this.eddsa) {
      if (eddsa) {
        this.eddsa = eddsa;
      } else {
        this.eddsa = await buildEddsa();
      }
    }

    this.poseidonEddsaKeypair.publicKey = this.eddsa.prv2pub(
      this.poseidonEddsaKeypair?.privateKey,
    );
    if (!this.poseidonEddsaKeypair.publicKey) {
      throw new AccountError(
        AccountErrorCode.POSEIDON_EDDSA_GET_PUBKEY_FAILED,
        "signEddsa",
        "poseidonEddsaKeypair.privateKey undefined",
      );
    }
    return this.poseidonEddsaKeypair.publicKey;
  }

  static getEddsaPrivateKey(seed: string) {
    const privkeySeed = seed + "poseidonEddsaKeypair";
    return {
      publicKey: undefined,
      privateKey: blake2b.create(b2params).update(privkeySeed).digest(),
    };
  }

  encryptionPublicKeyToBytes() {
    return new BN(this.encryptionKeypair.publicKey).toBuffer("be", 32);
  }

  // TODO: Add check for uint8array to be well formed
  async signEddsa(msg: string | Uint8Array, eddsa?: any): Promise<Uint8Array> {
    if (!this.poseidonEddsaKeypair) {
      throw new AccountError(
        AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
        "signEddsa",
        "poseidonEddsaKeypair.privateKey undefined",
      );
    }

    if (!this.eddsa) {
      if (eddsa) {
        this.eddsa = eddsa;
      } else {
        this.eddsa = await buildEddsa();
      }
    }

    if (typeof msg == "string") {
      return this.eddsa.packSignature(
        this.eddsa.signPoseidon(
          this.poseidonEddsaKeypair.privateKey,
          this.eddsa.F.e(ffjavascript.Scalar.e(msg)),
        ),
      );
    } else {
      return this.eddsa.packSignature(
        this.eddsa.signPoseidon(this.poseidonEddsaKeypair.privateKey, msg),
      );
    }
  }

  /**
   * Sign a message using keypair private key
   *
   * @param {string|number|BigNumber} commitment a hex string with commitment
   * @param {string|number|BigNumber} merklePath a hex string with merkle path
   * @returns {BigNumber} a hex string with signature
   */
  sign(poseidon: any, commitment: any, merklePath: any) {
    return poseidon.F.toString(
      poseidon([this.privkey.toString(), commitment.toString(), merklePath]),
    );
  }

  /**
   * @description gets domain separated aes secret to execlude the possibility of nonce reuse
   * @note we derive an aes key for every utxo of every merkle tree
   *
   * @param {PublicKey} merkleTreePdaPublicKey
   * @param {number} index the xth transaction for the respective merkle tree
   * @returns {Uint8Array} the blake2b hash of the aesSecret + merkleTreePdaPublicKey.toBase58() + index.toString()
   */
  getAesUtxoViewingKey(
    merkleTreePdaPublicKey: PublicKey,
    index: number,
  ): Uint8Array {
    return this.getDomainSeparatedAesSecretKey(
      merkleTreePdaPublicKey.toBase58() + index.toString(),
    );
  }

  getDomainSeparatedAesSecretKey(domain: string): Uint8Array {
    return blake2b
      .create(b2params)
      .update(this.aesSecret + domain)
      .digest();
  }

  static createBurner(poseidon: any, seed: String, index: BN): Account {
    if (seed.length < 32) {
      throw new AccountError(
        AccountErrorCode.INVALID_SEED_SIZE,
        "constructor",
        "seed too short length less than 32",
      );
    }
    const burnerSeed = blake2b
      .create(b2params)
      .update(seed + "burnerSeed" + index.toString("hex"))
      .digest();
    const burnerSeedString = new BN(burnerSeed).toString("hex");

    return new Account({ poseidon, seed: burnerSeedString, burner: true });
  }

  static fromBurnerSeed(poseidon: any, burnerSeed: Uint8Array): Account {
    const burnerSeedString = new BN(burnerSeed).toString("hex");
    return new Account({ poseidon, seed: burnerSeedString, burner: true });
  }

  static fromPrivkey(
    poseidon: any,
    privateKey: Uint8Array,
    encryptionPrivateKey: Uint8Array,
    aesSecret: Uint8Array,
  ): Account {
    const privkey = new BN(privateKey);
    return new Account({
      poseidon,
      privateKey: privkey,
      encryptionPrivateKey,
      aesSecret,
    });
  }

  static fromPubkey(
    publicKey: Uint8Array,
    encPubkey: Uint8Array,
    poseidon: any,
  ): Account {
    const pubKey = new BN(publicKey, undefined, "be");
    return new Account({
      publicKey: pubKey,
      encryptionPublicKey: encPubkey,
      poseidon,
    });
  }

  static getEncryptionKeyPair(seed: String): nacl.BoxKeyPair {
    const encSeed = seed + "encryption";
    const encryptionPrivateKey = blake2b
      .create(b2params)
      .update(encSeed)
      .digest();
    return nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
  }

  static generateShieldedPrivateKey(seed: String): BN {
    const privkeySeed = seed + "shielded";
    const privateKey = new BN(
      blake2b.create(b2params).update(privkeySeed).digest(),
    );
    return privateKey;
  }

  static generateAesSecret(seed: String, domain?: string): Uint8Array {
    const privkeySeed = seed + "aes";
    return Uint8Array.from(
      blake2b.create(b2params).update(privkeySeed).digest(),
    );
  }

  static generateShieldedPublicKey(privateKey: BN, poseidon: any): BN {
    return new BN(poseidon.F.toString(poseidon([privateKey])));
  }

  async encryptAes(
    message: Uint8Array,
    iv: Uint8Array,
    domain: string = "default",
  ) {
    if (iv.length != 16)
      throw new AccountError(
        UtxoErrorCode.INVALID_NONCE_LENGHT,
        "encryptAes",
        `Required iv length 16, provided ${iv.length}`,
      );
    const secretKey = this.getDomainSeparatedAesSecretKey(domain);
    const ciphertext = await encrypt(
      message,
      secretKey,
      iv,
      "aes-256-cbc",
      true,
    );
    return new Uint8Array([...iv, ...ciphertext]);
  }

  async decryptAes(encryptedBytes: Uint8Array, domain: string = "default") {
    const iv = encryptedBytes.subarray(0, 16);
    const encryptedMessageBytes = encryptedBytes.subarray(16);
    const secretKey = this.getDomainSeparatedAesSecretKey(domain);
    const cleartext = await decrypt(
      encryptedMessageBytes,
      secretKey,
      iv,
      "aes-256-cbc",
      true,
    );
    return cleartext;
  }

  // TODO: add static encryptTo function to account
  // static encryptNacl()
  // decryptNacl()
}
