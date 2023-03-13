import { Account } from "./account";
import nacl, { box } from "tweetnacl";

const randomBN = (nbytes = 30) => {
  return new anchor.BN(nacl.randomBytes(nbytes));
};
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
import { assert } from "chai";
import { UtxoError, UtxoErrorCode } from "./errors";
export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
// TODO: move to constants
export const N_ASSETS = 2;
export const N_ASSET_PUBKEYS = 3;

export class Utxo {
  /**
   * @param {BN[]} amounts array of utxo amounts, amounts[0] is the sol amount amounts[1] is the spl amount
   * @param {PublicKey[]} assets  array of utxo assets, assets[0] is the sol asset assets[1] is the spl asset
   * @param {BN} blinding Blinding factor, a 31 bytes big number, to add randomness to the commitment hash.
   * @param {Account} account the account owning the utxo.
   * @param {index} index? the index of the utxo's commitment hash in the Merkle tree.
   * @param {Array<any>} appData application data of app utxos not provided for normal utxos.
   * @param {PublicKey} verifierAddress the solana address of the verifier, SystemProgramId/BN(0) for system verifiers.
   * @param {BN} verifierAddressCircuit hashAndTruncateToCircuit(verifierAddress) to fit into 254 bit field size of bn254.
   * @param {BN} instructionType is the poseidon hash of app utxo data. This compresses the app data and ties it to the app utxo.
   * @param {BN} poolType is the pool type domain of the utxo default is [0;32].
   * @param {any} poseidon poseidon hasher instance.
   * @param {boolean} includeAppData flag whether to include app data when serializing utxo to bytes.
   * @param {string} _commitment cached commitment hash to avoid recomputing.
   * @param {string} _nullifier cached nullifier hash to avoid recomputing.
   */
  amounts: BN[];
  assets: PublicKey[];
  assetsCircuit: BN[];
  blinding: BN;
  account: Account;
  index?: number;
  appData: Array<any>;
  verifierAddress: PublicKey;
  verifierAddressCircuit: BN;
  instructionType: BN;
  poolType: BN;
  _commitment?: string;
  _nullifier?: string;
  poseidon: any;
  includeAppData: boolean;

  /**
   * @description Initialize a new utxo - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
   *
   * @param {BN[]} amounts array of utxo amounts, amounts[0] is the sol amount amounts[1] is the spl amount
   * @param {PublicKey[]} assets  array of utxo assets, assets[0] is the sol asset assets[1] is the spl asset
   * @param {BN} blinding Blinding factor, a 31 bytes big number, to add randomness to the commitment hash.
   * @param {Account} account the account owning the utxo.
   * @param {index} index? the index of the utxo's commitment hash in the Merkle tree.
   * @param {Array<any>} appData application data of app utxos not provided for normal utxos.
   * @param {PublicKey} verifierAddress the solana address of the verifier, SystemProgramId/BN(0) for system verifiers.
   * @param {BN} instructionType is the poseidon hash of app utxo data. This compresses the app data and ties it to the app utxo.
   * @param {any} poseidon poseidon hasher instance.
   * @param {boolean} includeAppData flag whether to include app data when serializing utxo to bytes.
   * @param {function} appDataFromBytesFn function to deserialize appData from bytes.
   * @param {appData} appData array of application data, is used to compute the instructionDataHash.
   */
  constructor({
    poseidon,
    // TODO: reduce to one (the first will always be 0 and the third is not necessary)
    assets = [SystemProgram.programId],
    amounts = [new BN("0")],
    account,
    blinding = new BN(randomBN(), 31, "be"),
    poolType = new BN("0"),
    verifierAddress = SystemProgram.programId,
    appData = [],
    appDataFromBytesFn,
    index,
    includeAppData = false,
    instructionType,
  }: {
    poseidon: any;
    assets?: PublicKey[];
    amounts?: BN[];
    account?: Account;
    blinding?: BN;
    poolType?: BN;
    verifierAddress?: PublicKey;
    appData?: Array<any>;
    appDataFromBytesFn?: Function;
    index?: number;
    includeAppData?: boolean;
    instructionType?: BN;
  }) {
    // check that blinding is 31 bytes
    try {
      blinding.toArray("be", 31);
    } catch (_) {
      throw new UtxoError(
        UtxoErrorCode.BLINDING_EXCEEDS_SIZE,
        "constructor",

        `Bliding ${blinding}, exceeds size of 31 bytes/248 bit.`,
      );
    }
    if (assets.length != amounts.length) {
      throw new UtxoError(
        UtxoErrorCode.INVALID_ASSET_OR_AMOUNTS_LENGTH,
        "constructor",

        `Length missmatch assets: ${assets.length} != amounts: ${amounts.length}`,
      );
    }
    if (assets.length > N_ASSETS) {
      throw new UtxoError(
        UtxoErrorCode.EXCEEDED_MAX_ASSETS,
        "constructor",

        `assets.length ${assets.length} > N_ASSETS ${N_ASSETS}`,
      );
    }

    while (assets.length < N_ASSETS) {
      assets.push(SystemProgram.programId);
    }

    for (var i = 0; i < N_ASSETS; i++) {
      if (amounts[i] < new BN(0)) {
        throw new UtxoError(
          UtxoErrorCode.NEGATIVE_AMOUNT,
          "constructor",

          `amount cannot be negative, amounts[${i}] = ${amounts[i]}`,
        );
      }
    }

    while (amounts.length < N_ASSETS) {
      amounts.push(new BN(0));
    }
    if (!account) {
      account = new Account({ poseidon });
    }

    // TODO: check that this does not lead to hickups since publicAmount cannot withdraw the fee asset sol
    if (assets[1].toBase58() == SystemProgram.programId.toBase58()) {
      amounts[0] = amounts[0].add(amounts[1]);
      amounts[1] = new BN(0);
    }

    // checks that amounts are U64
    this.amounts = amounts.map((x) => {
      try {
        x.toArray("be", 8);
      } catch (_) {
        throw new UtxoError(
          UtxoErrorCode.NOT_U64,
          "constructor",
          `amount ${x} not a u64`,
        );
      }
      return new BN(x.toString());
    });

    this.blinding = blinding;
    this.account = account;
    this.index = index;
    this.assets = assets;
    this.poseidon = poseidon;
    this.appData = appData;
    this.poolType = poolType;
    this.includeAppData = includeAppData;

    // TODO: make variable length
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
      if (appDataFromBytesFn && !instructionType) {
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
      } else if (instructionType) {
        this.instructionType = instructionType;
      } else {
        throw new UtxoError(
          UtxoErrorCode.APP_DATA_FROM_BYTES_FUNCTION_UNDEFINED,
          "constructor",
          "No appDataFromBytesFn provided",
        );
      }
    } else {
      this.instructionType = new BN("0");
    }
  }

  /**
   * @description Parses a utxo to bytes.
   * @returns {Uint8Array}
   */
  toBytes() {
    const assetIndex = getAssetIndex(this.assets[1]);
    if (assetIndex.toString() == "-1") {
      throw new UtxoError(
        UtxoErrorCode.ASSET_NOT_FOUND,
        "toBytes",
        "Asset not found in lookup table",
      );
    }

    // case no or excluding appData
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

  /**
   * @description Parses a utxo from bytes.
   * @param poseidon poseidon hasher instance
   * @param bytes byte array of a serialized utxo
   * @param account account of the utxo
   * @param appDataFromBytesFn function to parse app data from bytes
   * @param includeAppData whether to include app data when encrypting or not
   * @returns {Utxo}
   */
  // TODO: make robust and versatile for any combination of filled in fields or not
  // TODO: find a better solution to get the private key in
  // TODO: check length to rule out parsing app utxo
  // TODO: validate account
  // TODO: add generic utxo type which builds from idl
  static fromBytes({
    poseidon,
    bytes,
    account,
    appDataFromBytesFn,
    includeAppData = false,
    index,
  }: {
    poseidon: any;
    bytes: Uint8Array;
    account?: Account;
    appDataFromBytesFn?: Function;
    includeAppData?: boolean;
    index: number;
  }): Utxo {
    const blinding = new BN(bytes.slice(0, 31), undefined, "be");
    const amounts = [
      new BN(bytes.slice(31, 39), undefined, "be"),
      new BN(bytes.slice(39, 47), undefined, "be"),
    ];
    const assets = [
      SystemProgram.programId,
      fetchAssetByIdLookUp(new BN(bytes.slice(47, 55), undefined, "be")),
    ];

    if (account) {
      return new Utxo({
        poseidon,
        assets,
        amounts,
        account,
        blinding,
        includeAppData,
        index,
      });
    }
    // TODO: add identifier that utxo is app utxo
    // TODO: add option to use account abstraction standard pubkey new BN(0)
    else {
      const instructionType = new BN(bytes.slice(55, 87), 32, "be");

      const poolType = new BN(bytes.slice(87, 95), 8, "be");
      const verifierAddress = new PublicKey(bytes.slice(95, 127));
      // ...new Array(1), separator is otherwise 0
      const appData = bytes.slice(128, bytes.length);
      const burnerAccount = Account.fromPrivkey(
        poseidon,
        appData.slice(72, 104),
      );
      return new Utxo({
        poseidon,
        assets,
        amounts,
        account: burnerAccount,
        blinding,
        instructionType,
        appData: Array.from([...appData]),
        appDataFromBytesFn,
        verifierAddress,
        includeAppData,
        index,
      });
    }
  }

  /**
   * @description Returns commitment for this utxo
   * @description PoseidonHash(amountHash, shieldedPubkey, blinding, assetHash, instructionType, poolType, verifierAddressCircuit)
   * @returns {string}
   */
  getCommitment(): string {
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
      let commitment: string = this.poseidon.F.toString(
        this.poseidon([
          amountHash,
          this.account.pubkey.toString(),
          this.blinding.toString(),
          assetHash.toString(),
          this.instructionType.toString(),
          this.poolType,
          this.verifierAddressCircuit,
        ]),
      );
      this._commitment = commitment;
      return this._commitment;
    } else {
      return this._commitment;
    }
  }

  /**
   * @description Computes the nullifier for this utxo.
   * @description PoseidonHash(commitment, index, signature)
   * @param {number} index Merkle tree index of the utxo commitment (Optional)
   *
   * @returns {string}
   */
  getNullifier(index?: number | undefined) {
    if (this.index === undefined && index) {
      this.index = index;
    }

    if (
      this.index === undefined &&
      this.amounts[0].eq(new BN(0)) &&
      this.amounts[1].eq(new BN(0))
    ) {
      this.index = 0;
    } else if (this.index === undefined) {
      throw new UtxoError(
        UtxoErrorCode.INDEX_NOT_PROVIDED,
        "getNullifier",
        "The index of an utxo in the merkle tree is required to compute the nullifier hash.",
      );
    }

    if (
      (!this.amounts[0].eq(new BN(0)) || !this.amounts[1].eq(new BN(0))) &&
      this.account.privkey.toString() === "0"
    ) {
      throw new UtxoError(
        UtxoErrorCode.ACCOUNT_HAS_NO_PRIVKEY,
        "getNullifier",
        "The index of an utxo in the merkle tree is required to compute the nullifier hash.",
      );
    }

    if (!this._nullifier) {
      const signature = this.account.privkey
        ? this.account.sign(this.getCommitment(), this.index || 0)
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
   * @description Encrypts the utxo to the utxo's accounts public key with nacl.box.
   *
   * @returns {Uint8Array} with the last 24 bytes being the nonce
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
      this.account.encryptionKeypair.publicKey,
      CONSTANT_SECRET_AUTHKEY,
    );

    return new Uint8Array([...ciphertext, ...nonce]);
  }

  // TODO: add decrypt custom (app utxos with idl)
  /**
   * @description Decrypts a utxo from an array of bytes, the last 24 bytes are the nonce.
   * @param {any} poseidon
   * @param {Uint8Array} encBytes
   * @param {Account} account
   * @param {number} index
   * @returns {Utxo | null}
   */
  static decrypt({
    poseidon,
    encBytes,
    account,
    index,
  }: {
    poseidon: any;
    encBytes: Uint8Array;
    account: Account;
    index: number;
  }): Utxo | null {
    const encryptedUtxo = new Uint8Array(Array.from(encBytes.slice(0, 71)));
    const nonce = new Uint8Array(Array.from(encBytes.slice(71, 71 + 24)));

    if (account.encryptionKeypair.secretKey) {
      const cleartext = box.open(
        encryptedUtxo,
        nonce,
        nacl.box.keyPair.fromSecretKey(CONSTANT_SECRET_AUTHKEY).publicKey,
        account.encryptionKeypair.secretKey,
      );
      if (!cleartext) {
        return null;
      }
      const bytes = Buffer.from(cleartext);
      return Utxo.fromBytes({ poseidon, bytes, account, index });
    } else {
      return null;
    }
  }

  /**
   * @description Compares two Utxos.
   * @param {Utxo} utxo0
   * @param {Utxo} utxo1
   */
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
