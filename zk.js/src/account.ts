const nacl = require("tweetnacl");
import { box } from "tweetnacl";
const { encrypt, decrypt } = require("ethereum-cryptography/aes");

import { BN } from "@coral-xyz/anchor";
import {
  AccountError,
  AccountErrorCode,
  TransactionParametersErrorCode,
  UtxoErrorCode,
  BN_0,
} from "./index";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };
const ffjavascript = require("ffjavascript");
const buildEddsa = require("circomlibjs").buildEddsa;
import { PublicKey } from "@solana/web3.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
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
  hashingSecret?: Uint8Array;
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
    seed = bs58.encode(nacl.randomBytes(32)),
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
    if (aesSecret && !privateKey) {
      this.aesSecret = aesSecret;
      this.privkey = BN_0;
      this.pubkey = BN_0;
      this.encryptionKeypair = {
        publicKey: new Uint8Array(32),
        secretKey: new Uint8Array(32),
      };
    }

    // creates a burner utxo by using the index for domain separation
    else if (burner) {
      if (!seed) {
        throw new AccountError(
          AccountErrorCode.SEED_UNDEFINED,
          "constructor",
          "seed is required to create a burner account",
        );
      }
      if (bs58.decode(seed).length < 32) {
        throw new AccountError(
          AccountErrorCode.INVALID_SEED_SIZE,
          "constructor",
          "seed too short length less than 32",
        );
      }
      // burnerSeed can be shared since hash cannot be inverted - only share this for app utxos
      // sharing the burnerSeed saves 32 bytes in onchain data if it is require to share both
      // the encryption and private key of a utxo
      this.burnerSeed = new BN(bs58.decode(seed)).toArrayLike(Buffer, "be", 32);
      this.privkey = Account.generateShieldedPrivateKey(seed, poseidon);
      this.encryptionKeypair = Account.getEncryptionKeyPair(seed);
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      this.poseidonEddsaKeypair = Account.getEddsaPrivateKey(
        this.burnerSeed.toString(),
      );
      this.aesSecret = Account.generateSecret(
        b2params.dkLen,
        this.burnerSeed.toString(),
        "aes",
      );
      this.hashingSecret = Account.generateSecret(
        b2params.dkLen,
        this.burnerSeed.toString(),
        "hashing",
      );
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
      this.privkey = BN_0;
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
      this.privkey = Account.generateShieldedPrivateKey(seed, poseidon);
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      this.poseidonEddsaKeypair = Account.getEddsaPrivateKey(seed);
      this.aesSecret = Account.generateSecret(b2params.dkLen, seed, "aes");
      this.hashingSecret = Account.generateSecret(
        b2params.dkLen,
        seed,
        "hashing",
      );
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
    return new BN(this.encryptionKeypair.publicKey).toArrayLike(
      Buffer,
      "be",
      32,
    );
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
    salt: string,
  ): Uint8Array {
    return Account.generateSecret(
      b2params.dkLen,
      this.aesSecret?.toString(),
      merkleTreePdaPublicKey.toBase58() + salt,
    );
  }

  getUtxoPrefixViewingKey(salt: string): Uint8Array {
    return Account.generateSecret(
      b2params.dkLen,
      this.hashingSecret?.toString(),
      salt,
    );
  }

  generateUtxoPrefixHash(commitmentHash: Uint8Array, dkLen?: number) {
    const input = Uint8Array.from([
      ...this.getUtxoPrefixViewingKey("hashing"),
      ...commitmentHash,
    ]);

    return blake2b.create({ dkLen }).update(input).digest();
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
      .update(seed + "burnerSeed" + index.toString())
      .digest();
    const burnerSeedString = bs58.encode(burnerSeed);

    return new Account({ poseidon, seed: burnerSeedString, burner: true });
  }

  static fromBurnerSeed(poseidon: any, burnerSeed: string): Account {
    return new Account({ poseidon, seed: burnerSeed, burner: true });
  }

  static fromPrivkey(
    poseidon: any,
    privateKey: string,
    encryptionPrivateKey: string,
    aesSecret: string,
  ): Account {
    if (!privateKey) {
      throw new AccountError(
        AccountErrorCode.PRIVATE_KEY_UNDEFINED,
        "constructor",
      );
    }
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
    const privkey = new BN(bs58.decode(privateKey));
    return new Account({
      poseidon,
      privateKey: privkey,
      encryptionPrivateKey: bs58.decode(encryptionPrivateKey),
      aesSecret: bs58.decode(aesSecret),
    });
  }

  getPrivateKeys(): {
    privateKey: string;
    encryptionPrivateKey: string;
    aesSecret: string;
  } {
    if (!this.aesSecret) {
      throw new AccountError(
        AccountErrorCode.AES_SECRET_UNDEFINED,
        "getPrivateKeys",
      );
    }
    return {
      privateKey: bs58.encode(this.privkey.toArray("be", 32)),
      encryptionPrivateKey: bs58.encode(this.encryptionKeypair.secretKey),
      aesSecret: bs58.encode(this.aesSecret),
    };
  }

  static fromPubkey(publicKey: string, poseidon: any): Account {
    let decoded = bs58.decode(publicKey);
    if (decoded.length != 64)
      throw new AccountError(
        AccountErrorCode.INVALID_PUBLIC_KEY_SIZE,
        "fromPubkey",
        `Expected publickey size 64 bytes as bs58 encoded string, the first 32 bytes are the shielded publickey the second 32 bytes are the encryption publickey provided length ${decoded} string ${publicKey}`,
      );

    const pubKey = new BN(decoded.subarray(0, 32), undefined, "be");
    return new Account({
      publicKey: pubKey,
      encryptionPublicKey: decoded.subarray(32, 64),
      poseidon,
    });
  }

  getPublicKey(): string {
    let concatPublicKey = new Uint8Array([
      ...this.pubkey.toArray("be", 32),
      ...this.encryptionKeypair.publicKey,
    ]);
    return bs58.encode(concatPublicKey);
  }

  static getEncryptionKeyPair(seed: String): nacl.BoxKeyPair {
    const encSeed = seed + "encryption";
    const encryptionPrivateKey = blake2b
      .create(b2params)
      .update(encSeed)
      .digest();
    return nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
  }

  static generateShieldedPrivateKey(seed: String, poseidon: any): BN {
    const privkeySeed = seed + "shielded";
    const privateKey = new BN(
      poseidon.F.toString(
        poseidon([
          new BN(blake2b.create(b2params).update(privkeySeed).digest()),
        ]),
      ),
    );
    return privateKey;
  }

  static generateSecret(
    dkLen: number,
    seed?: String,
    domain?: String,
  ): Uint8Array {
    return Uint8Array.from(
      blake2b.create({ dkLen }).update(`${seed}${domain}`).digest(),
    );
  }

  static generateShieldedPublicKey(privateKey: BN, poseidon: any): BN {
    return new BN(poseidon.F.toString(poseidon([privateKey])));
  }

  static async encryptAes(
    aesSecret: Uint8Array,
    message: Uint8Array,
    iv: Uint8Array,
  ) {
    if (iv.length != 16)
      throw new AccountError(
        UtxoErrorCode.INVALID_NONCE_LENGTH,
        "encryptAes",
        `Required iv length 16, provided ${iv.length}`,
      );
    const secretKey = aesSecret;
    const ciphertext = await encrypt(
      message,
      secretKey,
      iv,
      "aes-256-cbc",
      true,
    );
    return new Uint8Array([...iv, ...ciphertext]);
  }

  static async decryptAes(aesSecret: Uint8Array, encryptedBytes: Uint8Array) {
    const iv = encryptedBytes.slice(0, 16);
    const encryptedMessageBytes = encryptedBytes.slice(16);
    const secretKey = aesSecret;
    const cleartext = await decrypt(
      encryptedMessageBytes,
      secretKey,
      iv,
      "aes-256-cbc",
      true,
    );
    return cleartext;
  }

  static encryptNacl(
    publicKey: Uint8Array,
    message: Uint8Array,
    signerSecretKey: Uint8Array,
    nonce?: Uint8Array,
    returnWithoutNonce?: boolean,
  ): Uint8Array {
    if (!nonce) {
      nonce = nacl.randomBytes(nacl.nonceLength);
    }
    const ciphertext = box(message, nonce!, publicKey, signerSecretKey);

    if (returnWithoutNonce) {
      return Uint8Array.from([...ciphertext]);
    }
    return Uint8Array.from([...nonce!, ...ciphertext]);
  }

  decryptNacl(
    ciphertext: Uint8Array,
    nonce?: Uint8Array,
    signerpublicKey?: Uint8Array,
  ): Uint8Array | null {
    if (!nonce) {
      nonce = ciphertext.slice(0, 24);
      ciphertext = ciphertext.slice(24);
    }
    if (!signerpublicKey) {
      signerpublicKey = ciphertext.slice(0, 32);
      ciphertext = ciphertext.slice(32);
    }

    return nacl.box.open(
      ciphertext,
      nonce,
      signerpublicKey,
      this.encryptionKeypair.secretKey,
    );
  }
}
