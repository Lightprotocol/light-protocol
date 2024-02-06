import nacl from "tweetnacl";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";

import { LightWasm } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "../types";
import { Account } from "../account";
import {
  UTXO_PREFIX_LENGTH,
  BN_0,
  COMPRESSED_UTXO_BYTES_LENGTH,
  UNCOMPRESSED_UTXO_BYTES_LENGTH,
  ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  MERKLE_TREE_HEIGHT,
  DEFAULT_UTXO_TYPE,
  UTXO_VERSION_V0,
  UTXO_POOL_TYPE_V0,
} from "../constants";
import { UtxoError, UtxoErrorCode, CreateUtxoErrorCode } from "../errors";
import { IDL_LIGHT_PSP2IN2OUT } from "../idls";
import { hashAndTruncateToCircuit } from "../utils/hash-utils";
import { fetchAssetByIdLookUp } from "../utils/fetch-utils";

import { BN254, createBN254 } from "./bn254";
import {
  PlaceHolderTData,
  ProgramOutUtxo,
  programOutUtxoToBytes,
} from "./program-utxo";

export const randomBN = (nbytes = 30) => new BN(nacl.randomBytes(nbytes));
const randomPrefixBytes = () => nacl.randomBytes(UTXO_PREFIX_LENGTH);

/** Public key of Poseidon-hashed keypair */
type CompressionPublicKey = BN254;

/** Describes the generic utxo details applicable to every utxo. */
export type BaseUtxo = {
  /** Identifier and commitment to the utxo, is inserted as leaf into state tree */
  hash: BN254;
  /** Compression public key of the user or public key program owning the utxo */
  owner: CompressionPublicKey | PublicKey;
  /** Optional number of lamports and SPL amount assigned to the utxo */
  amounts: BN[];
  /** Optional native mint and SPL mint address respective to 'amounts' */
  assets: PublicKey[];
  /** Random value to force uniqueness of 'hash' */
  blinding: BN254;
  /** Optional type of utxo for custom circuits, defaults to 'native' */
  type: string;
  /** Default to '0' */
  version: string;
  /** Default to '0' */
  poolType: string;
  /** Indicator for whether the utxo is empty, for readability */
  isFillingUtxo: boolean;
  /** Default 'true'. Whether the inputs to 'hash' are public or not. Useful for confidential compute. */
  isPublic: boolean;
  /** Optional persistent id of the utxo. Used for compressed PDAs and non-fungible tokens */
  address?: BN254;
  /** Optional public key of program that owns the metadata */
  metadataOwner?: PublicKey;
  /**
   *	metadata which is immutable in normal transactions.
   *	metadata can be updated by the metadataOwner with a dedicated system psp.
   */
  metadata?: any; /// TODO: add metadata type
  // /** hash of metadata */
  // metadataHash?: string;
  /** hash of metadataHash and metadataOwner */
  metaHash?: BN254;
};

/** Utxo that had previously been inserted into a state Merkle tree */
export type Utxo = Omit<BaseUtxo, "owner"> & {
  /** Compression public key of the user that owns the utxo */
  owner: CompressionPublicKey;
  /** Hash that invalidates utxo once inserted into nullifier queue, if isPublic = true it defaults to: 'hash' */
  nullifier: BN254;
  /** Numerical identifier of the Merkle tree which the 'hash' is part of */
  merkletreeId: number;
  /** Proof path attached to the utxo. Can be reconstructed using event history */
  merkleProof: string[];
  /** Index of 'hash' as inserted into the Merkle tree. Max safe tree depth using number type would be **52, roughly 4.5 x 10^15 leaves */
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

/** Type safety: enforce that the utxo is not encrypted */
export type Public = { isPublic: true };
export type PublicBaseUtxo = BaseUtxo & Public;
export type PublicUtxo = Utxo & Public;
export type PublicOutUtxo = OutUtxo & Public;

export type NullifierInputs = {
  signature: BN;
  /** hash of the utxo preimage */
  hash: BN254;
  merkleTreeLeafIndex: BN;
};

type UtxoHashInputs = {
  owner: string;
  amounts: string[];
  assetsCircuitInput: string[];
  blinding: string;
  poolType: string;
  version: string;
  dataHash: string;
  metaHash: string;
  address: string;
};

export function createFillingOutUtxo({
  lightWasm,
  owner,
}: {
  lightWasm: LightWasm;
  owner: CompressionPublicKey;
}): OutUtxo {
  return createOutUtxo({
    owner,
    amounts: [BN_0],
    assets: [SystemProgram.programId],
    isFillingUtxo: true,
    lightWasm,
  });
}

export function checkAssetAndAmountIntegrity(
  assets: PublicKey[],
  amounts: BN[],
): { assets: PublicKey[]; amounts: BN[] } {
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
  return { assets, amounts };
}

export const getDefaultUtxoTypeAndVersionV0 = (): {
  type: "native";
  version: "0";
  poolType: "0";
} => ({
  type: DEFAULT_UTXO_TYPE,
  version: UTXO_VERSION_V0,
  poolType: UTXO_POOL_TYPE_V0,
});

/**
 * Hashes and truncates assets to fit within 254-bit modulo space.
 * Returns decimal string.
 * SOL = "0"
 * */
export function stringifyAssetsToCircuitInput(assets: PublicKey[]): string[] {
  return assets.map((asset: PublicKey, index) => {
    if (index !== 0 && asset.toBase58() === SystemProgram.programId.toBase58())
      return "0";
    return hashAndTruncateToCircuit(asset.toBytes()).toString();
  });
}

/** Utxo and ProgramUtxo */
export function getUtxoHashInputs(
  owner: BN254,
  amounts: BN[],
  assets: PublicKey[],
  blinding: BN254,
  poolType: string,
  version: string,
  dataHash?: BN254,
  metaHash?: BN254,
  address?: BN254,
): UtxoHashInputs {
  return {
    owner: owner.toString(),
    amounts: amounts.map((amount) => amount.toString()),
    assetsCircuitInput: stringifyAssetsToCircuitInput(assets),
    blinding: blinding.toString(),
    poolType,
    version,
    dataHash: dataHash?.toString() || "0",
    metaHash: metaHash?.toString() || "0",
    address: address?.toString() || "0",
  };
}

/** Utxo only */
export function createOutUtxo({
  owner,
  amounts,
  assets,
  lightWasm,
  blinding = new BN(randomBN(), 30, "be"),
  encryptionPublicKey,
  isFillingUtxo = false,
  metaHash,
  address,
}: {
  owner: CompressionPublicKey;
  amounts: BN[];
  assets: PublicKey[];
  lightWasm: LightWasm;
  blinding?: BN254;
  encryptionPublicKey?: Uint8Array;
  isFillingUtxo?: boolean;
  metaHash?: BN254;
  address?: BN254;
}): OutUtxo {
  const { poolType, version, type } = getDefaultUtxoTypeAndVersionV0();

  ({ assets, amounts } = checkAssetAndAmountIntegrity(assets, amounts));

  const utxoHashInputs = getUtxoHashInputs(
    owner,
    amounts,
    assets,
    blinding,
    poolType,
    version,
    undefined,
    metaHash,
    address,
  );

  const hash = getUtxoHash(lightWasm, utxoHashInputs);

  return {
    owner,
    encryptionPublicKey,
    amounts,
    assets,
    type,
    blinding,
    poolType,
    hash,
    version,
    isFillingUtxo,
    metaHash,
    address,
    isPublic: false, // TODO: make isPublic dynamic
  };
}

/** Utxo and ProgramUtxo */
export function getUtxoHash(
  lightWasm: LightWasm,
  utxoHashInputs: UtxoHashInputs,
): BN254 {
  const {
    owner,
    amounts,
    assetsCircuitInput,
    blinding,
    poolType,
    version,
    dataHash,
    metaHash,
    address,
  } = utxoHashInputs;

  const amountHash = lightWasm.poseidonHashString(amounts);
  const assetHash = lightWasm.poseidonHashString(
    assetsCircuitInput.map((x) => x.toString()),
  );

  return lightWasm.poseidonHashBN([
    version,
    amountHash,
    owner,
    blinding,
    assetHash,
    dataHash,
    poolType,
    metaHash,
    address,
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
    /// TODO: currently, the system-idl assumes that utxoDataHash is present with value "0".
    /// However, the OutUtxo type doesnt have a dataHash field.
    /// Hence this default value.
    utxoDataHash: BN_0,
    accountCompressionPublicKey: new BN(outUtxo.owner),
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

  let decodedUtxo;
  try {
    decodedUtxo = coder.decode("outUtxo", bytes);
  } catch (e) {
    /// TODO: inspect this. Currently, the system-idl assumes that utxoDataHash is present with value "0".
    /// However, the OutUtxo type doesnt have a dataHash field. Hence decoding throws.
    return null;
  }
  // TODO: evaluate whether there is a better way to handle the case of a compressed program utxo which currently not deserialized correctly when taken from encrypted utxos
  if (decodedUtxo.utxoDataHash.toString() !== "0") return null;

  const assets = [
    SystemProgram.programId,
    fetchAssetByIdLookUp(decodedUtxo.splAssetIndex, assetLookupTable),
  ];
  const owner = compressed
    ? account?.keypair.publicKey
    : decodedUtxo.accountCompressionPublicKey;

  const outUtxo = createOutUtxo({
    owner: createBN254(owner),
    encryptionPublicKey: new BN(decodedUtxo.accountEncryptionPublicKey).eq(BN_0)
      ? undefined
      : decodedUtxo.accountEncryptionPublicKey,
    amounts: decodedUtxo.amounts,
    assets,
    blinding: decodedUtxo.blinding,
    lightWasm,
    metaHash: decodedUtxo.metaHash, // check whether need to force BN254
    address: decodedUtxo.address,
  });
  return outUtxo;
}

/** Create outUtxo from base58 string */
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

/** Convert utxo instance into a base58-encoded string. */
export async function outUtxoToString(
  utxo: OutUtxo,
  assetLookupTable: string[],
): Promise<string> {
  const bytes = await outUtxoToBytes(utxo, assetLookupTable);
  return bs58.encode(bytes);
}

/** Encrypt transaction output utxo */
export async function encryptOutUtxo({
  utxo,
  lightWasm,
  account,
  merkleTreePdaPublicKey,
  compressed = false,
  assetLookupTable,
}: {
  utxo: ProgramOutUtxo<PlaceHolderTData> | OutUtxo;
  lightWasm: LightWasm;
  account?: Account;
  merkleTreePdaPublicKey?: PublicKey;
  compressed?: boolean;
  assetLookupTable: string[];
}): Promise<Uint8Array> {
  const bytes =
    "data" in utxo
      ? await programOutUtxoToBytes(utxo, assetLookupTable, compressed)
      : await outUtxoToBytes(utxo, assetLookupTable, compressed);
  const byteArray = new Uint8Array(bytes);

  const hash32 = utxo.hash.toArrayLike(Buffer, "be", 32);

  const encryptedUtxo = await encryptOutUtxoInternal({
    bytes: byteArray,
    hash: hash32,
    lightWasm,
    account,
    merkleTreePdaPublicKey,
    compressed,
    encryptionPublicKey: utxo.encryptionPublicKey,
  });
  return encryptedUtxo;
}

export async function encryptOutUtxoInternal({
  bytes,
  hash,
  lightWasm,
  account,
  merkleTreePdaPublicKey,
  compressed = false,
  isFillingUtxo = false,
  encryptionPublicKey,
}: {
  bytes: Uint8Array;
  lightWasm: LightWasm;
  hash: Uint8Array;
  account?: Account;
  merkleTreePdaPublicKey?: PublicKey;
  compressed?: boolean;
  isFillingUtxo?: boolean;
  encryptionPublicKey?: Uint8Array;
}): Promise<Uint8Array> {
  if (encryptionPublicKey) {
    const ciphertext = lightWasm.encryptNaclUtxo(
      encryptionPublicKey,
      bytes,
      hash,
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
      hash,
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
    bytes,
    account,
    assetLookupTable,
    compressed,
    lightWasm,
  });

  return Result.Ok(outUtxo);
}

export async function decryptOutUtxoInternal({
  encBytes,
  account,
  merkleTreePdaPublicKey,
  aes,
  utxoHash,
  compressed = false,
}: {
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
    : account.decryptNaclUtxo(encBytes, utxoHash);

  return cleartext;
}

export type CreateUtxoInputs = {
  hash: BN254;
  owner: CompressionPublicKey;
  amounts: BN[];
  assets: PublicKey[];
  blinding: BN254;
  merkleProof: string[];
  merkleTreeLeafIndex?: number;
  address?: BN254;
  metaHash?: BN254;
};

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
    hash: outUtxo.hash,
    blinding: outUtxo.blinding,
    amounts: outUtxo.amounts,
    assets: outUtxo.assets,
    merkleProof,
    merkleTreeLeafIndex,
    metaHash: outUtxo.metaHash,
    address: outUtxo.address,
    owner: outUtxo.owner,
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
  merkleProof?: string[];
  merkleTreeLeafIndex?: number;
}): Utxo {
  const outUtxo = createOutUtxo({
    owner: account.keypair.publicKey,
    encryptionPublicKey,
    amounts,
    assets,
    blinding,
    isFillingUtxo,
    lightWasm,
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
    hash,
    blinding,
    amounts: uncheckedAmounts,
    assets: uncheckedAssets,
    merkleProof,
    metaHash,
    address,
    owner,
  } = createUtxoInputs;

  const { assets, amounts } = checkAssetAndAmountIntegrity(
    uncheckedAssets,
    uncheckedAmounts,
  );

  const { poolType, version, type } = getDefaultUtxoTypeAndVersionV0();

  if (merkleTreeLeafIndex === undefined) {
    throw new UtxoError(
      CreateUtxoErrorCode.MERKLE_TREE_INDEX_UNDEFINED,
      "createUtxo",
      "Merkle tree index is undefined",
    );
  }

  const merkleTreeLeafIndexBN = isFillingUtxo
    ? BN_0
    : new BN(merkleTreeLeafIndex);

  const merkleProofInternal = isFillingUtxo
    ? new Array(MERKLE_TREE_HEIGHT).fill("0")
    : merkleProof;

  const nullifier = createNullifierWithAccountSignature(
    account,
    hash,
    merkleTreeLeafIndexBN,
    lightWasm,
  );

  const utxo: Utxo = {
    owner,
    amounts,
    assets,
    blinding,
    poolType,
    hash,
    version,
    isFillingUtxo,
    nullifier,
    merkleTreeLeafIndex: merkleTreeLeafIndexBN.toNumber(),
    merkleProof: merkleProofInternal,
    merkletreeId: BN_0.toNumber(), // TODO: make merkletreeId dynamic (support parallel merkle trees)
    type,
    metaHash,
    address,
    isPublic: false, // TODO: make isPublic dynamic
  };
  return utxo;
}

export function createNullifierWithAccountSignature(
  account: Account,
  hash: BN254,
  merkleTreeLeafIndexInternal: BN254,
  lightWasm: LightWasm,
): BN254 {
  /// TODO: account.sign should accept BN for hash and leafIndex
  const signature = account.sign(
    hash.toString(),
    merkleTreeLeafIndexInternal.toNumber(),
  );

  const nullifierInputs: NullifierInputs = {
    signature,
    hash,
    merkleTreeLeafIndex: merkleTreeLeafIndexInternal,
  };

  return getNullifier(lightWasm, nullifierInputs);
}

/** Utility, derive nullifier hash */
export function getNullifier(
  lightWasm: LightWasm,
  inputs: NullifierInputs,
): BN254 {
  return lightWasm.poseidonHashBN([
    inputs.hash,
    inputs.merkleTreeLeafIndex,
    inputs.signature,
  ]);
}
