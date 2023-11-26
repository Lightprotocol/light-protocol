import { BN, BorshAccountsCoder, utils } from "@coral-xyz/anchor";
import { IDL } from "./types/multisig";
import { Account } from "@lightprotocol/zk.js";
import * as nacl from "tweetnacl";
import { MAX_SIGNERS } from "./constants";
import { assert } from "chai";
const bs58 = require("bs58");

// TODO:
// add encrypt to primitive, nonces are H(base_nonce||pubkey), H(base_nonce||pubkey), H(base_nonce||pubkey), etc.
// if only one recipient use nonce directly
// fields [base_nonce], [encryptedAes1,..., encryptedAesN ], [aesCiphertext],
// standardized, the first 32 bytes are the pubkey,
// encrypt to aes

export class MultisigParams {
  signersEncryptionPublicKeys: Array<Uint8Array>;
  threshold: BN;
  publicKeyX: Array<Uint8Array>;
  publicKeyY: Array<Uint8Array>;
  poseidon: any;
  nrSigners: BN;
  appDataHash?: string;
  seed: Uint8Array;
  account: Account;
  priorMultiSigSlot: BN;
  priorMultiSigHash: Uint8Array;
  priorMultiSigSeed: Uint8Array;

  constructor({
    poseidon,
    threshold,
    nrSigners,
    publicKeyX,
    publicKeyY,
    signersEncryptionPublicKeys,
    seed,
    priorMultiSigSlot,
    priorMultiSigHash,
    priorMultiSigSeed,
  }: {
    poseidon: any;
    threshold: number;
    nrSigners: number;
    publicKeyX: Array<Uint8Array>;
    publicKeyY: Array<Uint8Array>;
    signersEncryptionPublicKeys: Array<Uint8Array>;
    seed: Uint8Array;
    priorMultiSigSlot: BN;
    priorMultiSigHash: Uint8Array;
    priorMultiSigSeed: Uint8Array;
  }) {
    this.threshold = new BN(threshold);
    this.publicKeyX = publicKeyX;
    this.publicKeyY = publicKeyY;
    this.nrSigners = new BN(nrSigners);
    this.signersEncryptionPublicKeys = signersEncryptionPublicKeys;
    this.appDataHash = MultisigParams.getHash(
      poseidon,
      MultisigParams.toArray(
        poseidon,
        threshold,
        nrSigners,
        publicKeyX,
        publicKeyY,
      ),
    );
    this.seed = seed;
    this.account = new Account({ poseidon, seed: bs58.encode(seed) });
    this.priorMultiSigSlot = priorMultiSigSlot;
    this.priorMultiSigHash = priorMultiSigHash;
    this.priorMultiSigSeed = priorMultiSigSeed;
  }

  static async createNewMultiSig({
    poseidon,
    signers,
    threshold,
    randomSeed = nacl.randomBytes(32),
  }: {
    poseidon: any;
    signers: Account[];
    threshold: number;
    randomSeed?: Uint8Array;
  }) {
    if (signers.length > MAX_SIGNERS) {
      throw new Error(`Too many signers ${signers.length} > 6`);
    }
    if (threshold > signers.length) {
      throw new Error(
        `Not enough signers ${signers.length} for threshold ${threshold}`,
      );
    }
    if (poseidon === undefined) {
      throw new Error("Poseidon instance not defined");
    }
    const dummyAccount = new Account({
      poseidon,
      seed: new Uint8Array(32).fill(3).toString(),
    });
    dummyAccount.poseidonEddsaKeypair = {
      publicKey: [new Uint8Array(32).fill(0), new Uint8Array(32).fill(0)],
      privateKey: new Uint8Array(32).fill(0),
    };

    let publicKeyX: Uint8Array[] = [];
    let publicKeyY: Uint8Array[] = [];

    for (let signer of signers) {
      const pubkey = await signer.getEddsaPublicKey();
      publicKeyX.push(pubkey[0]);
      publicKeyY.push(pubkey[1]);
    }

    while (MAX_SIGNERS > signers.length) {
      signers.push(dummyAccount);
      publicKeyX.push(new Uint8Array(32).fill(0));
      publicKeyY.push(new Uint8Array(32).fill(0));
    }

    const signersEncryptionPublicKeys = signers.map(
      (signer) => signer.encryptionKeypair.publicKey,
    );

    return new MultisigParams({
      poseidon,
      threshold,
      nrSigners: signers.length,
      publicKeyX,
      publicKeyY,
      signersEncryptionPublicKeys,
      seed: randomSeed,
      priorMultiSigSlot: new BN(0),
      priorMultiSigHash: new Uint8Array(32).fill(0),
      priorMultiSigSeed: new Uint8Array(32).fill(0),
    });
  }

  async toBytes(): Promise<Buffer> {
    let coder = new BorshAccountsCoder(IDL);
    return await coder.encode("createMultiSig", this);
  }

  static fromBytes(poseidon, bytes: Buffer): MultisigParams {
    let coder = new BorshAccountsCoder(IDL);
    let decoded = coder.decode("createMultiSig", bytes);
    return new MultisigParams({ poseidon, ...decoded });
  }

  print() {
    console.log("----------------- MultiSig Parameters -----------------");
    console.log("Threshold: ", this.threshold.toString());
    console.log("Number of Signers: ", this.nrSigners.toString());
    console.log(`Shielded pubkey: ${this.appDataHash}`);
    console.log("Shared encryption public key: <encryption-key>");
    console.log("Shared encryption private key: <encryption-key>");

    for (
      let i = 0;
      i < Math.min(this.nrSigners.toNumber(), this.publicKeyX.length);
      i++
    ) {
      console.log(
        `Signer: ${i}`,
        utils.bytes.hex.encode(
          Buffer.from(
            Array.from([...this.publicKeyX[i], ...this.publicKeyY[i]]).flat(),
          ),
        ),
      );
    }
  }

  debugString(): string {
    let log = "----------------- MultiSig Parameters -----------------\n";
    log += "threshold: " + this.threshold.toString() + "\n";
    log += "Number of Signers: " + this.nrSigners.toString() + "\n";
    log += "Shielded pubkey: " + this.appDataHash + "\n";

    for (
      let i = 0;
      Math.min(this.nrSigners.toNumber(), this.publicKeyX.length);
      i++
    ) {
      log +=
        "Signer: " +
        i +
        utils.bytes.hex.encode(
          Buffer.from(
            Array.from([...this.publicKeyX[i], ...this.publicKeyY[i]]).flat(),
          ),
        ) +
        "\n";
    }
    return log;
  }

  static toArray(poseidon, threshold, nrSigners, publicKeyX, publicKeyY) {
    return [
      new BN(threshold),
      new BN(nrSigners),
      ...publicKeyX.map((s) =>
        new BN(poseidon.F.toString(s)).toArrayLike(Buffer, "le", 32),
      ),
      ...publicKeyY.map((s) =>
        new BN(poseidon.F.toString(s)).toArrayLike(Buffer, "be", 32),
      ),
    ];
  }

  static getHash(poseidon, array) {
    return poseidon.F.toString(poseidon(array));
  }

  static equal(multiSig1: MultisigParams, multiSig2: MultisigParams) {
    multiSig1.publicKeyX.map((key, i) =>
      assert.equal(
        key.toString(),
        multiSig2.publicKeyX[i].toString(),
        `invalid publicKeyX key ${i}`,
      ),
    );
    multiSig1.publicKeyY.map((key, i) =>
      assert.equal(
        key.toString(),
        multiSig2.publicKeyY[i].toString(),
        `invalid publicKeyY key ${i}`,
      ),
    );
    multiSig1.signersEncryptionPublicKeys.map((key, i) =>
      assert.equal(
        key.toString(),
        multiSig2.signersEncryptionPublicKeys[i].toString(),
        `invalid encryption key ${i}`,
      ),
    );
    assert.equal(
      multiSig1.priorMultiSigSlot.toString(),
      multiSig2.priorMultiSigSlot.toString(),
      "invalid priorMultiSigSlot",
    );
    assert.equal(
      multiSig1.priorMultiSigHash.toString(),
      multiSig2.priorMultiSigHash.toString(),
      "invalid priorMultiSigHash",
    );
    assert.equal(
      multiSig1.priorMultiSigSeed.toString(),
      multiSig2.priorMultiSigSeed.toString(),
      "invalid priorMultiSigSeed",
    );
    assert.equal(
      multiSig1.threshold.toString(),
      multiSig2.threshold.toString(),
      "invalid threshold",
    );
    assert.equal(
      multiSig1.nrSigners.toString(),
      multiSig2.nrSigners.toString(),
      "invalid nrSigners",
    );

    assert.equal(
      multiSig1.seed.toString(),
      multiSig2.seed.toString(),
      "invalid seed",
    );
    assert.equal(
      multiSig1.appDataHash.toString(),
      multiSig2.appDataHash.toString(),
      "invalid appDataHash",
    );
  }
}
