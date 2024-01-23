import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN, BorshAccountsCoder, Idl } from "@coral-xyz/anchor";
import {
  Account,
  BN_0,
  COMPRESSED_UTXO_BYTES_LENGTH,
  createAccountObject,
  createOutUtxo,
  createUtxo,
  CreateUtxoErrorCode,
  CreateUtxoInputs,
  decryptOutUtxoInternal,
  encryptOutUtxoInternal,
  fetchAssetByIdLookUp,
  OutUtxo,
  STANDARD_COMPRESSION_PRIVATE_KEY,
  STANDARD_COMPRESSION_PUBLIC_KEY,
  UNCOMPRESSED_UTXO_BYTES_LENGTH,
  UtxoError,
  UtxoErrorCode,
  Utxo,
  hashAndTruncateToCircuit,
} from "../index";
import { LightWasm } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "../types";
// TODO: make utxoData depend on idl with generic type
export type ProgramOutUtxo = {
  outUtxo: OutUtxo;
  pspId: PublicKey;
  pspIdl: Idl;
  includeUtxoData: boolean;
  utxoName: string;
};

export function createProgramOutUtxo({
  encryptionPublicKey,
  amounts,
  assets,
  blinding,
  lightWasm,
  pspId,
  pspIdl,
  includeUtxoData = true,
  utxoData,
  utxoName,
  utxoDataHash,
}: {
  encryptionPublicKey?: Uint8Array;
  amounts: BN[];
  assets: PublicKey[];
  blinding?: BN;
  lightWasm: LightWasm;
  pspId: PublicKey;
  pspIdl: Idl;
  includeUtxoData?: boolean;
  utxoData: any;
  utxoName: string;
  utxoDataHash?: BN;
}): ProgramOutUtxo {
  checkUtxoData(utxoData, pspIdl, utxoName + "OutUtxo");

  const outUtxo = createOutUtxo({
    publicKey: hashAndTruncateToCircuit(pspId.toBytes()),
    encryptionPublicKey,
    amounts,
    assets,
    blinding,
    isFillingUtxo: false,
    lightWasm,
    utxoData,
    utxoDataHash,
  });

  const programOutUtxo: ProgramOutUtxo = {
    outUtxo,
    pspId,
    pspIdl,
    includeUtxoData,
    utxoName,
  };
  return programOutUtxo;
}

export const checkUtxoData = (utxoData: any, idl: any, circuitName: string) => {
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

  // skip keys which are not utxo data
  let enabled = false;
  inputKeys.forEach((key) => {
    if (enabled) {
      if (!utxoData[key])
        throw new Error(
          `Missing input --> ${key.toString()} in circuit ==> ${circuitName}`,
        );
    } else {
      if (key === "accountEncryptionPublicKey") {
        enabled = true;
      }
    }
  });
};

// TODO: make general for unlimited lengths
export const createUtxoDataHash = (utxoData: any, lightWasm: LightWasm): BN => {
  let hashArray: any[] = [];
  for (const attribute in utxoData) {
    hashArray.push(utxoData[attribute]);
  }
  hashArray = hashArray.flat();
  hashArray = hashArray.map((val) => val.toString());
  if (hashArray.length > 16) {
    throw new UtxoError(
      UtxoErrorCode.INVALID_APP_DATA,
      "constructor",
      "utxoData length exceeds 16",
    );
  }
  const utxoDataHash = new BN(
    lightWasm.poseidonHash(hashArray),
    undefined,
    "be",
  );
  return utxoDataHash;
};

// TODO: remove verifier index from encrypted utxo data
/**
 * @description Parses a utxo to bytes.
 * @returns {Uint8Array}
 */
export async function programOutUtxoToBytes(
  outUtxo: ProgramOutUtxo,
  assetLookupTable: string[],
  compressed: boolean = false,
): Promise<Uint8Array> {
  const serializeObject = {
    ...outUtxo,
    ...outUtxo.outUtxo,
    ...outUtxo.outUtxo.utxoData,
    dataHash: outUtxo.outUtxo.utxoDataHash,
    accountCompressionPublicKey: new BN(outUtxo.outUtxo.publicKey),
    accountEncryptionPublicKey: outUtxo.outUtxo.encryptionPublicKey
      ? outUtxo.outUtxo.encryptionPublicKey
      : new Uint8Array(32).fill(0),
    verifierAddressIndex: BN_0,
    splAssetIndex: new BN(
      assetLookupTable.findIndex(
        (asset) => asset === outUtxo.outUtxo.assets[1].toBase58(),
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
  const coder = new BorshAccountsCoder(outUtxo.pspIdl);
  const serializedData = await coder.encode(
    outUtxo.utxoName + "OutUtxo",
    serializeObject,
  );

  // Compressed serialization does not store the account since for an encrypted utxo
  // we assume that the user who is able to decrypt the utxo knows the corresponding account.
  return compressed
    ? serializedData.subarray(0, COMPRESSED_UTXO_BYTES_LENGTH)
    : serializedData;
}

// TODO: support multiple utxo names, to pick the correct one (we can probably match the name from the discriminator)
export function programOutUtxoFromBytes({
  bytes,
  account,
  assetLookupTable,
  compressed = false,
  lightWasm,
  pspId,
  pspIdl,
  utxoName,
  includeUtxoData,
}: {
  bytes: Buffer;
  account?: Account;
  assetLookupTable: string[];
  compressed?: boolean;
  lightWasm: LightWasm;
  pspId: PublicKey;
  pspIdl: Idl;
  utxoName: string;
  includeUtxoData?: boolean;
}): ProgramOutUtxo {
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
  const coder = new BorshAccountsCoder(pspIdl);
  const decodedUtxoData = coder.decode(utxoName + "OutUtxo", bytes);

  const assets = [
    SystemProgram.programId,
    fetchAssetByIdLookUp(decodedUtxoData.splAssetIndex, assetLookupTable),
  ];
  if (!pspIdl.accounts)
    throw new UtxoError(
      UtxoErrorCode.APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS,
      "fromBytes",
    );
  const utxoData = createAccountObject(
    decodedUtxoData,
    pspIdl.accounts,
    "utxoOutUtxoAppData", // TODO: make name flexible
  );

  const programUtxo = createProgramOutUtxo({
    encryptionPublicKey: new BN(decodedUtxoData.accountEncryptionPublicKey).eq(
      BN_0,
    )
      ? undefined
      : decodedUtxoData.accountEncryptionPublicKey,
    amounts: decodedUtxoData.amounts,
    assets,
    blinding: decodedUtxoData.blinding,
    lightWasm,
    pspId,
    pspIdl,
    includeUtxoData,
    utxoData,
    utxoName,
    utxoDataHash: decodedUtxoData.dataHash,
  });
  return programUtxo;
}

export function programOutUtxoFromString(
  string: string,
  assetLookupTable: string[],
  account: Account,
  lightWasm: LightWasm,
  compressed: boolean = false,
  pspId: PublicKey,
  pspIdl: Idl,
  utxoName: string,
): ProgramOutUtxo {
  return programOutUtxoFromBytes({
    bytes: bs58.decode(string),
    assetLookupTable,
    account,
    compressed,
    lightWasm,
    pspId,
    pspIdl,
    utxoName,
  });
}

/**
 * Converts the Utxo instance into a base58 encoded string.
 * @async
 * @returns {Promise<string>} A promise that resolves to the base58 encoded string representation of the Utxo.
 */
export async function programOutUtxoToString(
  utxo: ProgramOutUtxo,
  assetLookupTable: string[],
): Promise<string> {
  const bytes = await programOutUtxoToBytes(utxo, assetLookupTable);
  return bs58.encode(bytes);
}

export async function encryptProgramOutUtxo({
  utxo,
  lightWasm,
  account,
  merkleTreePdaPublicKey,
  compressed = false,
  assetLookupTable,
}: {
  utxo: ProgramOutUtxo;
  lightWasm: LightWasm;
  account?: Account;
  merkleTreePdaPublicKey?: PublicKey;
  compressed?: boolean;
  assetLookupTable: string[];
}): Promise<Uint8Array> {
  const bytes = await programOutUtxoToBytes(utxo, assetLookupTable, compressed);
  const utxoHash = new BN(utxo.outUtxo.utxoHash).toArrayLike(Buffer, "be", 32);

  const encryptedUtxo = await encryptOutUtxoInternal({
    bytes,
    utxoHash,
    lightWasm,
    account,
    merkleTreePdaPublicKey,
    compressed,
    publicKey: utxo.outUtxo.publicKey,
    encryptionPublicKey: utxo.outUtxo.encryptionPublicKey,
  });
  return encryptedUtxo;
}

export async function decryptProgramOutUtxo({
  lightWasm,
  encBytes,
  account,
  merkleTreePdaPublicKey,
  aes,
  utxoHash,
  compressed = false,
  assetLookupTable,
  pspId,
  pspIdl,
  utxoName,
}: {
  lightWasm: LightWasm;
  encBytes: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  aes: boolean;
  utxoHash: Uint8Array;
  compressed?: boolean;
  assetLookupTable: string[];
  pspId: PublicKey;
  pspIdl: Idl;
  utxoName: string;
}): Promise<Result<ProgramOutUtxo | null, UtxoError>> {
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
    programOutUtxoFromBytes({
      lightWasm,
      bytes,
      account,
      assetLookupTable,
      compressed,
      pspId,
      pspIdl,
      utxoName,
    }),
  );
}

export type ProgramUtxo = {
  utxo: Utxo;
  pspId: PublicKey;
  pspIdl: Idl;
  includeUtxoData: boolean;
};

export function createProgramUtxo({
  createUtxoInputs,
  account,
  lightWasm,
  pspId,
  pspIdl,
  includeUtxoData = true,
  utxoData,
  utxoName,
}: {
  createUtxoInputs: CreateUtxoInputs;
  lightWasm: LightWasm;
  pspId: PublicKey;
  pspIdl: Idl;
  includeUtxoData?: boolean;
  utxoData: any;
  utxoName: string;
  account: Account;
}): ProgramUtxo {
  const utxoDataInternal = utxoData;
  checkUtxoData(utxoDataInternal, pspIdl, utxoName + "OutUtxo");
  const dataHash = createUtxoDataHash(utxoDataInternal, lightWasm);
  createUtxoInputs["utxoDataHash"] = dataHash.toString();
  createUtxoInputs["owner"] = pspId;
  createUtxoInputs["utxoData"] = utxoDataInternal;
  createUtxoInputs["utxoName"] = utxoName;
  const utxo = createUtxo(lightWasm, account, createUtxoInputs, false);
  const programOutUtxo: ProgramUtxo = {
    utxo,
    pspId,
    pspIdl,
    includeUtxoData,
  };
  return programOutUtxo;
}

export interface DecryptProgramUtxoParams {
  encBytes: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  aes: boolean;
  utxoHash: Uint8Array;
  lightWasm: LightWasm;
  compressed?: boolean;
  merkleProof: string[];
  merkleTreeLeafIndex: number;
  assetLookupTable: string[];
  pspId: PublicKey;
  pspIdl: Idl;
  utxoName: string;
}

export async function decryptProgramUtxo({
  encBytes,
  account,
  merkleTreePdaPublicKey,
  aes,
  utxoHash,
  lightWasm,
  compressed = false,
  merkleProof,
  merkleTreeLeafIndex,
  assetLookupTable,
  pspId,
  pspIdl,
  utxoName,
}: DecryptProgramUtxoParams): Promise<Result<ProgramUtxo | null, UtxoError>> {
  const decryptedProgramOutUtxo = await decryptProgramOutUtxo({
    encBytes,
    account,
    merkleTreePdaPublicKey,
    aes,
    utxoHash,
    lightWasm,
    compressed,
    assetLookupTable,
    pspId,
    pspIdl,
    utxoName,
  });
  if (!decryptedProgramOutUtxo.value) {
    return decryptedProgramOutUtxo as Result<ProgramUtxo | null, UtxoError>;
  }
  if (
    decryptedProgramOutUtxo.value.outUtxo.publicKey ===
    STANDARD_COMPRESSION_PUBLIC_KEY.toString()
  ) {
    const bs58Standard = bs58.encode(
      STANDARD_COMPRESSION_PRIVATE_KEY.toArray(),
    );
    const bs5832 = bs58.encode(new Uint8Array(32).fill(1));
    account = Account.fromPrivkey(lightWasm, bs58Standard, bs5832, bs5832);
  }

  return Result.Ok(
    programOutUtxoToProgramUtxo(
      decryptedProgramOutUtxo.value,
      merkleProof,
      merkleTreeLeafIndex,
      lightWasm,
      account,
    ),
  );
}

export function programOutUtxoToProgramUtxo(
  programOutUtxo: ProgramOutUtxo,
  merkleProof: string[],
  merkleTreeLeafIndex: number,
  lightWasm: LightWasm,
  account: Account,
): ProgramUtxo {
  const inputs: CreateUtxoInputs = {
    utxoHash: programOutUtxo.outUtxo.utxoHash,
    blinding: programOutUtxo.outUtxo.blinding.toString(),
    amounts: programOutUtxo.outUtxo.amounts,
    assets: programOutUtxo.outUtxo.assets,
    merkleProof,
    merkleTreeLeafIndex,
    owner: programOutUtxo.pspId,
  };
  return createProgramUtxo({
    createUtxoInputs: inputs,
    account,
    lightWasm,
    pspId: programOutUtxo.pspId,
    pspIdl: programOutUtxo.pspIdl,
    includeUtxoData: programOutUtxo.includeUtxoData,
    utxoData: programOutUtxo.outUtxo.utxoData,
    utxoName: programOutUtxo.utxoName,
  });
}
