import nacl, { box, randomBytes } from "tweetnacl";

const randomBN = (nbytes = 30) => {
  return new anchor.BN(nacl.randomBytes(nbytes));
};
const { encrypt, decrypt } = require("ethereum-cryptography/aes");

exports.randomBN = randomBN;
const anchor = require("@coral-xyz/anchor");

import { PublicKey, SystemProgram } from "@solana/web3.js";

var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;

import {
  AccountClient,
  ACCOUNT_DISCRIMINATOR_SIZE,
  BN,
  BorshAccountsCoder,
  Idl,
} from "@coral-xyz/anchor";
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
  COMPRESSED_UTXO_BYTES_LENGTH,
  UNCOMPRESSED_UTXO_BYTES_LENGTH,
  ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
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

    // if appDataBytes parse appData from bytes
    if (appData) {
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
      if (appDataHash && appDataHash.toString() !== this.appDataHash.toString())
        throw new UtxoError(
          UtxoErrorCode.INVALID_APP_DATA,
          "constructor",
          "appDataHash and appData are inconsistent, appData produced a different hash than appDataHash",
        );
      this.appData = appData;
      this.appDataIdl = appDataIdl;
    } else if (appDataHash) {
      this.appDataHash = appDataHash;
    } else {
      this.appDataHash = new BN("0");
    }
  }

  /**
   * @description Parses a utxo to bytes.
   * @returns {Uint8Array}
   */
  async toBytes(compressed: boolean = false) {
    this.splAssetIndex = getAssetIndex(this.assets[1]);

    if (this.splAssetIndex.eq(new BN("-1"))) {
      throw new UtxoError(
        UtxoErrorCode.ASSET_NOT_FOUND,
        "toBytes",
        "Asset not found in lookup table",
      );
    }

    let serializeObject = {
      ...this,
      accountShieldedPublicKey: this.account.pubkey,
      accountEncryptionPublicKey: this.account.encryptionKeypair.publicKey,
      verifierAddressIndex:
        this.verifierAddress.toBase58() === SystemProgram.programId.toBase58()
          ? new BN("0")
          : new BN("1"),
    };
    let serializedData;
    if (!this.appDataIdl || !this.includeAppData) {
      let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);

      serializedData = await coder.encode("utxo", serializeObject);
    } else if (this.appDataIdl) {
      let coder = new BorshAccountsCoder(this.appDataIdl);
      serializeObject = {
        ...serializeObject,
        ...this.appData,
        verifierAddressIndex:
          this.verifierAddress.toBase58() === SystemProgram.programId.toBase58()
            ? new BN("0")
            : new BN("1"),
      };
      serializedData = await coder.encode("utxo", serializeObject);
    } else {
      throw new UtxoError(
        UtxoErrorCode.APP_DATA_IDL_UNDEFINED,
        "constructor",
        "Should include app data but no appDataIdl provided",
      );
    }
    // Compressed serialization does not store the account since for an encrypted utxo
    // we assume that the user who is able to decrypt the utxo knows the corresponding account.
    if (compressed) {
      return serializedData.subarray(0, COMPRESSED_UTXO_BYTES_LENGTH);
    } else {
      return serializedData;
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
  // TODO: take array of idls as input and select the idl with the correct verifierIndex
  static fromBytes({
    poseidon,
    bytes,
    account,
    includeAppData = true,
    index,
    appDataIdl,
    verifierAddress,
  }: {
    poseidon: any;
    bytes: Buffer;
    account?: Account;
    includeAppData?: boolean;
    index?: number;
    appDataIdl?: Idl;
    verifierAddress?: PublicKey;
  }): Utxo {
    // assumes it is compressed and adds 64 0 bytes padding
    if (bytes.length === COMPRESSED_UTXO_BYTES_LENGTH) {
      let tmp: Uint8Array = Uint8Array.from([...Array.from(bytes)]);
      bytes = Buffer.from([
        ...tmp,
        ...new Uint8Array(
          UNCOMPRESSED_UTXO_BYTES_LENGTH - COMPRESSED_UTXO_BYTES_LENGTH,
        ).fill(0),
      ]);
      includeAppData = false;
      if (!account)
        throw new UtxoError(
          CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
          "fromBytes",
          "For deserializing a compressed utxo an account is required.",
        );
    }

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

    // TODO: make lookup function and or tie to idl
    verifierAddress =
      decodedUtxoData.verifierAddressIndex.toString() !== "0"
        ? new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS")
        : SystemProgram.programId;
    if (!account) {
      account = Account.fromPubkey(
        decodedUtxoData.accountShieldedPublicKey.toBuffer(),
        decodedUtxoData.accountEncryptionPublicKey,
        poseidon,
      );
    }

    return new Utxo({
      assets,
      account,
      index,
      poseidon,
      appDataIdl,
      includeAppData,
      appData,
      verifierAddress,
      ...decodedUtxoData,
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
      // console.log("this.keypair.pubkey ", this.account.pubkey.toString());
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
   * @returns {Uint8Array} with the first 24 bytes being the nonce
   */
  async encrypt(
    poseidon: any,
    merkleTreePdaPublicKey?: PublicKey,
    transactionIndex?: number,
  ): Promise<Uint8Array> {
    const bytes_message = await this.toBytes(true);

    var nonce = new BN(this.getCommitment(poseidon))
      .toBuffer("le", 32)
      .subarray(0, 24);

    if (!this.account.aesSecret) {
      // CONSTANT_SECRET_AUTHKEY is used to minimize the number of bytes sent to the blockchain.
      // This results in poly135 being useless since the CONSTANT_SECRET_AUTHKEY is public.
      // However, ciphertext integrity is guaranteed since a hash of the ciphertext is included in a zero-knowledge proof.
      const ciphertext = box(
        bytes_message,
        nonce,
        this.account.encryptionKeypair.publicKey,
        CONSTANT_SECRET_AUTHKEY,
      );

      return Uint8Array.from([
        ...ciphertext,
        ...new Array(128 - ciphertext.length).fill(0),
      ]);
    } else {
      if (!merkleTreePdaPublicKey)
        throw new UtxoError(
          UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED,
          "encrypt",
          "For aes encryption the merkle tree pda publickey is necessary to derive the viewingkey",
        );
      if (transactionIndex === undefined)
        throw new UtxoError(
          UtxoErrorCode.TRANSACTION_INDEX_UNDEFINED,
          "encrypt",
          "For aes encryption the transaction index is necessary to derive the viewingkey",
        );

      const iv16 = nonce.subarray(0, 16);

      const ciphertext = await encrypt(
        bytes_message,
        this.account.getAesUtxoViewingKey(
          merkleTreePdaPublicKey,
          transactionIndex,
        ),
        iv16,
        "aes-256-cbc",
        true,
      );

      // adding the 8 unused nonce bytes as padding at the end to make the ciphertext the same length as nacl box ciphertexts
      return Uint8Array.from([
        ...ciphertext,
        ...new Array(128 - ciphertext.length).fill(0),
      ]);
    }
  }

  /**
   * @description Decrypts a utxo from an array of bytes, the last 24 bytes are the nonce.
   * @param {any} poseidon
   * @param {Uint8Array} encBytes
   * @param {Account} account
   * @param {number} index
   * @returns {Utxo | null}
   */
  static async decrypt({
    poseidon,
    encBytes,
    account,
    index,
    merkleTreePdaPublicKey,
    transactionIndex,
    aes = true,
    commitment,
  }: {
    poseidon: any;
    encBytes: Uint8Array;
    account: Account;
    index: number;
    merkleTreePdaPublicKey?: PublicKey;
    transactionIndex?: number;
    aes?: boolean;
    commitment: Uint8Array;
  }): Promise<Utxo | null> {
    if (aes) {
      if (!account.aesSecret) {
        throw new UtxoError(UtxoErrorCode.AES_SECRET_UNDEFINED, "decrypt");
      }
      if (!merkleTreePdaPublicKey)
        throw new UtxoError(
          UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED,
          "encrypt",
          "For aes decryption the merkle tree pda publickey is necessary to derive the viewingkey",
        );
      if (transactionIndex === undefined)
        throw new UtxoError(
          UtxoErrorCode.TRANSACTION_INDEX_UNDEFINED,
          "encrypt",
          "For aes decryption the transaction index is necessary to derive the viewingkey",
        );
      const encryptedUtxo = encBytes.subarray(
        0,
        ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
      );

      const iv = commitment.subarray(0, 16);
      try {
        const cleartext = await decrypt(
          encryptedUtxo,
          account.getAesUtxoViewingKey(
            merkleTreePdaPublicKey,
            transactionIndex,
          ),
          iv,
          "aes-256-cbc",
          true,
        );

        const bytes = Buffer.from(cleartext);
        return Utxo.fromBytes({
          poseidon,
          bytes,
          account,
          index,
        });
      } catch (_) {
        return null;
      }
    } else {
      const nonce = commitment.subarray(0, 24);
      const encryptedUtxo = encBytes.subarray(
        0,
        NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
      );

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
  }

  /**
   * @description Compares two Utxos.
   * @param {Utxo} utxo0
   * @param {Utxo} utxo1
   */
  static equal(
    poseidon: any,
    utxo0: Utxo,
    utxo1: Utxo,
    skipNullifier: boolean = false,
  ) {
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
    if (!skipNullifier) {
      if (utxo0.index || utxo1.index) {
        if (utxo0.account.privkey || utxo1.account.privkey) {
          assert.equal(
            utxo0.getNullifier(poseidon)?.toString(),
            utxo1.getNullifier(poseidon)?.toString(),
            "nullifier",
          );
        }
      }
    }
  }
}

exports.Utxo = Utxo;
