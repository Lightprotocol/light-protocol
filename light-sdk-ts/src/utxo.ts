import nacl, { box } from "tweetnacl";

const randomBN = (nbytes = 30) => {
  return new anchor.BN(nacl.randomBytes(nbytes));
};

exports.randomBN = randomBN;
const anchor = require("@coral-xyz/anchor");
import { PublicKey, SystemProgram } from "@solana/web3.js";
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;

import { AccountClient, BN, BorshAccountsCoder, Idl } from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  UtxoError,
  UtxoErrorCode,
  CONSTANT_SECRET_AUTHKEY,
  fetchAssetByIdLookUp,
  getAssetIndex,
  hashAndTruncateToCircuit,
  Account,
  IDL_VERIFIER_PROGRAM_ZERO,
  CreateUtxoErrorCode,
  createAccountObject,
} from "./index";

export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
// TODO: move to constants
export const N_ASSETS = 2;
export const N_ASSET_PUBKEYS = 3;

// TODO: Idl support for U256
// TODO: add static createSolUtxo()
// TODO: remove account as attribute and from constructor, replace with shieldedPublicKey
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
   * @param {BN} appDataHash is the poseidon hash of app utxo data. This compresses the app data and ties it to the app utxo.
   * @param {BN} poolType is the pool type domain of the utxo default is [0;32].
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
  appData: any;
  verifierAddress: PublicKey;
  verifierAddressCircuit: BN;
  appDataHash: BN;
  poolType: BN;
  _commitment?: string;
  _nullifier?: string;
  includeAppData: boolean;
  transactionVersion: string;
  splAssetIndex?: BN;
  appDataIdl?: Idl;

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
   * @param {BN} appDataHash is the poseidon hash of app utxo data. This compresses the app data and ties it to the app utxo.
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
    index,
    appDataHash,
    appData,
    appDataIdl,
    includeAppData = true,
  }: {
    poseidon: any;
    assets?: PublicKey[];
    amounts?: BN[];
    account?: Account;
    blinding?: BN;
    poolType?: BN;
    verifierAddress?: PublicKey;
    index?: number;
    appData?: any;
    appDataIdl?: Idl;
    includeAppData?: boolean;
    appDataHash?: BN;
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

    // TODO: check that this does not lead to hickups since publicAmountSpl cannot withdraw the fee asset sol
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
    if (!account) {
      this.account = new Account({ poseidon });
    } else {
      this.account = account;
    }

    this.blinding = blinding;
    this.index = index;
    this.assets = assets;
    this.appData = appData;
    this.poolType = poolType;
    this.includeAppData = includeAppData;
    this.transactionVersion = "0";

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

    if (appDataHash && appData)
      throw new UtxoError(
        UtxoErrorCode.APP_DATA_DEFINED,
        "constructor",
        "Cannot provide both app data and appDataHash",
      );

    // if appDataBytes parse appData from bytes
    if (appData && !appDataHash) {
      if (!appDataIdl)
        throw new UtxoError(
          UtxoErrorCode.APP_DATA_IDL_UNDEFINED,
          "constructor",
          "",
        );
      if (!appDataIdl.accounts)
        throw new UtxoError(
          UtxoErrorCode.APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS,
          "APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS",
        );
      let i = appDataIdl.accounts.findIndex((acc) => {
        return acc.name === "utxo";
      });
      if (i === -1)
        throw new UtxoError(
          UtxoErrorCode.UTXO_APP_DATA_NOT_FOUND_IN_IDL,
          "constructor",
        );
      let accountClient = new AccountClient(
        appDataIdl,
        appDataIdl.accounts[i],
        SystemProgram.programId,
      );
      // TODO: perform type check that appData has all the attributes and these have the correct types and not more
      let hashArray = [];
      for (var attribute in appData) {
        hashArray.push(appData[attribute]);
      }
      this.appDataHash = new BN(
        leInt2Buff(
          unstringifyBigInts(poseidon.F.toString(poseidon(hashArray))),
          32,
        ),
        undefined,
        "le",
      );
      this.appData = appData;
      this.appDataIdl = appDataIdl;
    } else {
      this.appDataHash = new BN("0");
    }
  }

  /**
   * @description Parses a utxo to bytes.
   * @returns {Uint8Array}
   */
  toBytes() {
    this.splAssetIndex = getAssetIndex(this.assets[1]);

    if (this.splAssetIndex.toString() == "-1") {
      throw new UtxoError(
        UtxoErrorCode.ASSET_NOT_FOUND,
        "toBytes",
        "Asset not found in lookup table",
      );
    }
    if (!this.appDataIdl || !this.includeAppData) {
      let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);

      return coder.encode("utxo", this);
    } else if (this.appDataIdl) {
      let coder = new BorshAccountsCoder(this.appDataIdl);
      let object = {
        ...this,
        blinding: this.blinding,
        ...this.appData,
      };
      return coder.encode("utxo", object);
    } else {
      throw new UtxoError(
        UtxoErrorCode.APP_DATA_IDL_UNDEFINED,
        "constructor",
        "Should include app data but no appDataIdl provided",
      );
    }
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
  static fromBytes({
    poseidon,
    bytes,
    account,
    includeAppData = true,
    index,
    appDataIdl,
  }: {
    poseidon: any;
    bytes: Buffer;
    account?: Account;
    includeAppData?: boolean;
    index: number;
    appDataIdl?: Idl;
  }): Utxo {
    let decodedUtxoData: any;
    let assets: Array<PublicKey>;
    let appData: any = undefined;
    // TODO: should I check whether an account is passed or not?
    if (!appDataIdl) {
      let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);
      decodedUtxoData = coder.decode("utxo", bytes);
    } else {
      if (!appDataIdl.accounts)
        throw new UtxoError(
          UtxoErrorCode.APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS,
          "fromBytes",
        );

      let coder = new BorshAccountsCoder(appDataIdl);
      decodedUtxoData = coder.decode("utxo", bytes);
      appData = createAccountObject(
        decodedUtxoData,
        appDataIdl.accounts,
        "utxoAppData",
      );
    }
    assets = [
      SystemProgram.programId,
      fetchAssetByIdLookUp(decodedUtxoData.splAssetIndex),
    ];

    return new Utxo({
      assets,
      account,
      index,
      poseidon,
      appDataIdl,
      includeAppData,
      ...decodedUtxoData,
      appData,
    });
  }

  /**
   * @description Returns commitment for this utxo
   * @description PoseidonHash(amountHash, shieldedPubkey, blinding, assetHash, appDataHash, poolType, verifierAddressCircuit)
   * @returns {string}
   */
  getCommitment(poseidon: any): string {
    if (!this._commitment) {
      let amountHash = poseidon.F.toString(poseidon(this.amounts));
      let assetHash = poseidon.F.toString(
        poseidon(this.assetsCircuit.map((x) => x.toString())),
      );
      let publicKey: BN;
      if (this.account) {
        publicKey = this.account.pubkey;
      } else {
        throw new UtxoError(
          CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
          "getCommitment",
          "Neither Account nor shieldedPublicKey was provided",
        );
      }
      // console.log("this.assetsCircuit ", this.assetsCircuit);

      // console.log("amountHash ", amountHash.toString());
      // console.log("this.keypair.pubkey ", this.keypair.pubkey.toString());
      // console.log("this.blinding ", this.blinding.toString());
      // console.log("assetHash ", assetHash.toString());
      // console.log("this.appDataHash ", this.appDataHash.toString());
      // console.log("this.poolType ", this.poolType.toString());
      let commitment: string = poseidon.F.toString(
        poseidon([
          this.transactionVersion,
          amountHash,
          publicKey.toString(),
          this.blinding.toString(),
          assetHash.toString(),
          this.appDataHash.toString(),
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
  getNullifier(poseidon: any, index?: number | undefined) {
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
        ? this.account.sign(
            poseidon,
            this.getCommitment(poseidon),
            this.index || 0,
          )
        : 0;
      // console.log("this.getCommitment() ", this.getCommitment());
      // console.log("this.index || 0 ", this.index || 0);
      // console.log("signature ", signature);

      this._nullifier = poseidon.F.toString(
        poseidon([this.getCommitment(poseidon), this.index || 0, signature]),
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
  // TODO: add padding option with 0s to 128 bytes length
  async encrypt(): Promise<Uint8Array> {
    const bytes_message = await this.toBytes();

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
    return Uint8Array.from([...nonce, ...ciphertext]);
  }

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
    const encryptedUtxo = new Uint8Array(Array.from(encBytes.slice(24, 104)));
    const nonce = new Uint8Array(Array.from(encBytes.slice(0, 24)));

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
      return Utxo.fromBytes({
        poseidon,
        bytes,
        account,
        index,
      });
    } else {
      return null;
    }
  }

  /**
   * @description Compares two Utxos.
   * @param {Utxo} utxo0
   * @param {Utxo} utxo1
   */
  static equal(poseidon: any, utxo0: Utxo, utxo1: Utxo) {
    assert.equal(
      utxo0.amounts[0].toString(),
      utxo1.amounts[0].toString(),
      "solAmount",
    );
    assert.equal(
      utxo0.amounts[1].toString(),
      utxo1.amounts[1].toString(),
      "splAmount",
    );
    assert.equal(
      utxo0.assets[0].toBase58(),
      utxo1.assets[0].toBase58(),
      "solAsset",
    );
    assert.equal(utxo0.assets[1].toBase58(), utxo1.assets[1].toBase58()),
      "splAsset";
    assert.equal(
      utxo0.assetsCircuit[0].toString(),
      utxo1.assetsCircuit[0].toString(),
      "solAsset circuit",
    );
    assert.equal(
      utxo0.assetsCircuit[1].toString(),
      utxo1.assetsCircuit[1].toString(),
      "splAsset circuit",
    );
    assert.equal(
      utxo0.appDataHash.toString(),
      utxo1.appDataHash.toString(),
      "appDataHash",
    );
    assert.equal(
      utxo0.poolType.toString(),
      utxo1.poolType.toString(),
      "poolType",
    );
    assert.equal(
      utxo0.verifierAddress.toString(),
      utxo1.verifierAddress.toString(),
      "verifierAddress",
    );
    assert.equal(
      utxo0.verifierAddressCircuit.toString(),
      utxo1.verifierAddressCircuit.toString(),
      "verifierAddressCircuit",
    );
    assert.equal(
      utxo0.getCommitment(poseidon)?.toString(),
      utxo1.getCommitment(poseidon)?.toString(),
      "commitment",
    );

    if (utxo0.index || utxo1.index) {
      assert.equal(
        utxo0.getNullifier(poseidon)?.toString(),
        utxo1.getNullifier(poseidon)?.toString(),
        "nullifier",
      );
    }
  }
}

exports.Utxo = Utxo;
