import nacl, { box } from "tweetnacl";

const randomBN = (nbytes = 30) => {
  return new anchor.BN(nacl.randomBytes(nbytes));
};
import { encrypt, decrypt } from "ethereum-cryptography/aes";
const { sha3_256 } = require("@noble/hashes/sha3");

const anchor = require("@coral-xyz/anchor");

import { PublicKey, SystemProgram } from "@solana/web3.js";

const ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;

import { BN, BorshAccountsCoder, Idl } from "@coral-xyz/anchor";
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
  fetchVerifierByIdLookUp,
  setEnvironment,
  FIELD_SIZE,
  BN_0,
} from "./index";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
export const randomPrefixBytes = () => nacl.randomBytes(PREFIX_LENGTH);

// TODO: move to constants
export const N_ASSETS = 2;
export const N_ASSET_PUBKEYS = 3;
export const PREFIX_LENGTH = 4;

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
  assetsCircuit: BN[] = [];
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
  appDataIdl?: Idl;
  splAssetIndex: BN;
  verifierProgramIndex: BN;

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
    amounts = [BN_0],
    account,
    blinding = new BN(randomBN(), 31, "be"),
    poolType = BN_0,
    verifierAddress = SystemProgram.programId,
    index,
    appDataHash,
    appData,
    appDataIdl,
    includeAppData = true,
    assetLookupTable,
    verifierProgramLookupTable,
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
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
  }) {
    if (!blinding.eq(blinding.mod(FIELD_SIZE))) {
      throw new UtxoError(
        UtxoErrorCode.BLINDING_EXCEEDS_FIELD_SIZE,
        "constructor",
        `Blinding ${blinding}, exceeds field size.`,
      );
    }
    if (assets.length != amounts.length) {
      throw new UtxoError(
        UtxoErrorCode.INVALID_ASSET_OR_AMOUNTS_LENGTH,
        "constructor",

        `Length mismatch assets: ${assets.length} != amounts: ${amounts.length}`,
      );
    }
    if (assets.length > N_ASSETS) {
      throw new UtxoError(
        UtxoErrorCode.EXCEEDED_MAX_ASSETS,
        "constructor",

        `assets.length ${assets.length} > N_ASSETS ${N_ASSETS}`,
      );
    }

    if (assets.findIndex((asset) => !asset) !== -1) {
      throw new UtxoError(
        UtxoErrorCode.ASSET_UNDEFINED,
        "constructor",
        `asset in index ${index} is undefined. All assets: ${assets}`,
      );
    }

    if (assets.findIndex((asset) => !asset) !== -1) {
      throw new UtxoError(
        UtxoErrorCode.ASSET_UNDEFINED,
        "constructor",
        `asset in index ${index} is undefined. All assets: ${assets}`,
      );
    }

    while (assets.length < N_ASSETS) {
      assets.push(SystemProgram.programId);
    }

    let i = 0;
    while (i < N_ASSETS) {
      const amount = amounts[i];
      if (amount?.lt?.(BN_0)) {
        throw new UtxoError(
          UtxoErrorCode.NEGATIVE_AMOUNT,
          "constructor",
          `amount cannot be negative, amounts[${i}] = ${amount ?? "undefined"}`,
        );
      }
      i++;
    }

    while (amounts.length < N_ASSETS) {
      amounts.push(BN_0);
    }

    // TODO: check that this does not lead to hiccups since publicAmountSpl cannot withdraw the fee asset sol
    if (assets[1].toBase58() == SystemProgram.programId.toBase58()) {
      amounts[0] = amounts[0].add(amounts[1]);
      amounts[1] = BN_0;
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

    this.account = account || new Account({ poseidon });
    this.blinding = blinding;
    this.index = index;
    this.assets = assets;
    this.appData = appData;
    this.poolType = poolType;
    this.includeAppData = includeAppData;
    this.transactionVersion = "0";

    if (
      assets[1].toBase58() === SystemProgram.programId.toBase58() &&
      !amounts[1].isZero()
    ) {
      throw new UtxoError(
        UtxoErrorCode.POSITIVE_AMOUNT,
        "constructor",
        `spl amount cannot be positive, amounts[1] = ${
          amounts[1] ?? "undefined"
        }`,
      );
    }
    // TODO: make variable length
    else if (assets[1].toBase58() != SystemProgram.programId.toBase58()) {
      this.assetsCircuit = [
        hashAndTruncateToCircuit(SystemProgram.programId.toBytes()),
        hashAndTruncateToCircuit(this.assets[1].toBytes()),
      ];
    } else if (this.amounts[0].isZero()) {
      this.assetsCircuit = [BN_0, BN_0];
    }
    // else if (!this.amounts[0].isZero()) {
    //   throw new UtxoError(
    //     UtxoErrorCode.NON_ZERO_AMOUNT,
    //     "constructor",
    //     `amount not zero, amounts[0] = ${this.amounts[0] ?? "undefined"}`,
    //   );
    // }
    else {
      this.assetsCircuit = [
        hashAndTruncateToCircuit(SystemProgram.programId.toBytes()),
        BN_0,
      ];
    }

    if (verifierAddress.toBase58() == SystemProgram.programId.toBase58()) {
      this.verifierAddress = verifierAddress;
      this.verifierAddressCircuit = BN_0;
      this.verifierProgramIndex = BN_0;
    } else {
      this.verifierAddress = verifierAddress;
      this.verifierAddressCircuit = hashAndTruncateToCircuit(
        verifierAddress.toBytes(),
      );

      this.verifierProgramIndex = new BN(
        verifierProgramLookupTable.findIndex((verifierAddress: string) => {
          return verifierAddress === this.verifierAddress.toBase58();
        }),
      );
      if (this.verifierProgramIndex.isNeg())
        throw new UtxoError(
          UtxoErrorCode.VERIFIER_INDEX_NOT_FOUND,
          "constructor",
          `verifier pubkey ${this.verifierAddress}, not found in lookup table`,
        );
    }
    this.splAssetIndex = getAssetIndex(this.assets[1], assetLookupTable);
    if (this.splAssetIndex.isNeg())
      throw new UtxoError(
        UtxoErrorCode.ASSET_NOT_FOUND,
        "constructor",
        `asset pubkey ${this.assets[1]}, not found in lookup table`,
      );
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
      // TODO: add inputs type check
      // TODO: unify with Prover.ts
      // perform type check that appData has all the attributes
      const checkAppData = (appData: any, idl: any) => {
        const circuitName = "utxoAppData";
        const circuitIdlObject = idl.accounts!.find(
          (account: any) => account.name === circuitName,
        );

        if (!circuitIdlObject) {
          throw new Error(`${circuitName} does not exist in anchor idl`);
        }

        const fieldNames = circuitIdlObject.type.fields.map(
          (field: { name: string }) => field.name,
        );
        const inputKeys: string[] = [];

        fieldNames.forEach((fieldName: string) => {
          inputKeys.push(fieldName);
        });

        let inputsObject: { [key: string]: any } = {};

        inputKeys.forEach((key) => {
          inputsObject[key] = appData[key];
          if (!inputsObject[key])
            throw new Error(
              `Missing input --> ${key.toString()} in circuit ==> ${circuitName}`,
            );
        });
      };
      checkAppData(appData, appDataIdl);
      let hashArray: any[] = [];
      for (const attribute in appData) {
        hashArray.push(appData[attribute]);
      }
      hashArray = hashArray.flat();
      if (hashArray.length > 16) {
        throw new UtxoError(
          UtxoErrorCode.INVALID_APP_DATA,
          "constructor",
          "appData length exceeds 16",
        );
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
      this.appDataHash = BN_0;
    }
  }

  /**
   * @description Parses a utxo to bytes.
   * @returns {Uint8Array}
   */
  async toBytes(compressed: boolean = false) {
    let serializeObject = {
      ...this,
      accountShieldedPublicKey: this.account.pubkey,
      accountEncryptionPublicKey: this.account.encryptionKeypair.publicKey,
      verifierAddressIndex: this.verifierProgramIndex,
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
        verifierAddressIndex: this.verifierProgramIndex,
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
    return compressed
      ? serializedData.subarray(0, COMPRESSED_UTXO_BYTES_LENGTH)
      : serializedData;
  }

  /**
   * @description Parses an utxo from bytes.
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
    assetLookupTable,
    verifierProgramLookupTable,
  }: {
    poseidon: any;
    bytes: Buffer;
    account?: Account;
    includeAppData?: boolean;
    index?: number;
    appDataIdl?: Idl;
    verifierAddress?: PublicKey;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
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
      fetchAssetByIdLookUp(decodedUtxoData.splAssetIndex, assetLookupTable),
    ];

    verifierAddress = fetchVerifierByIdLookUp(
      decodedUtxoData.verifierAddressIndex,
      verifierProgramLookupTable,
    );
    if (!account) {
      let concatPublicKey = bs58.encode(
        new Uint8Array([
          ...decodedUtxoData.accountShieldedPublicKey.toArray("be", 32),
          ...decodedUtxoData.accountEncryptionPublicKey,
        ]),
      );
      account = Account.fromPubkey(concatPublicKey, poseidon);
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
      verifierProgramLookupTable,
      assetLookupTable,
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
   * @param poseidon
   * @param {number} index Merkle tree index of the utxo commitment (Optional)
   *
   * @returns {string}
   */
  getNullifier(poseidon: any, index?: number | undefined) {
    if (this.index === undefined) {
      if (index) {
        this.index = index;
      } else if (this.amounts[0].isZero() && this.amounts[1].isZero()) {
        this.index = 0;
      } else {
        throw new UtxoError(
          UtxoErrorCode.INDEX_NOT_PROVIDED,
          "getNullifier",
          "The index of a UTXO in the Merkle tree is required to compute the nullifier hash.",
        );
      }
    }

    if (
      (!this.amounts[0].eq(BN_0) || !this.amounts[1].eq(BN_0)) &&
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
    compressed: boolean = true,
  ): Promise<Uint8Array> {
    const bytes_message = await this.toBytes(compressed);
    const commitment = new BN(this.getCommitment(poseidon)).toBuffer("le", 32);
    const nonce = commitment.subarray(0, 24);

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

      let prefix = randomPrefixBytes();
      return Uint8Array.from([...prefix, ...ciphertext]);
    } else {
      if (!merkleTreePdaPublicKey)
        throw new UtxoError(
          UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED,
          "encrypt",
          "For aes encryption the merkle tree pda publickey is necessary to derive the viewingkey",
        );

      setEnvironment();
      const iv16 = nonce.subarray(0, 16);
      const ciphertext = await encrypt(
        bytes_message,
        this.account.getAesUtxoViewingKey(
          merkleTreePdaPublicKey,
          bs58.encode(commitment),
        ),
        iv16,
        "aes-256-cbc",
        true,
      );

      let prefix = this.account.generateUtxoPrefixHash(
        commitment,
        PREFIX_LENGTH,
      );
      if (!compressed) return Uint8Array.from([...prefix, ...ciphertext]);
      const padding = sha3_256
        .create()
        .update(Uint8Array.from([...nonce, ...bytes_message]))
        .digest();

      // adding the 8 bytes as padding at the end to make the ciphertext the same length as nacl box ciphertexts of (120 + PREFIX_LENGTH) bytes
      return Uint8Array.from([
        ...prefix,
        ...ciphertext,
        ...padding.subarray(0, 8),
      ]);
    }
  }

  /**
   * @description Checks the UTXO prefix hash of PREFIX_LENGTH-bytes
   *
   * @returns {boolean} true || false
   */
  static checkPrefixHash({
    account,
    commitment,
    prefixBytes,
  }: {
    account: Account;
    commitment: Uint8Array;
    prefixBytes: Uint8Array;
  }): boolean {
    let p1 = account
      .generateUtxoPrefixHash(commitment, PREFIX_LENGTH)
      .join(",");
    let p2 = prefixBytes.join(",");
    return p1 === p2;
  }

  // TODO: unify compressed and includeAppData into a parsingConfig or just keep one
  /**
   * @description Decrypts an utxo from an array of bytes without checking the UTXO prefix hash, the last 24 bytes are the nonce.
   * @param {any} poseidon
   * @param {Uint8Array} encBytes
   * @param {Account} account
   * @param {number} index
   * @returns {Utxo | null}
   */
  static async decryptUnchecked({
    poseidon,
    encBytes,
    account,
    index,
    merkleTreePdaPublicKey,
    aes,
    commitment,
    appDataIdl,
    compressed = true,
    assetLookupTable,
    verifierProgramLookupTable,
  }: {
    poseidon: any;
    encBytes: Uint8Array;
    account: Account;
    index: number;
    merkleTreePdaPublicKey?: PublicKey;
    aes: boolean;
    commitment: Uint8Array;
    appDataIdl?: Idl;
    compressed?: boolean;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
  }): Promise<Utxo | null> {
    encBytes = encBytes.slice(PREFIX_LENGTH);
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
      if (compressed) {
        encBytes = encBytes.slice(0, ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH);
      }
      setEnvironment();
      const iv16 = commitment.slice(0, 16);

      try {
        const cleartext = await decrypt(
          encBytes,
          account.getAesUtxoViewingKey(
            merkleTreePdaPublicKey,
            bs58.encode(commitment),
          ),
          iv16,
          "aes-256-cbc",
          true,
        );

        const bytes = Buffer.from(cleartext);

        return Utxo.fromBytes({
          poseidon,
          bytes,
          account,
          index,
          appDataIdl,
          assetLookupTable,
          verifierProgramLookupTable,
        });
      } catch (e) {
        // TODO: return errors - omitted for now because of different error messages on different systems
        return null;
      }
    } else {
      const nonce = commitment.slice(0, 24);
      if (compressed) {
        encBytes = encBytes.slice(
          0,
          NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
        );
      }

      if (account.encryptionKeypair.secretKey) {
        const cleartext = box.open(
          encBytes,
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
          appDataIdl,
          assetLookupTable,
          verifierProgramLookupTable,
        });
      } else {
        return null;
      }
    }
  }

  // TODO: unify compressed and includeAppData into a parsingConfig or just keep one
  /**
   * @description Decrypts an utxo from an array of bytes by checking the UTXO prefix hash, the last 24 bytes are the nonce.
   * @param {any} poseidon
   * @param {Uint8Array} encBytes
   * @param {Account} account
   * @param {number} index
   * @returns {Utxo | boolean }
   */
  static async decrypt({
    poseidon,
    encBytes,
    account,
    index,
    merkleTreePdaPublicKey,
    aes,
    commitment,
    appDataIdl,
    compressed = true,
    assetLookupTable,
    verifierProgramLookupTable,
  }: {
    poseidon: any;
    encBytes: Uint8Array;
    account: Account;
    index: number;
    merkleTreePdaPublicKey?: PublicKey;
    aes: boolean;
    commitment: Uint8Array;
    appDataIdl?: Idl;
    compressed?: boolean;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
  }): Promise<Utxo | boolean> {
    const prefixBytes = encBytes.slice(0, PREFIX_LENGTH);
    if (aes && this.checkPrefixHash({ account, commitment, prefixBytes })) {
      const utxo = await Utxo.decryptUnchecked({
        poseidon,
        encBytes,
        account,
        index,
        merkleTreePdaPublicKey,
        aes,
        commitment,
        appDataIdl,
        compressed,
        assetLookupTable,
        verifierProgramLookupTable,
      });
      if (!utxo)
        return true; // prefixHash matches but decryption fails so utxo is null -> Returns TRUE (COLLISION)
      else return utxo; // prefixHash matches and decryption succeeds so utxo is good -> Returns UTXO (VALID)
    } else return false; // prefixHash doesn't match -> Returns FALSE (NO COLLISION)
  }

  /**
   * Creates a new Utxo from a given base58 encoded string.
   * @static
   * @param {string} string - The base58 encoded string representation of the Utxo.
   * @returns {Utxo} The newly created Utxo.
   */
  static fromString(
    string: string,
    poseidon: any,
    assetLookupTable: string[],
    verifierProgramLookupTable: string[],
  ): Utxo {
    return Utxo.fromBytes({
      bytes: bs58.decode(string),
      poseidon,
      assetLookupTable,
      verifierProgramLookupTable,
    });
  }

  /**
   * Converts the Utxo instance into a base58 encoded string.
   * @async
   * @returns {Promise<string>} A promise that resolves to the base58 encoded string representation of the Utxo.
   */
  async toString(): Promise<string> {
    const bytes = await this.toBytes();
    return bs58.encode(bytes);
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
    if (utxo0.amounts[0].toString() !== utxo1.amounts[0].toString()) {
      throw new Error(
        `solAmount not equal: ${utxo0.amounts[0].toString()} vs ${utxo1.amounts[0].toString()}`,
      );
    }

    if (utxo0.amounts[1].toString() !== utxo1.amounts[1].toString()) {
      throw new Error(
        `splAmount not equal: ${utxo0.amounts[1].toString()} vs ${utxo1.amounts[1].toString()}`,
      );
    }

    if (utxo0.assets[0].toBase58() !== utxo1.assets[0].toBase58()) {
      throw new Error(
        `solAsset not equal: ${utxo0.assets[0].toBase58()} vs ${utxo1.assets[0].toBase58()}`,
      );
    }

    if (utxo0.assets[1].toBase58() !== utxo1.assets[1].toBase58()) {
      throw new Error(
        `splAsset not equal: ${utxo0.assets[1].toBase58()} vs ${utxo1.assets[1].toBase58()}`,
      );
    }

    if (
      utxo0.assetsCircuit[0].toString() !== utxo1.assetsCircuit[0].toString()
    ) {
      throw new Error(
        `solAsset circuit not equal: ${utxo0.assetsCircuit[0].toString()} vs ${utxo1.assetsCircuit[0].toString()}`,
      );
    }

    if (
      utxo0.assetsCircuit[1].toString() !== utxo1.assetsCircuit[1].toString()
    ) {
      throw new Error(
        `splAsset circuit not equal: ${utxo0.assetsCircuit[1].toString()} vs ${utxo1.assetsCircuit[1].toString()}`,
      );
    }

    if (utxo0.appDataHash.toString() !== utxo1.appDataHash.toString()) {
      throw new Error(
        `appDataHash not equal: ${utxo0.appDataHash.toString()} vs ${utxo1.appDataHash.toString()}`,
      );
    }

    if (utxo0.poolType.toString() !== utxo1.poolType.toString()) {
      throw new Error(
        `poolType not equal: ${utxo0.poolType.toString()} vs ${utxo1.poolType.toString()}`,
      );
    }

    if (utxo0.verifierAddress.toString() !== utxo1.verifierAddress.toString()) {
      throw new Error(
        `verifierAddress not equal: ${utxo0.verifierAddress.toString()} vs ${utxo1.verifierAddress.toString()}`,
      );
    }

    if (
      utxo0.verifierAddressCircuit.toString() !==
      utxo1.verifierAddressCircuit.toString()
    ) {
      throw new Error(
        `verifierAddressCircuit not equal: ${utxo0.verifierAddressCircuit.toString()} vs ${utxo1.verifierAddressCircuit.toString()}`,
      );
    }

    if (
      utxo0.getCommitment(poseidon)?.toString() !==
      utxo1.getCommitment(poseidon)?.toString()
    ) {
      throw new Error(
        `commitment not equal: ${utxo0
          .getCommitment(poseidon)
          ?.toString()} vs ${utxo1.getCommitment(poseidon)?.toString()}`,
      );
    }

    if (!skipNullifier) {
      if (utxo0.index || utxo1.index) {
        if (utxo0.account.privkey || utxo1.account.privkey) {
          if (
            utxo0.getNullifier(poseidon)?.toString() !==
            utxo1.getNullifier(poseidon)?.toString()
          ) {
            throw new Error(
              `nullifier not equal: ${utxo0
                .getNullifier(poseidon)
                ?.toString()} vs ${utxo1.getNullifier(poseidon)?.toString()}`,
            );
          }
        }
      }
    }
  }

  static getAppInUtxoIndices(appUtxos: Utxo[]) {
    let isAppInUtxo = [];
    for (const i in appUtxos) {
      let array = new Array(4).fill(new BN(0));
      if (appUtxos[i].appData) {
        array[i] = new BN(1);
        isAppInUtxo.push(array);
      }
    }
    return isAppInUtxo;
  }
}
