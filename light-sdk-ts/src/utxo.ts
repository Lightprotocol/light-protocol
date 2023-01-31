import { Keypair } from "./keypair";
import nacl, { box } from "tweetnacl";
const crypto = require("crypto");
const randomBN = (nbytes = 30) => new anchor.BN(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;
const anchor = require("@coral-xyz/anchor");
import {
  fetchAssetByIdLookUp,
  getAssetIndex,
  hashAndTruncateToCircuit,
} from "./utils";
import { PublicKey, SystemProgram } from "@solana/web3.js";
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;

import { BN } from "@coral-xyz/anchor";
import { CONSTANT_SECRET_AUTHKEY } from "./constants";
import { MINT } from "./index";
import { hex } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { assert } from "chai";
export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
// TODO: move to constants
export const N_ASSETS = 2;
export const N_ASSET_PUBKEYS = 3;

// TODO: write test
export class Utxo {
  /** Initialize a new UTXO - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
   *
   * @param {new BN[]} amounts array UTXO amount
   * @param {new BN | new BN | number | string} blinding Blinding factor
   */

  amounts: BN[];
  assets: PublicKey[];
  assetsCircuit: BN[];
  blinding: BN;
  keypair: Keypair;
  index?: number;
  appData: Array<any>;
  verifierAddress: PublicKey;
  verifierAddressCircuit: BN;
  instructionType: BN;
  poolType: BN;
  _commitment: BN | null;
  _nullifier: BN | null;
  poseidon: any;
  includeAppData: boolean;

  constructor({
    poseidon,
    // TODO: reduce to one (the first will always be 0 and the third is not necessary)
    assets = [SystemProgram.programId],
    amounts = [new BN("0")],
    keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding = new BN(randomBN(), 31, "be"),
    poolType = new BN("0"),
    verifierAddress = SystemProgram.programId,
    appData = [],
    appDataFromBytesFn,
    index,
    includeAppData = false,
  }: {
    poseidon: any;
    assets?: PublicKey[];
    amounts?: BN[];
    keypair?: Keypair; // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding?: BN;
    poolType?: BN;
    verifierAddress?: PublicKey;
    appData?: Array<any>;
    appDataFromBytesFn?: Function;
    index?: number;
    includeAppData?: boolean;
  }) {
    // check that blinding is 31 bytes
    blinding.toArray("be", 31);
    if (assets.length != amounts.length) {
      throw `utxo constructor: asset.length  ${assets.length}!= amount.length ${amounts.length}`;
    }
    if (assets.length > N_ASSETS) {
      throw `assets.lengt ${assets.length} > N_ASSETS ${N_ASSETS}`;
    }

    while (assets.length < N_ASSETS) {
      assets.push(SystemProgram.programId);
    }

    for (var i = 0; i < N_ASSETS; i++) {
      if (amounts[i] < new BN(0)) {
        throw `utxo constructor: amount cannot be negative, amounts[${i}] = ${amounts[i]}`;
      }
    }

    while (amounts.length < N_ASSETS) {
      amounts.push(new BN(0));
    }
    if (!keypair) {
      keypair = new Keypair({ poseidon });
    }

    // TODO: check that this does not lead to hickups since publicAmount cannot withdraw the fee asset sol
    if (assets[1].toBase58() == SystemProgram.programId.toBase58()) {
      amounts[0] = amounts[0].add(amounts[1]);
      amounts[1] = new BN(0);
    } else {
    }

    this.amounts = amounts.map((x) => {
      try {
        // check that amounts are U64
        // TODO: add test
        x.toArray("be", 8);
      } catch (_) {
        throw new Error("amount not u64");
      }

      return new BN(x.toString());
    });
    this.blinding = blinding;
    this.keypair = keypair;
    this.index = index;
    this.assets = assets;
    this._commitment = null;
    this._nullifier = null;
    this.poseidon = poseidon;
    this.appData = appData;
    this.poolType = poolType;
    this.includeAppData = includeAppData;

    // TODO: make variable length
    // TODO: evaluate whether to hashAndTruncate feeAsset as well
    if (assets[1].toBase58() != SystemProgram.programId.toBase58()) {
      this.assetsCircuit = [
        hashAndTruncateToCircuit(SystemProgram.programId.toBytes()),
        hashAndTruncateToCircuit(this.assets[1].toBytes()),
      ];
    } else if (this.amounts[0].toString() === "0") {
      this.assetsCircuit = [new BN(0), new BN(0)];
    } else {
      this.assetsCircuit = [
        hashAndTruncateToCircuit(SystemProgram.programId.toBytes()),
        new BN(0),
      ];
    }

    if (verifierAddress.toBase58() == SystemProgram.programId.toBase58()) {
      this.verifierAddress = verifierAddress;
      this.verifierAddressCircuit = new BN(0);
    } else {
      this.verifierAddress = verifierAddress;
      this.verifierAddressCircuit = hashAndTruncateToCircuit(
        verifierAddress.toBytes(),
      );
    }
    if (appData.length > 0) {
      // TODO: change to poseidon hash which is reproducable in circuit
      // TODO: write function which creates the instructionTypeHash
      if (appDataFromBytesFn) {
        // console.log(
        //   "appDataFromBytesFn(appData) ",
        //   appDataFromBytesFn(appData).map((x) => x.toString()),
        // );
        // console.log("poseidon.F.toString ", poseidon.F.toString(
        //   poseidon(appDataFromBytesFn(appData))));

        this.instructionType = new BN(
          leInt2Buff(
            unstringifyBigInts(
              poseidon.F.toString(poseidon(appDataFromBytesFn(appData))),
            ),
            32,
          ),
          undefined,
          "le",
        );
      } else {
        throw new Error("No appDataFromBytesFn provided");
      }
    } else {
      this.instructionType = new BN("0");
    }
  }

  toBytes() {
    //TODO: get assetIndex(this.asset[1])
    const assetIndex = getAssetIndex(this.assets[1]);
    if (assetIndex.toString() == "-1") {
      throw new Error("Asset not found in lookup table");
    }

    // case no appData
    if (!this.includeAppData) {
      return new Uint8Array([
        ...this.blinding.toArray("be", 31),
        ...this.amounts[0].toArray("be", 8),
        ...this.amounts[1].toArray("be", 8),
        ...new BN(assetIndex).toArray("be", 8),
      ]);
    }

    return new Uint8Array([
      ...this.blinding.toArray("be", 31),
      ...this.amounts[0].toArray("be", 8),
      ...this.amounts[1].toArray("be", 8),
      ...assetIndex.toArray("be", 8),
      ...this.instructionType.toArray("be", 32),
      ...this.poolType.toArray("be", 8),
      ...this.verifierAddress.toBytes(),
      ...new Array(1),
      ...this.appData,
    ]);
  }

  // take a decrypted byteArray as input
  // TODO: make robust and versatile for any combination of filled in fields or not
  // TODO: find a better solution to get the private key in
  // TODO: check length to rule out parsing app utxo
  static fromBytes({
    poseidon,
    bytes,
    keypair,
    keypairInAppDataOffset,
    appDataLength,
    appDataFromBytesFn,
    includeAppData = false,
  }: {
    poseidon: any;
    bytes: Uint8Array;
    keypair?: Keypair;
    keypairInAppDataOffset?: number;
    appDataLength?: number;
    appDataFromBytesFn?: Function;
    includeAppData?: boolean;
  }): Utxo {
    const blinding = new BN(bytes.slice(0, 31), undefined, "be"); // blinding
    const amounts = [
      new BN(bytes.slice(31, 39), undefined, "be"),
      new BN(bytes.slice(39, 47), undefined, "be"),
    ]; // amounts
    const assets = [
      SystemProgram.programId,
      fetchAssetByIdLookUp(new BN(bytes.slice(47, 55), undefined, "be")),
    ]; // assets MINT

    if (keypair) {
      return new Utxo({
        poseidon,
        assets,
        amounts,
        keypair,
        blinding,
        includeAppData,
      });
    } else {
      const instructionType = new BN(bytes.slice(55, 87), 32, "be");

      const poolType = new BN(bytes.slice(87, 95), 8, "be");
      const verifierAddress = new PublicKey(bytes.slice(95, 127));
      // ...new Array(1), separator is otherwise 0
      const appData = bytes.slice(128, bytes.length);
      const burnerKeypair = Keypair.fromPrivkey(
        poseidon,
        appData.slice(72, 104),
      );
      return new Utxo({
        poseidon,
        assets,
        amounts,
        keypair: burnerKeypair,
        blinding,
        instructionType,
        appData,
        appDataFromBytesFn,
        verifierAddress,
        includeAppData,
      });
    }
  }

  /**
   * Returns commitment for this UTXO
   *signature:
   * @returns {BN}
   */
  getCommitment() {
    if (!this._commitment) {
      let amountHash = this.poseidon.F.toString(this.poseidon(this.amounts));
      let assetHash = this.poseidon.F.toString(
        this.poseidon(this.assetsCircuit.map((x) => x.toString())),
      );
      // console.log("this.assetsCircuit ", this.assetsCircuit);

      // console.log("amountHash ", amountHash.toString());
      // console.log("this.keypair.pubkey ", this.keypair.pubkey.toString());
      // console.log("this.blinding ", this.blinding.toString());
      // console.log("assetHash ", assetHash.toString());
      // console.log("this.instructionType ", this.instructionType.toString());
      // console.log("this.poolType ", this.poolType.toString());

      this._commitment = this.poseidon.F.toString(
        this.poseidon([
          amountHash,
          this.keypair.pubkey.toString(),
          this.blinding.toString(),
          assetHash.toString(),
          this.instructionType.toString(),
          this.poolType,
          this.verifierAddressCircuit,
        ]),
      );
    }
    return this._commitment;
  }

  /**
   * Returns nullifier for this UTXO
   *
   * @returns {BN}
   */
  getNullifier(index?: number | undefined) {
    if (!this.index) {
      this.index = index;
    }
    if (!this._nullifier) {
      if (
        //(this.amounts[0] > new BN(0) || this.amounts[0] > new BN(0))
        false &&
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
      // console.log("this.getCommitment() ", this.getCommitment());
      // console.log("this.index || 0 ", this.index || 0);
      // console.log("signature ", signature);

      this._nullifier = this.poseidon.F.toString(
        this.poseidon([this.getCommitment(), this.index || 0, signature]),
      );
    }
    // console.log("this._nullifier ", this._nullifier);

    return this._nullifier;
  }

  /**
   * Encrypt UTXO to recipient pubkey
   *
   * @returns {string}
   */
  // TODO: add fill option to 128 bytes to be filled with 0s
  // TODO: add encrypt custom (app utxos with idl)
  encrypt() {
    const bytes_message = this.toBytes();
    const nonce = newNonce();
    // CONSTANT_SECRET_AUTHKEY is used to minimize the number of bytes sent to the blockchain.
    // This results in poly135 being useless since the CONSTANT_SECRET_AUTHKEY is public.
    // However, ciphertext integrity is guaranteed since a hash of the ciphertext is included in a zero-knowledge proof.
    const ciphertext = box(
      bytes_message,
      nonce,
      this.keypair.encryptionKeypair.publicKey,
      CONSTANT_SECRET_AUTHKEY,
    );

    return new Uint8Array([...ciphertext, ...nonce]);
  }

  // TODO: add decrypt custom (app utxos with idl)
  static decrypt({
    poseidon,
    encBytes,
    keypair,
  }: {
    poseidon: any;
    encBytes: Uint8Array;
    keypair: Keypair;
  }): Utxo | null {
    const encryptedUtxo = new Uint8Array(Array.from(encBytes.slice(0, 71)));
    const nonce = new Uint8Array(Array.from(encBytes.slice(71, 71 + 24)));

    if (keypair.encryptionKeypair.secretKey) {
      const cleartext = box.open(
        encryptedUtxo,
        nonce,
        nacl.box.keyPair.fromSecretKey(CONSTANT_SECRET_AUTHKEY).publicKey,
        keypair.encryptionKeypair.secretKey,
      );
      if (!cleartext) {
        return null;
      }
      const bytes = Buffer.from(cleartext);
      return Utxo.fromBytes({ poseidon, bytes, keypair });
    } else {
      return null;
    }
  }

  static equal(utxo0: Utxo, utxo1: Utxo) {
    assert.equal(utxo0.amounts[0].toString(), utxo1.amounts[0].toString());
    assert.equal(utxo0.amounts[1].toString(), utxo1.amounts[1].toString());
    assert.equal(utxo0.assets[0].toBase58(), utxo1.assets[0].toBase58());
    assert.equal(utxo0.assets[1].toBase58(), utxo1.assets[1].toBase58());
    assert.equal(
      utxo0.assetsCircuit[0].toString(),
      utxo1.assetsCircuit[0].toString(),
    );
    assert.equal(
      utxo0.assetsCircuit[1].toString(),
      utxo1.assetsCircuit[1].toString(),
    );
    assert.equal(
      utxo0.instructionType.toString(),
      utxo1.instructionType.toString(),
    );
    assert.equal(utxo0.poolType.toString(), utxo1.poolType.toString());
    assert.equal(
      utxo0.verifierAddress.toString(),
      utxo1.verifierAddress.toString(),
    );
    assert.equal(
      utxo0.verifierAddressCircuit.toString(),
      utxo1.verifierAddressCircuit.toString(),
    );
    assert.equal(
      utxo0.getCommitment()?.toString(),
      utxo1.getCommitment()?.toString(),
    );
    assert.equal(
      utxo0.getNullifier()?.toString(),
      utxo1.getNullifier()?.toString(),
    );
  }
}

exports.Utxo = Utxo;
