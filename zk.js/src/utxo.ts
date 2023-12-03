import nacl from "tweetnacl";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN, BorshAccountsCoder, Idl } from "@coral-xyz/anchor";
import {
  Account,
  BN_0,
  COMPRESSED_UTXO_BYTES_LENGTH,
  createAccountObject,
  CreateUtxoErrorCode,
  ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  fetchAssetByIdLookUp,
  FIELD_SIZE,
  getAssetIndex,
  hashAndTruncateToCircuit,
  IDL_LIGHT_PSP2IN2OUT,
  N_ASSETS,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  UTXO_PREFIX_LENGTH,
  UNCOMPRESSED_UTXO_BYTES_LENGTH,
  UtxoError,
  UtxoErrorCode,
  TransactionParameters,
  BN_1,
} from "./index";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "./types/result";

const randomBN = (nbytes = 30) => {
  return new anchor.BN(nacl.randomBytes(nbytes));
};
const { sha3_256 } = require("@noble/hashes/sha3");

const anchor = require("@coral-xyz/anchor");

const ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;

export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
export const randomPrefixBytes = () => nacl.randomBytes(UTXO_PREFIX_LENGTH);

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
  publicKey: BN;
  encryptionPublicKey?: Uint8Array;
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
  isFillingUtxo: boolean;

  /**
   * @description Initialize a new utxo - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
   *
   * @param {BN[]} amounts array of utxo amounts, amounts[0] is the sol amount amounts[1] is the spl amount
   * @param {PublicKey[]} assets  array of utxo assets, assets[0] is the sol asset assets[1] is the spl asset
   * @param {BN} blinding Blinding factor, a 31 bytes big number, to add randomness to the commitment hash.
   * @param {BN} publicKey the shielded public key owning the utxo.
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
    publicKey,
    blinding = new BN(randomBN(), 31, "be"),
    poolType = BN_0,
    verifierAddress = SystemProgram.programId,
    index,
    appDataHash,
    appData,
    appDataIdl,
    includeAppData = true,
    assetLookupTable,
    isFillingUtxo = false,
    encryptionPublicKey,
  }: {
    poseidon: any;
    assets?: PublicKey[];
    amounts?: BN[];
    publicKey: BN;
    blinding?: BN;
    poolType?: BN;
    verifierAddress?: PublicKey;
    index?: number;
    appData?: any;
    appDataIdl?: Idl;
    includeAppData?: boolean;
    appDataHash?: BN;
    assetLookupTable: string[];
    isFillingUtxo?: boolean;
    encryptionPublicKey?: Uint8Array;
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

    // TODO: check that this does not lead to hiccups since publicAmountSpl
    // cannot unshield the fee asset sol
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

    this.encryptionPublicKey = encryptionPublicKey;
    this.isFillingUtxo = isFillingUtxo;
    this.publicKey = publicKey;
    this.blinding = blinding;
    this.index = index;
    this.assets = assets;
    this.appData = appData;
    this.verifierAddress = verifierAddress;
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
      this.verifierAddressCircuit = BN_0;
      this.verifierProgramIndex = BN_0;
    } else {
      this.verifierAddressCircuit = hashAndTruncateToCircuit(
        verifierAddress.toBytes(),
      );

      // NOTE(vadorovsky): Currently we don't use the `verifierProgramIndex`,
      // but we might revisit it and implement a registry for verifiers,
      // where each system verifier and registered PSP would have an unique
      // ID in the whole protocol. It's not certain though.
      //
      // For now, assign 0 to UTXOs coming from system verifier zero and
      // 1 to UTXOs coming from PSPs through other verifiers.
      if (appDataIdl) {
        this.verifierProgramIndex = BN_1;
      } else {
        this.verifierProgramIndex = BN_0;
      }
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
      const i = appDataIdl.accounts.findIndex((acc) => {
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

        const inputsObject: { [key: string]: any } = {};

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
          `appDataHash and appData are inconsistent, appData produced a different hash than appDataHash appData: ${JSON.stringify(
            appData,
          )}`,
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
      accountShieldedPublicKey: this.publicKey,
      accountEncryptionPublicKey: this.encryptionPublicKey
        ? this.encryptionPublicKey
        : new Uint8Array(32).fill(0),
      verifierAddressIndex: this.verifierProgramIndex,
    };
    let serializedData;
    if (!this.appDataIdl || !this.includeAppData) {
      const coder = new BorshAccountsCoder(IDL_LIGHT_PSP2IN2OUT);
      serializedData = await coder.encode("utxo", serializeObject);
    } else if (this.appDataIdl) {
      const coder = new BorshAccountsCoder(this.appDataIdl);
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
    assetLookupTable,
  }: {
    poseidon: any;
    bytes: Buffer;
    account?: Account;
    includeAppData?: boolean;
    index?: number;
    appDataIdl?: Idl;
    assetLookupTable: string[];
  }): Utxo {
    // assumes it is compressed and adds 64 0 bytes padding
    if (bytes.length === COMPRESSED_UTXO_BYTES_LENGTH) {
      const tmp: Uint8Array = Uint8Array.from([...Array.from(bytes)]);
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
    let appData: any = undefined;
    let verifierAddress: PublicKey;
    // TODO: should I check whether an account is passed or not?
    if (!appDataIdl) {
      const coder = new BorshAccountsCoder(IDL_LIGHT_PSP2IN2OUT);
      decodedUtxoData = coder.decode("utxo", bytes);
      verifierAddress = SystemProgram.programId;
    } else {
      if (!appDataIdl.accounts)
        throw new UtxoError(
          UtxoErrorCode.APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS,
          "fromBytes",
        );

      const coder = new BorshAccountsCoder(appDataIdl);
      decodedUtxoData = coder.decode("utxo", bytes);
      appData = createAccountObject(
        decodedUtxoData,
        appDataIdl.accounts,
        "utxoAppData",
      );

      verifierAddress = TransactionParameters.getVerifierProgramId(appDataIdl);
    }
    const assets = [
      SystemProgram.programId,
      fetchAssetByIdLookUp(decodedUtxoData.splAssetIndex, assetLookupTable),
    ];

    return new Utxo({
      assets,
      publicKey: !account
        ? decodedUtxoData.accountShieldedPublicKey
        : account.pubkey,
      encryptionPublicKey: new BN(
        decodedUtxoData.accountEncryptionPublicKey,
      ).eq(BN_0)
        ? undefined
        : new Uint8Array(decodedUtxoData.accountEncryptionPublicKey),
      index,
      poseidon,
      appDataIdl,
      includeAppData,
      appData,
      verifierAddress,
      ...decodedUtxoData,
      assetLookupTable,
    });
  }

  /**
   * @description Returns commitment for this utxo
   * @description PoseidonHash(amountHash, shieldedPubkey, blinding, assetHash, appDataHash, poolType, verifierAddressCircuit)
   * @returns {string}
   */
  getCommitment(poseidon: any): string {
    const amountHash = poseidon.F.toString(poseidon(this.amounts));
    const assetHash = poseidon.F.toString(
      poseidon(this.assetsCircuit.map((x) => x.toString())),
    );

    if (!this.publicKey) {
      throw new UtxoError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "getCommitment",
        "Neither Account nor shieldedPublicKey was provided",
      );
    }
    // console.log("this.assetsCircuit ", this.assetsCircuit);

    // console.log("amountHash ", amountHash.toString());
    // console.log("this.keypair.pubkey ", this.publicKey.toString());
    // console.log("this.blinding ", this.blinding.toString());
    // console.log("assetHash ", assetHash.toString());
    // console.log("this.appDataHash ", this.appDataHash.toString());
    // console.log("this.poolType ", this.poolType.toString());
    const commitment: string = poseidon.F.toString(
      poseidon([
        this.transactionVersion,
        amountHash,
        this.publicKey.toString(),
        this.blinding.toString(),
        assetHash.toString(),
        this.appDataHash.toString(),
        this.poolType,
        this.verifierAddressCircuit,
      ]),
    );
    this._commitment = commitment;
    return this._commitment;
  }

  /**
   * @description Computes the nullifier for this utxo.
   * @description PoseidonHash(commitment, index, signature)
   * @param poseidon
   * @param account
   * @param {number} index Merkle tree index of the utxo commitment (Optional)
   *
   * @returns {string}
   */
  getNullifier({
    poseidon,
    account,
    index,
  }: {
    poseidon: any;
    account: Account;
    index?: number | undefined;
  }): string {
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
    if (!account)
      throw new UtxoError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "getNullifier",
        "Account is required to compute the nullifier hash.",
      );

    const signature = account.sign(
      poseidon,
      this.getCommitment(poseidon),
      this.index || 0,
    );
    this._nullifier = poseidon.F.toString(
      poseidon([this.getCommitment(poseidon), this.index || 0, signature]),
    );

    return this._nullifier!;
  }

  // TODO: evaluate whether to add a flag to encrypt asymetrically
  /**
   * @description Encrypts the utxo to the utxo's accounts public key with nacl.box.
   *
   * @returns {Uint8Array} with the first 24 bytes being the nonce
   */
  async encrypt({
    poseidon,
    account,
    merkleTreePdaPublicKey,
    compressed = true,
  }: {
    poseidon: any;
    account?: Account;
    merkleTreePdaPublicKey?: PublicKey;
    compressed?: boolean;
  }): Promise<Uint8Array> {
    const bytes_message = await this.toBytes(compressed);
    const commitment = new BN(this.getCommitment(poseidon)).toArrayLike(
      Buffer,
      "le",
      32,
    );

    if (this.encryptionPublicKey) {
      const ciphertext = Account.encryptNaclUtxo(
        this.encryptionPublicKey,
        bytes_message,
        commitment,
      );

      const prefix = randomPrefixBytes();
      return Uint8Array.from([...prefix, ...ciphertext]);
    } else if (account) {
      if (!merkleTreePdaPublicKey)
        throw new UtxoError(
          UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED,
          "encrypt",
          "For aes encryption the merkle tree pda publickey is necessary to derive the viewingkey",
        );

      const ciphertext = await account.encryptAesUtxo(
        bytes_message,
        merkleTreePdaPublicKey,
        commitment,
      );

      // If utxo is filling utxo we don't want to decrypt it in the future, so we use a random prefix
      // we still want to encrypt it properly to be able to decrypt it if necessary as a safeguard.
      const prefix = !this.isFillingUtxo
        ? account.generateUtxoPrefixHash(commitment, UTXO_PREFIX_LENGTH)
        : randomPrefixBytes();
      if (!compressed) return Uint8Array.from([...prefix, ...ciphertext]);
      const padding = sha3_256
        .create()
        .update(Uint8Array.from([...commitment, ...bytes_message]))
        .digest();

      // adding the 8 bytes as padding at the end to make the ciphertext the same length as nacl box ciphertexts of (120 + PREFIX_LENGTH) bytes
      return Uint8Array.from([
        ...prefix,
        ...ciphertext,
        ...padding.subarray(0, 8),
      ]);
    } else {
      throw new UtxoError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "encrypt",
        "Neither account nor this.encryptionPublicKey is defined",
      );
    }
  }

  /**
   * @description Checks the UTXO prefix hash of UTXO_PREFIX_LENGTH-bytes
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
    const generatedPrefixHash = account.generateUtxoPrefixHash(
      commitment,
      UTXO_PREFIX_LENGTH,
    );
    return (
      generatedPrefixHash.length === prefixBytes.length &&
      generatedPrefixHash.every(
        (val: number, idx: number) => val === prefixBytes[idx],
      )
    );
  }

  // TODO: add method decryptWithViewingKey(viewingkey, bytes, commithash, aes) (issue is right now it's difficult to give a viewing key to another party and for this party to decrypt)
  /**
   * @description Decrypts an utxo from an array of bytes without checking the UTXO prefix hash.
   * The prefix hash is assumed exist and to be the first 4 bytes.
   * Thus, the first 4 bytes are ignored. The first by 16 / 24 bytes of the commitment are the IV / nonce.
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
  }: {
    poseidon: any;
    encBytes: Uint8Array;
    account: Account;
    index: number;
    merkleTreePdaPublicKey: PublicKey;
    aes: boolean;
    commitment: Uint8Array;
    appDataIdl?: Idl;
    compressed?: boolean;
    assetLookupTable: string[];
  }): Promise<Result<Utxo | null, UtxoError>> {
    // Remove UTXO prefix with length of UTXO_PREFIX_LENGTH from the encrypted bytes
    encBytes = encBytes.slice(UTXO_PREFIX_LENGTH);

    if (aes && !merkleTreePdaPublicKey) {
      throw new UtxoError(
        UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED,
        "encrypt",
        "Merkle tree pda public key is necessary for AES decryption",
      );
    }

    if (compressed) {
      const length = aes
        ? ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH
        : NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH;
      encBytes = encBytes.slice(0, length);
    }
    const cleartext = aes
      ? await account.decryptAesUtxo(
          encBytes,
          merkleTreePdaPublicKey,
          commitment,
        )
      : await account.decryptNaclUtxo(encBytes, commitment);

    if (!cleartext || cleartext.error || !cleartext.value)
      return Result.Ok(null);
    const bytes = Buffer.from(cleartext.value || cleartext);

    return Result.Ok(
      Utxo.fromBytes({
        poseidon,
        bytes,
        account,
        index,
        appDataIdl,
        assetLookupTable,
      }),
    );
  }

  // TODO: unify compressed and includeAppData into a parsingConfig or just keep one
  /**
   * * @description Decrypts an utxo from an array of bytes by checking the UTXO prefix hash,
   * * the first by 16 / 24 bytes of the commitment are the IV / nonce.
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
  }: {
    poseidon: any;
    encBytes: Uint8Array;
    account: Account;
    index: number;
    merkleTreePdaPublicKey: PublicKey;
    aes: boolean;
    commitment: Uint8Array;
    appDataIdl?: Idl;
    compressed?: boolean;
    assetLookupTable: string[];
  }): Promise<Result<Utxo | null, UtxoError>> {
    // Get UTXO prefix with length of UTXO_PREFIX_LENGTH from the encrypted bytes
    const prefixBytes = encBytes.slice(0, UTXO_PREFIX_LENGTH);

    // If AES is enabled and the prefix of the commitment matches the account and prefixBytes,
    // try to decrypt the UTXO
    if (aes && this.checkPrefixHash({ account, commitment, prefixBytes })) {
      const utxoResult = await Utxo.decryptUnchecked({
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
      });

      // If the return value of decryptUnchecked operation is valid
      if (utxoResult.value) {
        // Return the successfully decrypted UTXO
        return utxoResult;
      }
      // If decryption was unsuccessful, return an error message indicating a prefix collision
      return Result.Err(
        new UtxoError(
          UtxoErrorCode.PREFIX_COLLISION,
          "constructor",
          "Prefix collision when decrypting utxo. " + utxoResult.error ?? "",
        ),
      );
    }
    // If AES isn't enabled or the checkPrefixHash condition fails,
    // return a successful Result with `null`
    return Result.Ok(null);
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
  ): Utxo {
    return Utxo.fromBytes({
      bytes: bs58.decode(string),
      poseidon,
      assetLookupTable,
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
    account0?: Account,
    account1?: Account,
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
        if (account0 && account1) {
          if (
            utxo0.getNullifier({ poseidon, account: account0 })?.toString() !==
            utxo1.getNullifier({ poseidon, account: account1 })?.toString()
          ) {
            throw new Error(
              `nullifier not equal: ${utxo0
                .getNullifier(poseidon)
                ?.toString()} vs ${utxo1.getNullifier(poseidon)?.toString()}`,
            );
          }
          throw new Error("Account0 or Account1 not defined");
        }
      }
    }
  }

  static getAppInUtxoIndices(appUtxos: Utxo[]) {
    const isAppInUtxo: BN[][] = [];
    for (const i in appUtxos) {
      const array = new Array(4).fill(new BN(0));
      if (appUtxos[i].appData) {
        array[i] = new BN(1);
        isAppInUtxo.push(array);
      }
    }
    return isAppInUtxo;
  }
}
