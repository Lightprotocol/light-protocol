import nacl from "tweetnacl";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
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
import { LightWasm } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "./types";

export const randomBN = (nbytes = 30) => {
  return new BN(nacl.randomBytes(nbytes));
};
export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
export const randomPrefixBytes = () => nacl.randomBytes(UTXO_PREFIX_LENGTH);

export type OutUtxo = {
  publicKey: string;
  encryptionPublicKey?: Uint8Array; // is only set if the utxo should be sent to another public key and thus be encrypted asymetrically
  amounts: BN[];
  assets: PublicKey[];
  assetsCircuit: string[];
  blinding: BN;
  poolType: string;
  utxoHash: string;
  transactionVersion: string;
  isFillingUtxo: boolean;
  utxoDataHash: BN;
  utxoData?: any;
  metaHash?: BN;
  address?: BN;
};

export function createFillingOutUtxo({
  lightWasm,
  publicKey,
}: {
  lightWasm: LightWasm;
  publicKey: BN;
}): OutUtxo {
  return createOutUtxo({
    publicKey,
    amounts: [BN_0],
    assets: [SystemProgram.programId],
    isFillingUtxo: true,
    lightWasm,
  });
}

export function createOutUtxo({
  publicKey,
  encryptionPublicKey,
  amounts,
  assets,
  blinding = new BN(randomBN(), 31, "be"),
  isFillingUtxo = false,
  lightWasm,
  utxoDataHash = BN_0,
  metaHash,
  address,
  utxoData,
}: {
  publicKey: BN;
  encryptionPublicKey?: Uint8Array;
  amounts: BN[];
  assets: PublicKey[];
  blinding?: BN;
  isFillingUtxo?: boolean;
  lightWasm: LightWasm;
  utxoDataHash?: any;
  metaHash?: BN;
  address?: BN;
  utxoData?: any;
}): OutUtxo {
  const poolType = BN_0;
  const transactionVersion = BN_0;
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
  if (utxoData !== undefined && utxoDataHash === "0") {
    throw new UtxoError(
      UtxoErrorCode.UTXO_DATA_HASH_UNDEFINED,
      "createOutUtxo",
    );
  }

  const utxoHashInputs: UtxoHashInputs = {
    publicKey: publicKey.toString(),
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
    metaHash: metaHash ? metaHash.toString() : "0",
    address: address ? address.toString() : "0",
  };
  const utxoHash = getUtxoHash(lightWasm, utxoHashInputs);
  const outUtxo: OutUtxo = {
    publicKey: utxoHashInputs.publicKey,
    encryptionPublicKey,
    amounts,
    assets,
    assetsCircuit: utxoHashInputs.assetsCircuit,
    blinding: blinding,
    poolType: utxoHashInputs.poolType,
    utxoHash,
    transactionVersion: utxoHashInputs.transactionVersion,
    isFillingUtxo,
    utxoDataHash,
    utxoData,
    metaHash,
    address,
  };
  return outUtxo;
}

type UtxoHashInputs = {
  publicKey: string;
  amounts: string[];
  assetsCircuit: string[];
  blinding: string;
  poolType: string;
  transactionVersion: string;
  utxoDataHash: string;
  metaHash: string;
  address: string;
};

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
    metaHash,
    address,
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
  // console.log("transactionVersion", transactionVersion);
  // console.log("amountHash", amountHash);
  // console.log("publicKey", publicKey);
  // console.log("blinding", blinding);
  // console.log("assetHash", assetHash);
  // console.log("utxoDataHash", utxoDataHash);
  // console.log("poolType", poolType);
  // console.log("metaHash", metaHash);
  // console.log("address", address);
  return lightWasm.poseidonHashString([
    transactionVersion,
    amountHash,
    publicKey.toString(),
    blinding.toString(),
    assetHash.toString(),
    utxoDataHash.toString(),
    poolType.toString(),
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

export type Utxo = {
  publicKey: string;
  amounts: BN[];
  assets: PublicKey[];
  assetsCircuit: string[];
  blinding: string;
  poolType: string;
  utxoHash: string;
  transactionVersion: string;
  isFillingUtxo: boolean; // should I serialize this as well? no
  nullifier: string;
  merkleTreeLeafIndex: number;
  merkleProof: string[];
  utxoDataHash: string;
  utxoData: any;
  utxoName: string;
  metaHash?: BN;
  address?: BN;
};

export type NullifierInputs = {
  signature: string;
  utxoHash: string;
  merkleTreeLeafIndex: string;
};

export type CreateUtxoInputs = {
  utxoHash: string;
  blinding: string;
  amounts: BN[];
  assets: PublicKey[];
  merkleTreeLeafIndex?: number;
  merkleProof: string[];
  utxoDataHash?: string;
  utxoData?: any;
  utxoName?: string;
  address?: BN;
  metaHash?: BN;
  owner?: PublicKey;
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
    publicKey: account.keypair.publicKey,
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
  programOwner?: PublicKey,
  utxoData?: any,
): Utxo {
  const inputs: CreateUtxoInputs = {
    utxoHash: outUtxo.utxoHash,
    blinding: outUtxo.blinding.toString(),
    amounts: outUtxo.amounts,
    assets: outUtxo.assets,
    merkleProof,
    merkleTreeLeafIndex,
    metaHash: outUtxo.metaHash,
    address: outUtxo.address,
    utxoDataHash: outUtxo.utxoDataHash.toString(),
    owner: programOwner,
    utxoData,
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
    publicKey: account.keypair.publicKey,
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
    utxoHash,
    blinding,
    amounts,
    assets,
    merkleProof,
    utxoDataHash: utxoDataHashInput,
    utxoData,
    utxoName,
    metaHash,
    address,
    owner: ownerInput,
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
  if (
    utxoDataHashInput !== undefined &&
    !ownerInput &&
    utxoDataHashInput !== "0"
  ) {
    throw new UtxoError(
      UtxoErrorCode.NO_PUBLIC_KEY_PROVIDED_FOR_PROGRAM_UTXO,
      "createUtxo",
    );
  }
  const owner =
    utxoDataHashInput !== "0" && utxoDataHashInput !== undefined
      ? hashAndTruncateToCircuit(ownerInput!.toBytes()).toString()
      : account.keypair.publicKey.toString();
  const utxoDataHash =
    utxoDataHashInput !== "0" && utxoDataHashInput !== undefined
      ? utxoDataHashInput
      : "0";
  if (
    utxoDataHashInput !== "0" &&
    utxoDataHashInput !== undefined &&
    !utxoData
  ) {
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
    publicKey: owner,
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
    isFillingUtxo,
    nullifier,
    merkleTreeLeafIndex: merkleTreeLeafIndexInternal,
    merkleProof: merkleProofInternal,
    utxoDataHash,
    utxoName: utxoNameInternal,
    utxoData,
    metaHash,
    address,
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
