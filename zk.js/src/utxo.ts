import nacl from "tweetnacl";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { LightWasm } from "@lightprotocol/account.rs";
import {
  Account,
  BN_0,
  COMPRESSED_UTXO_BYTES_LENGTH,
  createUtxoDataHash,
  CreateUtxoErrorCode,
  ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  fetchAssetByIdLookUp,
  hashAndTruncateToCircuit,
  IDL_LIGHT_PSP2IN2OUT,
  MERKLE_TREE_HEIGHT,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  UNCOMPRESSED_UTXO_BYTES_LENGTH,
  UTXO_PREFIX_LENGTH,
  UtxoError,
  UtxoErrorCode,
} from "./index";
import { Result } from "./types";

const randomBN = (nbytes = 30) => new BN(nacl.randomBytes(nbytes));
const randomPrefixBytes = () => nacl.randomBytes(UTXO_PREFIX_LENGTH);

/** Public key of Poseidon-hashed keypair */
type CompressionPublicKey = string;

/** Describes the generic utxo details applicable to every utxo. */
export type BaseUtxo = {
  /** Identifier and commitment to the utxo, is inserted as leaf into state tree */
  hash: string;
  /** Hash that invalidates utxo once inserted into nullifier queue, if isPublic = true it defaults to: 'hash' */
  nullifier: string;
  /** Compression public key of the user or public key program owning the utxo */
  owner: CompressionPublicKey | PublicKey;
  /** Optional number of lamports and SPL amount assigned to the utxo */
  amounts: BN[];
  /** Optional native mint and SPL mint address respective to 'amounts' */
  assets: PublicKey[];
  /** Random value to force uniqueness of 'hash' */
  blinding: BN;
  /** Optional type of utxo for custom circuits, defaults to 'native' */
  type: string;
  /** Default '0' */
  version: string;
  /** Indicator for whether the utxo is empty, for readability */
  isFillingUtxo: boolean;
  /** Default 'true'. Whether the inputs to 'hash' are public or not. Useful for confidential compute. */
  isPublic: boolean;
  /** Optional persistent id of the utxo. Used for compressed PDAs and non-fungible tokens */
  address?: PublicKey;
  /** Optional public key of program that owns the metadata */
  metadataOwner?: PublicKey;
  /**
   *	metadata which is immutable in normal transactions.
   *	metadata can be updated by the metadataOwner with a dedicated system psp.
   */
  metadata?: any;
  /** hash of metadata */
  metadataHash?: string;
  /** hash of metadataHash and metadataOwner */
  metaHash?: string;
};

/** Utxo that had previously been inserted into a state Merkle tree */
export type Utxo = Omit<BaseUtxo, "owner"> & {
  /** Compression public key of the user that owns the utxo */
  owner: CompressionPublicKey;
  /** Numerical identifier of the Merkle tree which the 'hash' is part of */
  merkletreeId: BN;
  /** Proof path attached to the utxo. Can be reconstructed using event history */
  merkleProof: string[];
  /** Index of 'hash' as inserted into the Merkle tree */
  merkleTreeLeafIndex: number;
};

/** Utxo that is not inserted into the state tree yet. */
export type OutUtxo = Omit<BaseUtxo, "owner"> & {
  /** Compression public key of the user that owns the utxo */
  owner: CompressionPublicKey;
  /**
   * Optional public key of the ouput utxo owner once inserted into the state tree.
   * Only set if the utxo should be encrypted asymetrically.
   */
  encryptionPublicKey?: Uint8Array;
};

export type NullifierInputs = {
  signature: string;
  /** hash of the utxo preimage */
  hash: string;
  merkleTreeLeafIndex: string;
};

export type CreateUtxoInputs = {
  hash: string;
  amounts: BN[];
  assets: PublicKey[];
  blinding: string;
  merkleProof: string[];
  merkleTreeLeafIndex?: number;
  type?: string;
};

type UtxoHashInputs = {
  /** Compression public key of owner in base58 */
  owner: string;
  amounts: string[];
  assetsCircuit: string[];
  blinding: string;
  poolType: string;
  version: string;
  utxoDataHash: string;
  verifierAddressCircuit: string;
};

export function createFillingOutUtxo({
  lightWasm,
  owner,
}: {
  lightWasm: LightWasm;
  owner: BN;
}): OutUtxo {
  return createOutUtxo({
    owner,
    amounts: [BN_0],
    assets: [SystemProgram.programId],
    isFillingUtxo: true,
    lightWasm,
  });
}

export function createOutUtxo({
  owner,
  amounts,
  assets,
  lightWasm,
  encryptionPublicKey,
  blinding = new BN(randomBN(), 31, "be"),
  isFillingUtxo = false,
}: {
  owner: BN;
  amounts: BN[];
  assets: PublicKey[];
  lightWasm: LightWasm;
  encryptionPublicKey?: Uint8Array;
  blinding?: BN;
  isFillingUtxo?: boolean;
}): OutUtxo {
  const poolType = BN_0;
  const transactionVersion = BN_0;
  const verifierAddressCircuit = "0";
  // verifierAddress.toBase58() === SystemProgram.programId.toBase58()
  //   ? "0"
  //   : hashAndTruncateToCircuit(verifierAddress.toBytes()).toString();
  if (assets.length !== amounts.length) {
    throw new UtxoError(
      UtxoErrorCode.ASSETS_AMOUNTS_LENGTH_MISMATCH,
      "createOutUtxo",
      "Assets and amounts length mismatch",
    );
  }
  while (assets.length < 2) {
    assets.push(SystemProgram.programId);
    amounts.push(BN_0);
  }

  // const utxoDataHash = utxoData
  //   ? createUtxoDataHash(utxoData, lightWasm)
  //   : BN_0;

  const utxoDataHash = BN_0;

  const utxoHashInputs: UtxoHashInputs = {
    publicKey: owner.toString(),
    amounts: amounts.map((amount) => amount.toString()),
    assetsCircuit: assets.map((asset, index) => {
      if (
        index !== 0 &&
        asset.toBase58() === SystemProgram.programId.toBase58()
      )
        return "0";
      return hashAndTruncateToCircuit(asset.toBytes()).toString();
    }),
    blinding: blinding.toString(),
    poolType: poolType.toString(),
    transactionVersion: transactionVersion.toString(),
    utxoDataHash: utxoDataHash.toString(),
    verifierAddressCircuit,
  };
  const utxoHash = getUtxoHash(lightWasm, utxoHashInputs);
  const outUtxo: OutUtxo = {
    owner: utxoHashInputs.owner,
    encryptionPublicKey,
    amounts,
    assets,
    assetsCircuit: utxoHashInputs.assetsCircuit,
    blinding: blinding,
    poolType: utxoHashInputs.poolType,
    utxoHash,
    transactionVersion: utxoHashInputs.version,
    isFillingUtxo,
    verifierAddress,
    verifierAddressCircuit: utxoHashInputs.verifierAddressCircuit,
    utxoDataHash,
    utxoData,
  };
  return outUtxo;
}

export function getUtxoHash(
  lightWasm: LightWasm,
  utxoHashInputs: UtxoHashInputs,
): string {
  const {
    publicKey,
    amounts,
    assetsCircuit,
    blinding,
    poolType,
    transactionVersion,
    utxoDataHash,
    verifierAddressCircuit,
  } = utxoHashInputs;
  const amountHash = lightWasm.poseidonHashString(amounts);
  const assetHash = lightWasm.poseidonHashString(
    assetsCircuit.map((x) => x.toString()),
  );

  if (!publicKey) {
    throw new UtxoError(
      CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
      "getCommitment",
      "Neither Account nor compressionPublicKey was provided",
    );
  }

  return lightWasm.poseidonHashString([
    transactionVersion,
    amountHash,
    publicKey.toString(),
    blinding.toString(),
    assetHash.toString(),
    utxoDataHash.toString(),
    poolType.toString(),
    verifierAddressCircuit.toString(),
  ]);
}

/**
 * @description Parses a utxo to bytes.
 * @returns {Uint8Array}
 */
export async function outUtxoToBytes(
  outUtxo: OutUtxo,
  assetLookupTable: string[],
  compressed: boolean = false,
): Promise<Uint8Array> {
  const serializeObject = {
    ...outUtxo,
    accountCompressionPublicKey: new BN(outUtxo.publicKey),
    accountEncryptionPublicKey: outUtxo.encryptionPublicKey
      ? outUtxo.encryptionPublicKey
      : new Uint8Array(32).fill(0),
    splAssetIndex: new BN(
      assetLookupTable.findIndex(
        (asset) => asset === outUtxo.assets[1].toBase58(),
      ),
    ),
  };
  if (serializeObject.splAssetIndex.toString() === "-1") {
    throw new UtxoError(
      UtxoErrorCode.ASSET_NOT_FOUND,
      "outUtxoToBytes",
      `asset pubkey ${serializeObject.assets[1]}, not found in lookup table`,
    );
  }
  const coder = new BorshAccountsCoder(IDL_LIGHT_PSP2IN2OUT);
  const serializedData = await coder.encode("outUtxo", serializeObject);

  // Compressed serialization does not store the account since for an encrypted utxo
  // we assume that the user who is able to decrypt the utxo knows the corresponding account.
  return compressed
    ? serializedData.subarray(0, COMPRESSED_UTXO_BYTES_LENGTH)
    : serializedData;
}

export function outUtxoFromBytes({
  bytes,
  account,
  assetLookupTable,
  compressed = false,
  lightWasm,
}: {
  bytes: Buffer;
  account?: Account;
  assetLookupTable: string[];
  compressed?: boolean;
  lightWasm: LightWasm;
}): OutUtxo | null {
  // if it is compressed and adds 64 0 bytes padding
  if (compressed) {
    const tmp: Uint8Array = Uint8Array.from([...Array.from(bytes)]);
    bytes = Buffer.from([
      ...tmp,
      ...new Uint8Array(
        UNCOMPRESSED_UTXO_BYTES_LENGTH - COMPRESSED_UTXO_BYTES_LENGTH,
      ).fill(0),
    ]);
    if (!account)
      throw new UtxoError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "fromBytes",
        "For deserializing a compressed utxo an account is required.",
      );
  }
  const coder = new BorshAccountsCoder(IDL_LIGHT_PSP2IN2OUT);
  const decodedUtxoData = coder.decode("outUtxo", bytes);

  const assets = [
    SystemProgram.programId,
    fetchAssetByIdLookUp(decodedUtxoData.splAssetIndex, assetLookupTable),
  ];
  const publicKey = compressed
    ? account?.keypair.publicKey
    : decodedUtxoData.accountCompressionPublicKey;
  // TODO: evaluate whether there is a better way to handle the case of a compressed program utxo which currently not deserialized correctly when taken from encrypted utxos
  if (decodedUtxoData.utxoDataHash.toString() !== "0") {
    return null;
  }
  const outUtxo = createOutUtxo({
    publicKey: new BN(publicKey),
    encryptionPublicKey: new BN(decodedUtxoData.accountEncryptionPublicKey).eq(
      BN_0,
    )
      ? undefined
      : decodedUtxoData.accountEncryptionPublicKey,
    amounts: decodedUtxoData.amounts,
    assets,
    blinding: new BN(decodedUtxoData.blinding),
    lightWasm,
  });
  return outUtxo;
}

export function outUtxoFromString(
  string: string,
  assetLookupTable: string[],
  account: Account,
  lightWasm: LightWasm,
  compressed: boolean = false,
): OutUtxo | null {
  return outUtxoFromBytes({
    bytes: bs58.decode(string),
    assetLookupTable,
    account,
    compressed,
    lightWasm,
  });
}

/**
 * Converts the Utxo instance into a base58 encoded string.
 * @async
 * @returns {Promise<string>} A promise that resolves to the base58 encoded string representation of the Utxo.
 */
export async function outUtxoToString(
  utxo: OutUtxo,
  assetLookupTable: string[],
): Promise<string> {
  const bytes = await outUtxoToBytes(utxo, assetLookupTable);
  return bs58.encode(bytes);
}

export async function encryptOutUtxo({
  utxo,
  lightWasm,
  account,
  merkleTreePdaPublicKey,
  compressed = false,
  assetLookupTable,
}: {
  utxo: OutUtxo;
  lightWasm: LightWasm;
  account?: Account;
  merkleTreePdaPublicKey?: PublicKey;
  compressed?: boolean;
  assetLookupTable: string[];
}): Promise<Uint8Array> {
  const bytes = await outUtxoToBytes(utxo, assetLookupTable, compressed);
  const byteArray = new Uint8Array(bytes);
  const utxoHash = new BN(utxo.utxoHash).toArrayLike(Buffer, "be", 32);
  const encryptedUtxo = await encryptOutUtxoInternal({
    bytes: byteArray,
    utxoHash,
    lightWasm,
    account,
    merkleTreePdaPublicKey,
    compressed,
    publicKey: utxo.publicKey,
    encryptionPublicKey: utxo.encryptionPublicKey,
  });
  return encryptedUtxo;
}

export async function encryptOutUtxoInternal({
  bytes,
  lightWasm,
  account,
  merkleTreePdaPublicKey,
  compressed = false,
  isFillingUtxo = false,
  encryptionPublicKey,
  publicKey,
  utxoHash,
}: {
  bytes: Uint8Array;
  lightWasm: LightWasm;
  account?: Account;
  merkleTreePdaPublicKey?: PublicKey;
  compressed?: boolean;
  isFillingUtxo?: boolean;
  encryptionPublicKey?: Uint8Array;
  publicKey: string;
  utxoHash: Uint8Array;
}): Promise<Uint8Array> {
  if (encryptionPublicKey) {
    const ciphertext = lightWasm.encryptNaclUtxo(
      encryptionPublicKey,
      bytes,
      utxoHash,
    );
    // TODO: add option to use random or dedicated prefix for asynmetrically encrypted utxos which are sent to another party
    const prefix = !isFillingUtxo
      ? new BN(encryptionPublicKey).toArray("be", 32).slice(0, 4)
      : randomPrefixBytes();
    return Uint8Array.from([...prefix, ...ciphertext]);
  } else if (account) {
    if (!merkleTreePdaPublicKey)
      throw new UtxoError(
        UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED,
        "encrypt",
        "For aes encryption the merkle tree pda publickey is necessary to derive the viewingkey",
      );

    const ciphertext = account.encryptAesUtxo(
      bytes,
      merkleTreePdaPublicKey,
      utxoHash,
    );
    // If utxo is filling utxo we don't want to decrypt it in the future, so we use a random prefix
    // we still want to encrypt it properly to be able to decrypt it if necessary as a safeguard.
    const prefix = !isFillingUtxo
      ? account.generateLatestUtxoPrefixHash(merkleTreePdaPublicKey)
      : randomPrefixBytes();
    if (!compressed) return Uint8Array.from([...prefix, ...ciphertext]);

    // adding the 8 bytes as padding at the end to make the ciphertext the same length as nacl box ciphertexts of (120 + PREFIX_LENGTH) bytes
    return Uint8Array.from([...prefix, ...ciphertext]);
  } else {
    throw new UtxoError(
      CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
      "encrypt",
      "Neither account nor this.encryptionPublicKey is defined",
    );
  }
}
export async function decryptOutUtxo({
  lightWasm,
  encBytes,
  account,
  merkleTreePdaPublicKey,
  aes,
  utxoHash,
  compressed = false,
  assetLookupTable,
}: {
  lightWasm: LightWasm;
  encBytes: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  aes: boolean;
  utxoHash: Uint8Array;
  compressed?: boolean;
  assetLookupTable: string[];
}): Promise<Result<OutUtxo | null, UtxoError>> {
  const cleartext = await decryptOutUtxoInternal({
    lightWasm,
    encBytes,
    account,
    merkleTreePdaPublicKey,
    aes,
    utxoHash,
    compressed,
  });
  if (!cleartext || cleartext.error || !cleartext.value) return Result.Ok(null);
  const bytes = Buffer.from(cleartext.value);
  const outUtxo = outUtxoFromBytes({
    lightWasm,
    bytes,
    account,
    assetLookupTable,
    compressed,
  });

  return Result.Ok(outUtxo);
}

export async function decryptOutUtxoInternal({
  lightWasm,
  encBytes,
  account,
  merkleTreePdaPublicKey,
  aes,
  utxoHash,
  compressed = false,
}: {
  lightWasm: LightWasm;
  encBytes: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  aes: boolean;
  utxoHash: Uint8Array;
  compressed?: boolean;
}): Promise<Result<Uint8Array | null, Error>> {
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
    ? account.decryptAesUtxo(encBytes, merkleTreePdaPublicKey, utxoHash)
    : await account.decryptNaclUtxo(encBytes, utxoHash);

  return cleartext;
}

export function createFillingUtxo({
  lightWasm,
  account,
}: {
  lightWasm: LightWasm;
  account: Account;
}): Utxo {
  const outFillingUtxo = createFillingOutUtxo({
    lightWasm,
    owner: account.keypair.publicKey,
  });
  return outUtxoToUtxo(
    outFillingUtxo,
    new Array(MERKLE_TREE_HEIGHT).fill("0"),
    0,
    lightWasm,
    account,
  );
}

export async function decryptUtxo(
  encBytes: Uint8Array,
  account: Account,
  merkleTreePdaPublicKey: PublicKey,
  aes: boolean,
  utxoHash: Uint8Array,
  lightWasm: LightWasm,
  compressed: boolean = true,
  merkleProof: string[],
  merkleTreeLeafIndex: number,
  assetLookupTable: string[],
): Promise<Result<Utxo | null, UtxoError>> {
  const decryptedOutUtxo = await decryptOutUtxo({
    encBytes,
    account,
    merkleTreePdaPublicKey,
    aes,
    utxoHash,
    lightWasm,
    compressed,
    assetLookupTable,
  });
  if (!decryptedOutUtxo.value) {
    return decryptedOutUtxo as Result<Utxo | null, UtxoError>;
  }

  return Result.Ok(
    outUtxoToUtxo(
      decryptedOutUtxo.value,
      merkleProof,
      merkleTreeLeafIndex,
      lightWasm,
      account,
    ),
  );
}

export function outUtxoToUtxo(
  outUtxo: OutUtxo,
  merkleProof: string[],
  merkleTreeLeafIndex: number,
  lightWasm: LightWasm,
  account: Account,
): Utxo {
  const inputs: CreateUtxoInputs = {
    utxoHash: outUtxo.utxoHash,
    blinding: outUtxo.blinding.toString(),
    amounts: outUtxo.amounts,
    assets: outUtxo.assets,
    merkleProof,
    merkleTreeLeafIndex,
    verifierAddress:
      outUtxo.verifierAddress.toBase58() === SystemProgram.programId.toBase58()
        ? undefined
        : outUtxo.verifierAddress,
  };
  return createUtxo(lightWasm, account, inputs, outUtxo.isFillingUtxo);
}

export function createTestInUtxo({
  account,
  encryptionPublicKey,
  amounts,
  assets,
  blinding,
  isFillingUtxo,
  lightWasm,
  verifierAddress,
  merkleProof = ["1"],
  merkleTreeLeafIndex = 0,
}: {
  account: Account;
  encryptionPublicKey?: Uint8Array;
  amounts: BN[];
  assets: PublicKey[];
  blinding?: BN;
  isFillingUtxo?: boolean;
  lightWasm: LightWasm;
  verifierAddress?: PublicKey;
  merkleProof?: string[];
  merkleTreeLeafIndex?: number;
}): Utxo {
  const outUtxo = createOutUtxo({
    publicKey: account.keypair.publicKey,
    encryptionPublicKey,
    amounts,
    assets,
    blinding,
    isFillingUtxo,
    lightWasm,
    verifierAddress,
  });
  return outUtxoToUtxo(
    outUtxo,
    merkleProof,
    merkleTreeLeafIndex,
    lightWasm,
    account,
  );
}

export function createUtxo(
  lightWasm: LightWasm,
  account: Account,
  createUtxoInputs: CreateUtxoInputs,
  isFillingUtxo: boolean,
): Utxo {
  const {
    merkleTreeLeafIndex,
    utxoHash,
    blinding,
    amounts,
    assets,
    merkleProof,
    verifierAddress: verifierAddressInput,
    utxoDataHash: utxoDataHashInput,
    utxoData,
    utxoName,
  } = createUtxoInputs;
  while (assets.length < 2) {
    assets.push(SystemProgram.programId);
    amounts.push(BN_0);
  }

  const utxoNameInternal = utxoName ? utxoName : "native";
  const poolType = "0";
  const transactionVersion = "0";
  const merkleTreeLeafIndexInternal = isFillingUtxo ? 0 : merkleTreeLeafIndex;
  const merkleProofInternal = isFillingUtxo
    ? new Array(18).fill("0")
    : merkleProof;
  const verifierAddress = verifierAddressInput
    ? verifierAddressInput
    : SystemProgram.programId;
  const verifierAddressCircuit = verifierAddressInput
    ? hashAndTruncateToCircuit(verifierAddressInput.toBytes()).toString()
    : "0";
  const utxoDataHash = utxoDataHashInput ? utxoDataHashInput : "0";
  if (utxoDataHashInput && !utxoData) {
    throw new UtxoError(
      CreateUtxoErrorCode.UTXO_DATA_UNDEFINED,
      "createUtxo",
      "Utxo data is undefined",
    );
  }

  if (merkleTreeLeafIndexInternal === undefined) {
    throw new UtxoError(
      CreateUtxoErrorCode.MERKLE_TREE_INDEX_UNDEFINED,
      "createUtxo",
      "Merkle tree index is undefined",
    );
  }

  const signature = account
    .sign(utxoHash, merkleTreeLeafIndexInternal)
    .toString();

  const nullifierInputs: NullifierInputs = {
    signature,
    utxoHash,
    merkleTreeLeafIndex: merkleTreeLeafIndexInternal.toString(),
  };
  const nullifier = getNullifier(lightWasm, nullifierInputs);
  const utxo: Utxo = {
    publicKey: account.keypair.publicKey.toString(),
    amounts,
    assets,
    assetsCircuit: assets.map((asset, index) => {
      if (
        index !== 0 &&
        asset.toBase58() === SystemProgram.programId.toBase58()
      )
        return "0";
      return hashAndTruncateToCircuit(asset.toBytes()).toString();
    }),
    blinding,
    poolType,
    utxoHash,
    transactionVersion,
    verifierAddress,
    verifierAddressCircuit,
    isFillingUtxo,
    nullifier,
    merkleTreeLeafIndex: merkleTreeLeafIndexInternal,
    merkleProof: merkleProofInternal,
    utxoDataHash,
    utxoName: utxoNameInternal,
    utxoData,
  };
  return utxo;
}

export function getNullifier(
  lightWasm: LightWasm,
  inputs: NullifierInputs,
): string {
  return lightWasm.poseidonHashString([
    inputs.utxoHash,
    inputs.merkleTreeLeafIndex,
    inputs.signature,
  ]);
}
