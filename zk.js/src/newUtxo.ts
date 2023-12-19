import nacl from "tweetnacl";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN, BorshAccountsCoder, Idl } from "@coral-xyz/anchor";
import {
  Account,
  BN_0,
  BN_1,
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
import { Hasher } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "./types";

const randomBN = (nbytes = 30) => {
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
  isFillingUtxo = false,
  hasher,
}: {
  publicKey: BN;
  encryptionPublicKey?: Uint8Array;
  amounts: BN[];
  assets: PublicKey[];
  blinding?: BN;
  isFillingUtxo?: boolean;
  hasher: Hasher;
}): OutUtxo {
  const poolType = BN_0;
  const transactionVersion = BN_0;
  const utxoDataHash = "0";
  const verifierAddressCircuit = "0";
  const utxoHashInputs: UtxoHashInputs = {
    publicKey: publicKey.toString(),
    amounts: amounts.map((amount) => amount.toString()),
    assetsCircuit: assets.map((asset) =>
      hashAndTruncateToCircuit(asset.toBytes()).toString(),
    ),
    blinding: blinding.toString(),
    poolType: poolType.toString(),
    transactionVersion: transactionVersion.toString(),
    utxoDataHash,
    verifierAddressCircuit,
  };
  const utxoHash = getUtxoHash(hasher, utxoHashInputs);
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
    utxoDataHash: BN_0,
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
  hasher: Hasher,
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
  const amountHash = hasher.poseidonHashString(amounts);
  const assetHash = hasher.poseidonHashString(
    assetsCircuit.map((x) => x.toString()),
  );

  if (!publicKey) {
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
  return hasher.poseidonHashString([
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
  hasher,
}: {
  bytes: Buffer;
  account?: Account;
  assetLookupTable: string[];
  compressed?: boolean;
  hasher: Hasher;
}): OutUtxo {
  const poolType = "0";
  const transactionVersion = "0";
  const verifierAddress = SystemProgram.programId;
  const verifierAddressCircuit = "0";
  const utxoDataHash = "0";
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
    ? account?.pubkey
    : decodedUtxoData.accountShieldedPublicKey;
  const utxoHashInputs: UtxoHashInputs = {
    publicKey: publicKey.toString(),
    amounts: decodedUtxoData.amounts.map((amount: BN) => amount.toString()),
    assetsCircuit: assets.map((asset) =>
      hashAndTruncateToCircuit(asset.toBytes()).toString(),
    ),
    blinding: decodedUtxoData.blinding.toString(),
    poolType: poolType.toString(),
    transactionVersion: transactionVersion.toString(),
    utxoDataHash,
    verifierAddressCircuit,
  };

  const utxoHash = getUtxoHash(hasher, utxoHashInputs);

  const outUtxo: OutUtxo = {
    publicKey,
    encryptionPublicKey: new BN(decodedUtxoData.accountEncryptionPublicKey).eq(
      BN_0,
    )
      ? undefined
      : new Uint8Array(decodedUtxoData.accountEncryptionPublicKey),
    amounts: decodedUtxoData.amounts,
    assets,
    assetsCircuit: assets.map((asset) =>
      hashAndTruncateToCircuit(asset.toBytes()).toString(),
    ),
    blinding: decodedUtxoData.blinding,
    poolType,
    utxoHash,
    transactionVersion,
    verifierAddress,
    verifierAddressCircuit,
    utxoDataHash: BN_0,
    isFillingUtxo: decodedUtxoData.isFillingUtxo,
  };
  return outUtxo;
}

export function outUtxoFromString(
  string: string,
  assetLookupTable: string[],
  account: Account,
  hasher: Hasher,
  compressed: boolean = false,
): OutUtxo {
  return outUtxoFromBytes({
    bytes: bs58.decode(string),
    assetLookupTable,
    account,
    compressed,
    hasher,
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
  hasher,
  account,
  merkleTreePdaPublicKey,
  compressed = false,
  assetLookupTable,
}: {
  utxo: OutUtxo;
  hasher: Hasher;
  account?: Account;
  merkleTreePdaPublicKey?: PublicKey;
  compressed?: boolean;
  assetLookupTable: string[];
}): Promise<Uint8Array> {
  const bytes = await outUtxoToBytes(utxo, assetLookupTable, compressed);
  const utxoHash = new BN(utxo.utxoHash).toArrayLike(Buffer, "be", 32);

  if (utxo.encryptionPublicKey) {
    const ciphertext = Account.encryptNaclUtxo(
      utxo.encryptionPublicKey,
      bytes,
      utxoHash,
    );
    // TODO: add option to use random or dedicated prefix for asynmetrically encrypted utxos which are sent to another party
    const prefix = !utxo.isFillingUtxo
      ? new BN(utxo.publicKey).toArray("be", 32).slice(0, 4)
      : randomPrefixBytes();
    return Uint8Array.from([...prefix, ...ciphertext]);
  } else if (account) {
    if (!merkleTreePdaPublicKey)
      throw new UtxoError(
        UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED,
        "encrypt",
        "For aes encryption the merkle tree pda publickey is necessary to derive the viewingkey",
      );

    const ciphertext = await account.encryptAesUtxo(
      bytes,
      merkleTreePdaPublicKey,
      utxoHash,
      hasher,
    );

    // If utxo is filling utxo we don't want to decrypt it in the future, so we use a random prefix
    // we still want to encrypt it properly to be able to decrypt it if necessary as a safeguard.
    const prefix = !utxo.isFillingUtxo
      ? account.generateLatestUtxoPrefixHash(
          merkleTreePdaPublicKey,
          UTXO_PREFIX_LENGTH,
          hasher,
        )
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
  hasher,
  encBytes,
  account,
  merkleTreePdaPublicKey,
  aes,
  utxoHash,
  compressed = false,
  assetLookupTable,
}: {
  hasher: Hasher;
  encBytes: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  aes: boolean;
  utxoHash: Uint8Array;
  compressed?: boolean;
  assetLookupTable: string[];
}): Promise<Result<OutUtxo | null, UtxoError>> {
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
        utxoHash,
        hasher,
      )
    : await account.decryptNaclUtxo(encBytes, utxoHash);

  if (!cleartext || cleartext.error || !cleartext.value) return Result.Ok(null);
  const bytes = Buffer.from(cleartext.value || cleartext);

  return Result.Ok(
    outUtxoFromBytes({
      hasher,
      bytes,
      account,
      assetLookupTable,
      compressed,
    }),
  );
}

export type programOutUtxo = {
  outUtxo: OutUtxo;
  pspId: PublicKey;
  pspIdl: Idl;
  includeAppData: boolean;
};

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
  merkleTreeIndex: number;
  merkleProof: string[];
  utxoDataHash: string;
};

export type NullifierInputs = {
  signature: string;
  utxoHash: string;
  merkleTreeIndex: string;
};

export type CreateUtxoInputs = {
  utxoHash: string;
  merkleTreeIndex?: number;
  verifierAddress?: PublicKey;
  blinding: string;
  amounts: BN[];
  assets: PublicKey[];
  merkleProof: string[];
};

export async function decryptUtxo(
  encBytes: Uint8Array,
  account: Account,
  merkleTreePdaPublicKey: PublicKey,
  aes: boolean,
  utxoHash: Uint8Array,
  hasher: Hasher,
  compressed: boolean = true,
  merkleProof: string[],
  merkleTreeIndex: number,
  assetLookupTable: string[],
): Promise<Result<UtxoNew | null, UtxoError>> {
  const decryptedOutUtxo = await decryptOutUtxo({
    encBytes,
    account,
    merkleTreePdaPublicKey,
    aes,
    utxoHash,
    hasher,
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
      merkleTreeIndex,
      hasher,
      account,
    ),
  );
}

export function outUtxoToUtxo(
  outUtxo: OutUtxo,
  merkleProof: string[],
  merkleTreeIndex: number,
  hasher: Hasher,
  account: Account,
): UtxoNew {
  const inputs: CreateUtxoInputs = {
    utxoHash: outUtxo.utxoHash,
    blinding: outUtxo.blinding.toString(),
    amounts: outUtxo.amounts,
    assets: outUtxo.assets,
    merkleProof,
    merkleTreeIndex,
  };
  return createUtxo(hasher, account, inputs, outUtxo.isFillingUtxo);
}

export function createUtxo(
  hasher: Hasher,
  account: Account,
  createUtxoInputs: CreateUtxoInputs,
  isFillingUtxo: boolean,
): UtxoNew {
  const { merkleTreeIndex, utxoHash, blinding, amounts, assets, merkleProof } =
    createUtxoInputs;

  const poolType = "0";
  const transactionVersion = "0";
  const merkleTreeIndexInternal = isFillingUtxo ? 0 : merkleTreeIndex;
  const verifierAddress = SystemProgram.programId;
  const verifierAddressCircuit = "0";
  const utxoDataHash = "0";

  if (!merkleTreeIndexInternal) {
    throw new UtxoError(
      CreateUtxoErrorCode.MERKLE_TREE_INDEX_UNDEFINED,
      "createUtxo",
      "Merkle tree index is undefined",
    );
  }

  const signature = account
    .sign(hasher, utxoHash, merkleTreeIndexInternal)
    .toString();

  const nullifierInputs: NullifierInputs = {
    signature,
    utxoHash,
    merkleTreeIndex: merkleTreeIndexInternal.toString(),
  };
  const nullifier = getNullifier(hasher, nullifierInputs);
  const utxo: UtxoNew = {
    publicKey: account.pubkey.toString(),
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
    merkleTreeIndex: merkleTreeIndexInternal,
    merkleProof,
    utxoDataHash,
  };
  return utxo;
}

export function getNullifier(hasher: Hasher, inputs: NullifierInputs): string {
  return hasher.poseidonHashString([
    inputs.utxoHash,
    inputs.merkleTreeIndex,
    inputs.signature,
  ]);
}

// export async function utxoToBytes(
//   utxo: UtxoNew,
//   assetLookupTable: string[],
//   compressed: boolean = false,
// ) {
//   let serializeObject = {
//     ...utxo,
//     accountShieldedPublicKey: utxo.publicKey,
//     accountEncryptionPublicKey: new Uint8Array(32).fill(0),
//     verifierAddressIndex: 0,
//     splAssetIndex: assetLookupTable.findIndex(
//       (asset) => asset === utxo.assets[1].toBase58(),
//     ),
//   };

//   const coder = new BorshAccountsCoder(IDL_LIGHT_PSP2IN2OUT);
//   const serializedData = await coder.encode("utxo", serializeObject);
//   // Compressed serialization does not store the account since for an encrypted utxo
//   // we assume that the user who is able to decrypt the utxo knows the corresponding account.
//   return compressed
//     ? serializedData.subarray(0, COMPRESSED_UTXO_BYTES_LENGTH)
//     : serializedData;
// }

// export function utxoFromBytes({
//   hasher,
//   bytes,
//   account,
//   assetLookupTable,
//   compressed = false,
//   isFillingUtxo = false,
// }: {
//   hasher: Hasher;
//   bytes: Buffer;
//   account: Account;
//   assetLookupTable: string[];
//   compressed: boolean;
//   isFillingUtxo: boolean;
// }): UtxoNew {
//   const poolType = "0";
//   const transactionVersion = "0";
//   const verifierAddress = SystemProgram.programId;
//   const verifierAddressCircuit = "0";
//   const utxoDataHash = "0";

//   // if it is compressed adds 64 0 bytes padding and requires account
//   if (compressed) {
//     const tmp: Uint8Array = Uint8Array.from([...Array.from(bytes)]);
//     bytes = Buffer.from([
//       ...tmp,
//       ...new Uint8Array(
//         UNCOMPRESSED_UTXO_BYTES_LENGTH - COMPRESSED_UTXO_BYTES_LENGTH,
//       ).fill(0),
//     ]);
//     if (!account)
//       throw new UtxoError(
//         CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
//         "fromBytes",
//         "For deserializing a compressed utxo an account is required.",
//       );
//   }

//   // TODO: should I check whether an account is passed or not?
//   const coder = new BorshAccountsCoder(IDL_LIGHT_PSP2IN2OUT);
//   const decodedUtxoData = coder.decode("utxo", bytes);

//   const assets = [
//     SystemProgram.programId,
//     fetchAssetByIdLookUp(decodedUtxoData.splAssetIndex, assetLookupTable),
//   ];
//   const utxoHashInputs: UtxoHashInputs = {
//     publicKey: decodedUtxoData.accountShieldedPublicKey,
//     amounts: decodedUtxoData.amounts.map((amount: BN) => amount.toString()),
//     assetsCircuit: assets.map((asset) =>
//       hashAndTruncateToCircuit(asset.toBytes()).toString(),
//     ),
//     blinding: decodedUtxoData.blinding,
//     poolType,
//     transactionVersion,
//     utxoDataHash,
//     verifierAddressCircuit,
//   };
//   const utxoHash = getUtxoHash(hasher, utxoHashInputs);
//   const signature = account
//     .sign(hasher, utxoHash, decodedUtxoData.merkleTreeIndex)
//     .toString();
//   const nullifierInputs: NullifierInputs = {
//     signature,
//     utxoHash,
//     merkleTreeIndex: decodedUtxoData.merkleTreeIndex.toString(),
//   };
//   const nullifier = getNullifier(hasher, nullifierInputs);
//   const utxo: UtxoNew = {
//     publicKey: decodedUtxoData.accountShieldedPublicKey,
//     amounts: decodedUtxoData.amounts,
//     assets,
//     assetsCircuit: assets.map((asset) =>
//       hashAndTruncateToCircuit(asset.toBytes()).toString(),
//     ),
//     blinding: decodedUtxoData.blinding,
//     poolType: decodedUtxoData.poolType,
//     utxoHash,
//     transactionVersion,
//     verifierAddress,
//     verifierAddressCircuit,
//     isFillingUtxo,
//     nullifier,
//     merkleTreeIndex: decodedUtxoData.merkleTreeIndex,
//     merkleProof: decodedUtxoData.merkleProof,
//     utxoDataHash,
//   };
//   return utxo;
// }

// export function utxoFromString(
//   string: string,
//   hasher: Hasher,
//   assetLookupTable: string[],
//   account: Account,
//   compressed: boolean = false,
//   isFillingUtxo: boolean = false,
// ): UtxoNew {
//   return utxoFromBytes({
//     bytes: bs58.decode(string),
//     hasher,
//     assetLookupTable,
//     account,
//     compressed,
//     isFillingUtxo,
//   });
// }

// /**
//  * Converts the Utxo instance into a base58 encoded string.
//  * @async
//  * @returns {Promise<string>} A promise that resolves to the base58 encoded string representation of the Utxo.
//  */
// export async function toString(
//   utxo: UtxoNew,
//   assetLookupTable: string[],
// ): Promise<string> {
//   const bytes = await utxoToBytes(utxo, assetLookupTable);
//   return bs58.encode(bytes);
// }

// export function getAppInUtxoIndices(appUtxos: ProgramUtxo[]) {
//   const isAppInUtxo: BN[][] = [];
//   for (const i in appUtxos) {
//     const array = new Array(4).fill(new BN(0));
//     if (appUtxos[i].appData) {
//       array[i] = new BN(1);
//       isAppInUtxo.push(array);
//     }
//   }
//   return isAppInUtxo;
// }
