import nacl from "tweetnacl";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import {
  Account,
  BN_0,
  COMPRESSED_UTXO_BYTES_LENGTH,
  CreateUtxoErrorCode,
  ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  fetchAssetByIdLookUp,
  hashAndTruncateToCircuit,
  IDL_LIGHT_PSP2IN2OUT,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  randomPrefixBytes,
  UNCOMPRESSED_UTXO_BYTES_LENGTH,
  UTXO_PREFIX_LENGTH,
  UtxoError,
  UtxoErrorCode,
} from "./index";
import { LightWasm } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "./types";

export const randomBN = (nbytes = 30) => {
  return new anchor.BN(nacl.randomBytes(nbytes));
};
const { sha3_256 } = require("@noble/hashes/sha3");
const anchor = require("@coral-xyz/anchor");

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
  verifierAddress: PublicKey;
  verifierAddressCircuit: string;
  isFillingUtxo: boolean;
  utxoDataHash: BN;
};

export function createOutUtxo({
  publicKey,
  encryptionPublicKey,
  amounts,
  assets,
  blinding = new BN(randomBN(), 31, "be"),
  isFillingUtxo = false, lightWasm,
  verifierAddress = SystemProgram.programId,
  utxoDataHash = BN_0,
}: {
  publicKey: BN;
  encryptionPublicKey?: Uint8Array;
  amounts: BN[];
  assets: PublicKey[];
  blinding?: BN;
  isFillingUtxo?: boolean;
  lightWasm: LightWasm;
  verifierAddress?: PublicKey;
  utxoDataHash?: BN;
}): OutUtxo {
  const poolType = BN_0;
  const transactionVersion = BN_0;
  const verifierAddressCircuit =
    verifierAddress.toBase58() === SystemProgram.programId.toBase58()
      ? "0"
      : hashAndTruncateToCircuit(verifierAddress.toBytes()).toString();
  const utxoHashInputs: UtxoHashInputs = {
    publicKey: publicKey.toString(),
    amounts: amounts.map((amount) => amount.toString()),
    assetsCircuit: assets.map((asset) =>
      hashAndTruncateToCircuit(asset.toBytes()).toString(),
    ),
    blinding: blinding.toString(),
    poolType: poolType.toString(),
    transactionVersion: transactionVersion.toString(),
    utxoDataHash: utxoDataHash.toString(),
    verifierAddressCircuit,
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
    verifierAddress: SystemProgram.programId,
    verifierAddressCircuit: utxoHashInputs.verifierAddressCircuit,
    utxoDataHash,
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
  verifierAddressCircuit: string;
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
      "Neither Account nor shieldedPublicKey was provided",
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
    accountShieldedPublicKey: new BN(outUtxo.publicKey),
    accountEncryptionPublicKey: outUtxo.encryptionPublicKey
      ? outUtxo.encryptionPublicKey
      : new Uint8Array(32).fill(0),
    verifierAddressIndex: 0,
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
}): OutUtxo {
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
    : decodedUtxoData.accountShieldedPublicKey;

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
): OutUtxo {
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
      ? new BN(publicKey).toArray("be", 32).slice(0, 4)
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

    const decrypted = account.decryptAesUtxo(
      ciphertext,
      merkleTreePdaPublicKey,
      utxoHash,
    );
    // If utxo is filling utxo we don't want to decrypt it in the future, so we use a random prefix
    // we still want to encrypt it properly to be able to decrypt it if necessary as a safeguard.
    const prefix = !isFillingUtxo
      ? account.generateLatestUtxoPrefixHash(merkleTreePdaPublicKey)
      : randomPrefixBytes();
    if (!compressed) return Uint8Array.from([...prefix, ...ciphertext]);
    const padding = sha3_256
      .create()
      .update(Uint8Array.from([...utxoHash, ...bytes]))
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

  return Result.Ok(
    outUtxoFromBytes({
      lightWasm,
      bytes,
      account,
      assetLookupTable,
      compressed,
    }),
  );
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
    : account.decryptNaclUtxo(encBytes, utxoHash);

  return cleartext;
}

export type UtxoNew = {
  publicKey: string;
  amounts: BN[];
  assets: PublicKey[];
  assetsCircuit: string[];
  blinding: string;
  poolType: string;
  utxoHash: string;
  transactionVersion: string;
  verifierAddress: PublicKey;
  verifierAddressCircuit: string;
  isFillingUtxo: boolean; // should I serialize this as well?
  nullifier: string;
  merkleTreeLeafIndex: number;
  merkleProof: string[];
  utxoDataHash: string;
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
  verifierAddress?: PublicKey;
  utxoDataHash?: string;
};

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
): Promise<Result<UtxoNew | null, UtxoError>> {
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
    return decryptedOutUtxo as Result<UtxoNew | null, UtxoError>;
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
  hasher: LightWasm,
  account: Account,
): UtxoNew {
  const inputs: CreateUtxoInputs = {
    utxoHash: outUtxo.utxoHash,
    blinding: outUtxo.blinding.toString(),
    amounts: outUtxo.amounts,
    assets: outUtxo.assets,
    merkleProof,
    merkleTreeLeafIndex,
  };
  return createUtxo(hasher, account, inputs, outUtxo.isFillingUtxo);
}

export function createUtxo(
    lightWasm: LightWasm,
  account: Account,
  createUtxoInputs: CreateUtxoInputs,
  isFillingUtxo: boolean,
): UtxoNew {
  const {
    merkleTreeLeafIndex,
    utxoHash,
    blinding,
    amounts,
    assets,
    merkleProof,
    verifierAddress: verifierAddressInput,
    utxoDataHash: utxoDataHashInput,
  } = createUtxoInputs;

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

  if (!merkleTreeLeafIndexInternal) {
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
  const utxo: UtxoNew = {
    publicKey: account.keypair.publicKey.toString(),
    amounts,
    assets,
    assetsCircuit: assets.map((asset) =>
      hashAndTruncateToCircuit(asset.toBytes()).toString(),
    ),
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
  };
  return utxo;
}

export function getNullifier(lightWasm: LightWasm, inputs: NullifierInputs): string {
  return lightWasm.poseidonHashString([
    inputs.utxoHash,
    inputs.merkleTreeLeafIndex,
    inputs.signature,
  ]);
}
