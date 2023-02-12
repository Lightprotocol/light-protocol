const nacl = require("tweetnacl");
import { BN } from "@coral-xyz/anchor";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };
const circomlibjs = require("circomlibjs");
const ffjavascript = require("ffjavascript");

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
  poseidon: any;
  burnerSeed: Uint8Array;
  // keypair for eddsa poseidon signatures
  poseidonEddsa?: {
    publicKey?: [Uint8Array, Uint8Array];
    privateKey: Uint8Array;
  };
  eddsa: any;

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
  }: {
    poseidon?: any;
    seed?: string;
    burner?: Boolean;
    privateKey?: BN;
    publicKey?: BN;
    poseidonEddsaPrivateKey?: Uint8Array;
    eddsa?: any;
    encryptionPublicKey?: Uint8Array;
  }) {
    if (seed.length < 32) {
      throw "seed too short length less than 32";
    }
    if (poseidon) {
      this.poseidon = poseidon;
    }
    this.burnerSeed = new Uint8Array();
    // creates a burner utxo by using the index for domain separation
    if (burner) {
      // burnerSeed can be shared since hash cannot be inverted - only share this for app utxos
      // sharing the burnerSeed saves 32 bytes in onchain data if it is require to share both
      // the encryption and private key of a utxo
      this.burnerSeed = new BN(seed, "hex").toBuffer("be", 32);
      this.privkey = Account.generateShieldedPrivateKey(seed);
      this.encryptionKeypair = Account.getEncryptionKeyPair(seed);
      this.pubkey = Account.generateShieldedPublicKey(
        this.privkey,
        this.poseidon,
      );
      this.poseidonEddsa = Keypair.getEddsaPrivateKey(
        this.burnerSeed.toString(),
      );
    } else if (privateKey) {
      this.privkey = privateKey;
      this.pubkey = Account.generateShieldedPublicKey(
        this.privkey,
        this.poseidon,
      );
      if (poseidonEddsaPrivateKey) {
        this.poseidonEddsa = { privateKey: poseidonEddsaPrivateKey };
      }
      this.encryptionKeypair = {
        publicKey: encryptionPublicKey ? encryptionPublicKey : new Uint8Array(),
        secretKey: new Uint8Array(),
      };
    } else if (publicKey) {
      this.pubkey = publicKey;
      this.privkey = new BN("0");
      this.encryptionKeypair = {
        publicKey: encryptionPublicKey ? encryptionPublicKey : new Uint8Array(),
        secretKey: new Uint8Array(),
      };
    } else {
      this.privkey = Account.generateShieldedPrivateKey(seed);
      this.encryptionKeypair = Account.getEncryptionKeyPair(seed);
      this.pubkey = Account.generateShieldedPublicKey(
        this.privkey,
        this.poseidon,
      );
      this.poseidonEddsa = Keypair.getEddsaPrivateKey(seed);
    }
    this.eddsa = eddsa;
  }

  async getEddsaPublicKey(): Promise<[Uint8Array, Uint8Array]> {
    if (this.poseidonEddsa && this.eddsa) {
      this.poseidonEddsa.publicKey = this.eddsa.prv2pub(
        this.poseidonEddsa.privateKey,
      );
      if (this.poseidonEddsa.publicKey) {
        return this.poseidonEddsa.publicKey;
      } else {
        throw new Error("get poseidonEddsa.publicKey failed");
      }
    } else {
      throw new Error("poseidonEddsa.privateKey undefined");
    }
  }

  static getEddsaPrivateKey(seed: string) {
    const privkeySeed = seed + "poseidonEddsa";
    return {
      publicKey: undefined,
      privateKey: blake2b.create(b2params).update(privkeySeed).digest(),
    };
  }

  encryptionPublicKeyToBytes() {
    return new BN(this.encryptionKeypair.publicKey).toBuffer("be", 32);
  }

  // TODO: make eddsa wrapper class
  // TODO: include eddsa into static from methods
  async signEddsa(msg: string | Uint8Array, eddsa?: any): Promise<Uint8Array> {
    if (!this.eddsa) {
      if (!eddsa) {
        this.eddsa = eddsa;
      } else {
        throw new Error("Eddsa is not provided");
      }
    }
    if (this.poseidonEddsa) {
      if (typeof msg == "string") {
        return this.eddsa.packSignature(
          this.eddsa.signPoseidon(
            this.poseidonEddsa.privateKey,
            this.poseidon.F.e(ffjavascript.Scalar.e(msg)),
          ),
        );
      } else {
        return this.eddsa.packSignature(
          this.eddsa.signPoseidon(this.poseidonEddsa.privateKey, msg),
        );
      }
    } else {
      throw new Error("poseidonEddsa.privateKey undefined");
    }
  }

  /**
   * Sign a message using keypair private key
   *
   * @param {string|number|BigNumber} commitment a hex string with commitment
   * @param {string|number|BigNumber} merklePath a hex string with merkle path
   * @returns {BigNumber} a hex string with signature
   */
  sign(commitment: any, merklePath: any) {
    return this.poseidon.F.toString(
      this.poseidon([
        this.privkey.toString(),
        commitment.toString(),
        merklePath,
      ]),
    );
  }

  static createBurner(poseidon: any, seed: String, index: BN): Account {
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

  static fromPrivkey(poseidon: any, privateKey: Uint8Array): Account {
    const privkey = new BN(privateKey);
    return new Account({ poseidon, privateKey: privkey });
  }
  static fromPubkey(publicKey: Uint8Array, encPubkey: Uint8Array): Account {
    const pubKey = new BN(publicKey, undefined, "be");
    return new Account({ publicKey: pubKey, encryptionPublicKey: encPubkey });
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

  static generateShieldedPublicKey(privateKey: BN, poseidon: any): BN {
    return new BN(poseidon.F.toString(poseidon([privateKey])));
  }
}
