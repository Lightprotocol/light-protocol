const nacl = require("tweetnacl");
const anchor = require("@coral-xyz/anchor");
import { BN } from "@coral-xyz/anchor";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };

export class Keypair {
  /**
   * Initialize a new keypair. Generates a random private key if not defined
   *
   * @param {BN} privkey
   */
  privkey: BN;
  pubkey: BN;

  encryptionPublicKey: Uint8Array;
  encPrivateKey?: Uint8Array;
  poseidon: any;
  burnerSeed: Uint8Array;

  constructor({
    poseidon,
    seed = new BN(nacl.randomBytes(32)).toString("hex"),
    burner = false,
    privateKey,
    publicKey,
  }: {
    poseidon: any;
    seed?: string;
    burner?: Boolean;
    privateKey?: BN;
    publicKey?: BN;
  }) {
    if (seed.length < 32) {
      throw "seed too short length less than 32";
    }
    this.poseidon = poseidon;
    this.burnerSeed = new Uint8Array();
    // creates a burner utxo by using the index for domain separation
    if (burner) {
      // burnerSeed can be shared since hash cannot be inverted - only share this for app utxos
      // sharing the burnerSeed saves 32 bytes in onchain data if it is require to share both
      // the encryption and private key of a utxo
      this.burnerSeed = new BN(seed, "hex").toBuffer("be", 32);
      this.privkey = Keypair.generateShieldedPrivateKey(seed);
      this.encryptionPublicKey = Keypair.getEncryptionKeyPair(seed).publicKey;
      this.encPrivateKey = Keypair.getEncryptionKeyPair(seed).secretKey;
      this.pubkey = Keypair.generateShieldedPublicKey(
        this.privkey,
        this.poseidon,
      );
    } else if (privateKey) {
      this.privkey = privateKey;
      this.encryptionPublicKey = new Uint8Array();
      this.pubkey = Keypair.generateShieldedPublicKey(
        this.privkey,
        this.poseidon,
      );
    } else if (publicKey) {
      this.pubkey = publicKey;
      this.privkey = new BN("0");
      this.encryptionPublicKey = new Uint8Array();
    } else {
      this.privkey = Keypair.generateShieldedPrivateKey(seed);
      this.encryptionPublicKey = Keypair.getEncryptionKeyPair(seed).publicKey;
      this.encPrivateKey = Keypair.getEncryptionKeyPair(seed).secretKey;
      this.pubkey = Keypair.generateShieldedPublicKey(
        this.privkey,
        this.poseidon,
      );
    }
  }

  encryptionPublicKeyToBytes() {
    return new BN(this.encryptionPublicKey).toBuffer("be", 32);
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

  static createBurner(poseidon: any, seed: String, index: BN): Keypair {
    const burnerSeed = blake2b
      .create(b2params)
      .update(seed + "burnerSeed" + index.toString("hex"))
      .digest();
    const burnerSeedString = new BN(burnerSeed).toString("hex");

    return new Keypair({ poseidon, seed: burnerSeedString, burner: true });
  }

  static fromBurnerSeed(poseidon: any, burnerSeed: Uint8Array): Keypair {
    const burnerSeedString = new BN(burnerSeed).toString("hex");
    return new Keypair({ poseidon, seed: burnerSeedString, burner: true });
  }

  static fromPrivkey(poseidon: any, privateKey: Uint8Array): Keypair {
    const privkey = new BN(privateKey);
    return new Keypair({ poseidon, privateKey: privkey });
  }

  static fromPubkey(poseidon: any, publicKey: Uint8Array): Keypair {
    const pubKey = new BN(publicKey, undefined, "be");
    return new Keypair({ poseidon, publicKey: pubKey });
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
