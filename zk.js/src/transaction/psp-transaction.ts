import { AnchorProvider, BN, Idl, Program, utils } from "@coral-xyz/anchor";
import { upperCamelCase, camelCase } from "case-anything";
import { LightWasm } from "@lightprotocol/account.rs";
import { getIndices3D } from "@lightprotocol/circuit-lib.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { sha256 } from "@noble/hashes/sha256";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import nacl from "tweetnacl";
import {
  AUTHORITY,
  BN_0,
  BN_1,
  FIELD_SIZE,
  MERKLE_TREE_HEIGHT,
  N_ASSET_PUBKEYS,
  STANDARD_COMPRESSION_PRIVATE_KEY,
  STANDARD_COMPRESSION_PUBLIC_KEY,
  SYSTEM_PROGRAM_IDLS,
  lightPsp2in2outStorageId,
  merkleTreeProgramId,
} from "../constants";
import { Account } from "../account";
import {
  CreateUtxoErrorCode,
  RpcErrorCode,
  TransactionError,
  TransactionErrorCode,
  TransactionParametersError,
  TransactionParametersErrorCode,
  UserErrorCode,
} from "../errors";
import {truncateToCircuit, hashAndTruncateToCircuit, stringifyAssetsToCircuitInput} from "../utils/hash-utils";
import { Action } from "../types";
import { MerkleTreeConfig } from "../merkle-tree";
import { TokenData } from "../types";
import { MINT } from "../test-utils/constants-system-verifier";
import {
  BN254,
  createFillingOutUtxo,
  createFillingUtxo,
  createOutUtxos,
  selectInUtxos,
  Utxo,
  OutUtxo,
  encryptOutUtxo
} from "../utxo";
import {
  PlaceHolderTData,
  ProgramOutUtxo,
  ProgramUtxo,
} from "../utxo/program-utxo-types";
import { AppUtxoConfig } from "../types";
import { Rpc } from "../rpc";
import { Provider } from "../provider";
import {getAssetPubkeys, getVerifierProgramId} from "transaction/psp-util";

export type SystemProofInputs = {
  publicInUtxoDataHash: (string | BN)[];
  publicInUtxoHash: string[];
  transactionHashIsPublic: string;
  isMetaHashUtxo: BN[];
  isAddressUtxo: BN[];
  isOutProgramUtxo: BN[];
  isInProgramUtxo: BN[];
  inOwner: (string | BN)[];
  inAddress: BN[];
  isInAddress: BN[];
  isNewAddress: string[];
  publicNewAddress: string[];
  publicStateRoot: string[];
  publicNullifierRoot: string[];
  nullifierLeafIndex: string[];
  nullifierMerkleProof: string[][];
  publicNullifier: BN[];
  publicAmountSpl: string;
  publicAmountSol: string;
  publicMintPublicKey: string;
  leafIndex: number[];
  merkleProof: string[][];
  privatePublicDataHash: string;
  publicDataHash: string;
  publicOutUtxoHash: BN[];
  inAmount: BN[][];
  inBlinding: BN[];
  assetPublicKeys: string[];
  outAmount: BN[][];
  outBlinding: BN[];
  outOwner: (string | BN)[];
  inIndices: string[][][];
  outIndices: string[][][];
  inDataHash: BN[];
  outDataHash: BN[];
  metaHash: BN[];
  address: BN[];
};

export type DecompressTransactionInput = {
  mint?: PublicKey;
  message?: Buffer;
  merkleTreeSetPubkey: PublicKey;
  recipientSpl?: PublicKey;
  recipientSol?: PublicKey;
  inputUtxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outputUtxos?: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[];
  rpcPublicKey: PublicKey;
  lightWasm: LightWasm;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  rpcFee: BN;
  ataCreationFee: boolean;
  assetLookUpTable?: string[];
};

export type CompressTransactionInput = {
  mint?: PublicKey;
  message?: Buffer;
  merkleTreeSetPubkey: PublicKey;
  senderSpl?: PublicKey;
  inputUtxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outputUtxos?: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[];
  signer: PublicKey;
  lightWasm: LightWasm;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  assetLookUpTable?: string[];
};

export type TransactionInput = {
  message?: Buffer;
  merkleTreeSetPubkey: PublicKey;
  inputUtxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outputUtxos?: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[];
  rpcPublicKey: PublicKey;
  lightWasm: LightWasm;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  rpcFee: BN;
  assetLookUpTable?: string[];
};

//TODO: make part of transaction parameters(Transaction)
export type PspTransactionInput = {
  proofInputs: any;
  verifierIdl: Idl;
  circuitName: string;
  path: string;
  checkedInUtxos?: {
    type: string;
    utxo: Utxo | ProgramUtxo<PlaceHolderTData>;
  }[];
  checkedOutUtxos?: {
    type: string;
    utxo: OutUtxo | ProgramOutUtxo<PlaceHolderTData>;
  }[];
  inUtxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outUtxos?: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[];
  accounts?: any;
};

/// TODO: reinstante proper type. make PspProofInputs type generic to IDL
type compiledProofInputs = any;
// {
// systemProofInputs: any;
// pspProofInputs: any;
// };

export type VerifierConfig = {
  in: number;
  out: number;
};

export type DecompressAccounts = {
  recipientSol: PublicKey;
  recipientSpl: PublicKey;
  rpcPublicKey: PublicKey;
};

// TODO: make all inputs part of integrity hash
export type TransactionAccounts = {
  senderSpl: PublicKey;
  senderSol: PublicKey;
  recipientSpl: PublicKey;
  recipientSol: PublicKey;
  rpcPublicKey: PublicKey;
  merkleTreeSet: PublicKey;
  systemPspId: PublicKey;
  pspId?: PublicKey;
};

export type PublicTransactionVariables = {
  accounts: TransactionAccounts;
  publicAmountSpl: BN;
  publicAmountSol: BN;
  rpcFee: BN;
  ataCreationFee: boolean;
  encryptedUtxos: Uint8Array;
  publicMintPubkey: string;
  message?: Buffer;
  transactionHash: string;
  // TODO: rename to publicDataHash
  txIntegrityHash: BN;
};

export type PrivateTransactionVariables = {
  inputUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outputUtxos: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[];
  assetPubkeys: PublicKey[];
  assetPubkeysCircuit: string[];
};

export type Transaction = {
  private: PrivateTransactionVariables;
  public: PublicTransactionVariables;
};

export type CompressTransaction = Transaction & {
  action: Action;
};

export type DecompressTransaction = Transaction & {
  action: Action;
};

/** Sets undefined circuit inputs to zero. Helper for use in PSP clients */
export const setUndefinedPspCircuitInputsToZero = (
  proofInputs: any,
  idl: Idl,
  circuitName: string,
) => {
  const circuitIdlObject = idl.accounts!.find(
    (account) =>
      account.name.toUpperCase() ===
      `zK${circuitName}ProofInputs`.toUpperCase(),
  );

  if (!circuitIdlObject) {
    throw new TransactionError(
      TransactionErrorCode.CIRCUIT_NOT_FOUND,
      "setUndefinedPspCircuitInputsToZero",
      `${`zK${circuitName}ProofInputs`} does not exist in anchor idl`,
    );
  }

  const fieldNames = circuitIdlObject.type.fields;

  const inputsObject: { [key: string]: any } = {};
  const lastSytemField = "assetPublicKeys";
  let foundLastSystemField = false;
  fieldNames.forEach(({ name, type }) => {
    inputsObject[name] = proofInputs[name];

    if (!inputsObject[name] && foundLastSystemField) {
      // @ts-ignore
      if (type["array"] && type["array"][1].toString() !== "32") {
        // @ts-ignore
        inputsObject[name] = new Array(type["array"][1]).fill(BN_0);
      } else {
        inputsObject[name] = BN_0;
      }
    }
    if (name === lastSytemField) {
      foundLastSystemField = true;
    }
    if (inputsObject[name] === undefined) {
      delete inputsObject[name];
    }
  });
  return { ...proofInputs, ...inputsObject };
};

// how do I best steamline the transaction generation process for psps?
// 1. define circuit specific proof inputs which are not part of the utxos utxoData - check whether inputs which are not utxos pausible
// 2. define in and out utxos
// 3.1 filter utxos that go into selection for input utxos -> select miising utxos
// 3.2 create output utxos
// 3.3 create transaction parameters
// 4. compile app parameters
// 5. compile and prove etc.
export const createUtxoIndices = (
  utxos:
    | (Utxo | ProgramUtxo<PlaceHolderTData>)[]
    | (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[],
  utxoHash: BN254,
): BN[] => {
  const isAppInUtxo = new Array(4).fill(BN_0);
  for (const i in utxos) {
    if (utxos[i].hash.eq(utxoHash)) {
      isAppInUtxo[i] = BN_1;
    }
  }
  return isAppInUtxo;
};

// TODO: resolve out utxo vs program utxo type use
export const createPspProofInputs = (
  pspTransaction: PspTransactionInput,
  inputUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[],
  outputUtxos: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[],
  publicTransactionHash: string,
): any => {
  const inUtxosInputs = {};
  pspTransaction.checkedInUtxos?.forEach(({ type, utxo: programUtxo }) => {
    const utxo = programUtxo;

    if ("data" in programUtxo) {
      for (const field in programUtxo.data) {
        inUtxosInputs[`${type}${upperCamelCase(field)}`] =
          programUtxo.data[field];
      }
    }
    const splAssetCircuitInput = stringifyAssetsToCircuitInput(utxo.assets)[1];
    const isAppUtxo = createUtxoIndices(inputUtxos, utxo.hash);

    inUtxosInputs[`isInProgramUtxo${upperCamelCase(type)}`] = isAppUtxo;
    inUtxosInputs[`${camelCase(type)}Blinding`] = utxo.blinding;
    inUtxosInputs[`${camelCase(type)}AmountSol`] = utxo.amounts[0];
    inUtxosInputs[`${camelCase(type)}AmountSpl`] =
      utxo.amounts.length === 2 ? utxo.amounts[1] : BN_0;
    inUtxosInputs[`${camelCase(type)}AssetSpl`] = splAssetCircuitInput;
    inUtxosInputs[`${camelCase(type)}Owner`] =
      "dataHash" in utxo
        ? hashAndTruncateToCircuit(utxo.owner.toBytes())
        : utxo.owner;
    inUtxosInputs[`${camelCase(type)}Type`] = utxo.poolType;
    inUtxosInputs[`${camelCase(type)}Version`] = utxo.version;
    inUtxosInputs[`${camelCase(type)}MetaHash`] = utxo.metaHash || BN_0;
    inUtxosInputs[`${camelCase(type)}Address`] = utxo.address || BN_0;
    // utxo data hash is calculated in the circuit
  });

  // TODO: think about how to make outUtxos and programOutUtxos consistent, do I need utxoData in outUtxos?
  const outUtxosInputs = {};
  pspTransaction.checkedOutUtxos?.forEach(({ type, utxo: programUtxo }) => {
    const utxo = programUtxo;
    if ("data" in programUtxo) {
      for (const field in programUtxo.data) {
        outUtxosInputs[`${type}${upperCamelCase(field)}`] =
          programUtxo.data[field];
      }
    }
    const splAssetCircuitInput = stringifyAssetsToCircuitInput(utxo.assets)[1];

    const isAppUtxoIndices = createUtxoIndices(outputUtxos, utxo.hash);
    outUtxosInputs[`isOutProgramUtxo${upperCamelCase(type)}`] =
      isAppUtxoIndices;
    outUtxosInputs[`${camelCase(type)}Blinding`] = utxo.blinding;
    outUtxosInputs[`${camelCase(type)}AmountSol`] = utxo.amounts[0];
    outUtxosInputs[`${camelCase(type)}AmountSpl`] =
      utxo.amounts.length === 2 ? utxo.amounts[1] : BN_0;
    outUtxosInputs[`${camelCase(type)}AssetSpl`] = splAssetCircuitInput;
    outUtxosInputs[`${camelCase(type)}Owner`] =
      "dataHash" in utxo
        ? hashAndTruncateToCircuit(utxo.owner.toBytes())
        : utxo.owner;
    outUtxosInputs[`${camelCase(type)}Type`] = utxo.poolType;
    outUtxosInputs[`${camelCase(type)}Version`] = BN_0;
    outUtxosInputs[`${camelCase(type)}MetaHash`] = utxo.metaHash || BN_0;
    outUtxosInputs[`${camelCase(type)}Address`] = utxo.address || BN_0;
  });

  const publicProgramId = hashAndTruncateToCircuit(
    getVerifierProgramId(pspTransaction.verifierIdl).toBuffer(),
  );

  const compiledProofInputs = {
    ...pspTransaction.proofInputs,
    inOwner: inputUtxos?.map((utxo) =>
      "dataHash" in utxo
        ? hashAndTruncateToCircuit(utxo.owner.toBytes())
        : utxo.owner,
    ),
    publicTransactionHash,
    publicProgramId,
    ...inUtxosInputs,
    ...outUtxosInputs,
    inAsset: inputUtxos?.map((utxo) => {
      const assetCircuitInput = stringifyAssetsToCircuitInput(utxo.assets);
      return [assetCircuitInput[0], assetCircuitInput[1]];
    }),
    outAsset: outputUtxos?.map((utxo) => {
      const assetCircuitInput = stringifyAssetsToCircuitInput(utxo.assets);
      return [assetCircuitInput[0], assetCircuitInput[1]];
    }),
    inAddress: inputUtxos?.map((utxo) => utxo.address || BN_0),
    outAddress: outputUtxos?.map((utxo) => utxo.address || BN_0),
    inMetaHash: inputUtxos?.map((utxo) => utxo.metaHash || BN_0),
    outMetaHash: outputUtxos?.map((utxo) => utxo.metaHash || BN_0),
  };
  return compiledProofInputs;
};

// TODO: add check that length input utxos is as expected by the verifier idl
/// TODO: since inputUtxos are passed separately, they wont be converted to circuit inputs, that's a footgun we should resolve
export async function getSystemProof({
  account,
  inputUtxos,
  systemProofInputs,
  verifierIdl,
  getProver,
  wasmTester,
}: {
  account: Account;
  verifierIdl: Idl;
  inputUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  systemProofInputs: SystemProofInputs;
  getProver?: any;
  wasmTester?: any;
}) {
  const path = require("path");
  const firstPath = path.resolve(__dirname, "../../build-circuits/");

  return account.getProofInternal({
    firstPath,
    verifierIdl,
    proofInput: systemProofInputs,
    addPrivateKey: true,
    inputUtxos,
    getProver,
    wasmTester,
  });
}

/**
 * Prepares proof inputs.
 */
export function createSystemProofInputs({
  transaction,
  root,
  account,
  lightWasm,
}: {
  transaction: Transaction;
  root: string;
  account: Account;
  lightWasm: LightWasm;
}): SystemProofInputs {
  if (!transaction.public.txIntegrityHash)
    throw new TransactionError(
      TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
      "compile",
    );

  const publicNullifier = transaction.private.inputUtxos.map((x) => {
    let _account = account;
    if (!("data" in x) && new BN(x.owner).eq(STANDARD_COMPRESSION_PUBLIC_KEY)) {
      _account = Account.fromPrivkey(
        lightWasm,
        bs58.encode(STANDARD_COMPRESSION_PRIVATE_KEY.toArray("be", 32)),
        bs58.encode(STANDARD_COMPRESSION_PRIVATE_KEY.toArray("be", 32)),
        bs58.encode(STANDARD_COMPRESSION_PRIVATE_KEY.toArray("be", 32)),
      );
    }
    return x.nullifier;
  });

  const publicProgramCircuitInputs = {
    publicInUtxoDataHash: transaction.private.inputUtxos?.map((x) =>
      "dataHash" in x ? x.dataHash.toString() : BN_0.toString(),
    ),
    publicInUtxoHash: transaction.private.inputUtxos.map((x) =>
      x.hash.toString(),
    ),
    transactionHashIsPublic: "0",
    isMetaHashUtxo: transaction.private.inputUtxos.map((utxo) => {
      if (utxo.metaHash) return new BN(1);
      else return new BN(0);
    }),
    isAddressUtxo: transaction.private.outputUtxos.map((utxo) => {
      if (utxo.address) return new BN(1);
      else return new BN(0);
    }),
    isOutProgramUtxo: transaction.private.outputUtxos.map((utxo) => {
      if ("dataHash" in utxo) return new BN(1);
      else return new BN(0);
    }),
  };

  const programCircuitInputs = {
    isInProgramUtxo: transaction.private.inputUtxos.map((utxo) => {
      if ("dataHash" in utxo) return new BN(1);
      else return new BN(0);
    }),
    inOwner: transaction.private.inputUtxos.map((utxo) =>
      "dataHash" in utxo
        ? hashAndTruncateToCircuit(utxo.owner.toBytes())
        : utxo.owner,
    ),
    inAddress: transaction.private.inputUtxos.map((utxo) => {
      if (utxo.address) return utxo.address;
      else return new BN(0);
    }),
    isInAddress: transaction.private.inputUtxos.map((utxo) => {
      if (utxo.address) return new BN(1);
      else return new BN(0);
    }),
    isNewAddress: new Array(transaction.private.outputUtxos.length).fill("0"),
    publicNewAddress: new Array(transaction.private.outputUtxos.length).fill(
      "0",
    ),
  };

  // TODO: remove dummy nullifier inputs
  console.log(
    "using dummy nullifier inputs for until nullifier tree is active in programs.",
  );
  const proofInput = {
    ...publicProgramCircuitInputs,
    ...programCircuitInputs,
    publicStateRoot: new Array(transaction.private.inputUtxos.length).fill(
      root,
    ),
    publicNullifierRoot: new Array(transaction.private.inputUtxos.length).fill(
      "0",
    ), //new Array(transaction.private.inputUtxos.length).fill(root),
    nullifierLeafIndex: new Array(transaction.private.inputUtxos.length).fill(
      "0",
    ),
    nullifierMerkleProof: new Array(transaction.private.inputUtxos.length).fill(
      new Array(MERKLE_TREE_HEIGHT).fill("0"),
    ),
    publicNullifier,
    publicAmountSpl: transaction.public.publicAmountSpl.toString(),
    publicAmountSol: transaction.public.publicAmountSol.toString(),
    publicMintPublicKey: transaction.public.publicMintPubkey,
    leafIndex: transaction.private.inputUtxos?.map(
      (x) => x.merkleTreeLeafIndex,
    ),
    merkleProof: transaction.private.inputUtxos?.map((x) => x.merkleProof),
    privatePublicDataHash: transaction.public.txIntegrityHash.toString(),
    publicDataHash: transaction.public.txIntegrityHash.toString(),
    publicOutUtxoHash: transaction.private.outputUtxos.map((x) => x.hash),
    inAmount: transaction.private.inputUtxos?.map((x) => x.amounts),
    inBlinding: transaction.private.inputUtxos?.map((x) => x.blinding),
    assetPublicKeys: transaction.private.assetPubkeysCircuit,
    outAmount: transaction.private.outputUtxos?.map((x) => x.amounts),
    outBlinding: transaction.private.outputUtxos?.map((x) => x.blinding),
    outOwner: transaction.private.outputUtxos?.map((utxo) =>
      "data" in utxo
        ? hashAndTruncateToCircuit(utxo.owner.toBytes())
        : utxo.owner,
    ),
    inIndices: getIndices3D(
      transaction.private.inputUtxos[0].assets.length,
      N_ASSET_PUBKEYS,
      transaction.private.inputUtxos.map((utxo) =>
        stringifyAssetsToCircuitInput(utxo.assets),
      ),
      transaction.private.assetPubkeysCircuit,
    ),
    outIndices: getIndices3D(
      transaction.private.inputUtxos[0].assets.length,
      N_ASSET_PUBKEYS,
      transaction.private.outputUtxos.map((utxo) =>
        stringifyAssetsToCircuitInput(utxo.assets),
      ),
      transaction.private.assetPubkeysCircuit,
    ),
    inDataHash: transaction.private.inputUtxos?.map((x) =>
      "dataHash" in x ? x.dataHash : BN_0,
    ),
    outDataHash: transaction.private.outputUtxos?.map((x) =>
      "dataHash" in x ? x.dataHash : BN_0,
    ),
    metaHash: transaction.private.inputUtxos.map((utxo) => {
      if (utxo.metaHash) return utxo.metaHash;
      else return new BN(0);
    }),
    address: transaction.private.inputUtxos.map((utxo) => {
      if (utxo.address) return utxo.address;
      else return new BN(0);
    }),
  };
  return proofInput;
}

// TODO: implement privacy preserving fetching, this fetching strategy is not priaacy preserving for the rpc
export async function syncInputUtxosMerkleProofs({
  inputUtxos,
  rpc,
}: {
  inputUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  rpc: Rpc;
}): Promise<{
  syncedUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  root: string;
  index: number;
}> {
  // skip empty utxos
  const { merkleProofs, root, index } = (await rpc.getMerkleProofByIndexBatch(
    inputUtxos
      .filter((utxo) => !utxo.amounts[0].eq(BN_0) || !utxo.amounts[1].eq(BN_0))
      .map((utxo) => utxo.merkleTreeLeafIndex),
  ))!;
  let tmpIndex = 0;
  const syncedUtxos = inputUtxos?.map((utxo) => {
    // skip empty utxos
    if (!utxo.amounts[0].eq(BN_0) || !utxo.amounts[1].eq(BN_0)) {
      utxo.merkleProof = merkleProofs[tmpIndex];
      tmpIndex++;
    }
    return utxo;
  });
  return { syncedUtxos, root, index };
}

// compileProofInputs
export function createProofInputs({
  transaction,
  root,
  lightWasm,
  account,
  pspTransaction,
}: {
  transaction: Transaction;
  root: string;
  pspTransaction: PspTransactionInput;
  lightWasm: LightWasm;
  account: Account;
}): compiledProofInputs {
  const systemProofInputs: SystemProofInputs = createSystemProofInputs({
    transaction,
    root,
    lightWasm,
    account,
  });
  const pspProofInputs = createPspProofInputs(
    pspTransaction,
    transaction.private.inputUtxos,
    transaction.private.outputUtxos,
    transaction.public.transactionHash.toString(),
  );
  return {
    ...systemProofInputs,
    ...pspProofInputs,
  };
}
export function findIdlIndex(programId: string, idlObjects: Idl[]): number {
  for (let i = 0; i < idlObjects.length; i++) {
    const constants = idlObjects[i].constants;
    if (!constants)
      throw new TransactionError(
        TransactionErrorCode.IDL_CONSTANTS_UNDEFINED,
        "findIdlIndex",
        `Idl in index ${i} does not have any constants`,
      );

    for (const constant of constants) {
      if (
        constant.name === "PROGRAM_ID" &&
        constant.type === "string" &&
        constant.value === `"${programId}"`
      ) {
        return i;
      }
    }
  }

  return -1; // Return -1 if the programId is not found in any IDL object
}

export function getVerifierConfig(verifierIdl: Idl): VerifierConfig {
  const accounts = verifierIdl.accounts;
  const resultElement = accounts!.find(
    (account) =>
      account.name.startsWith("zK") && account.name.endsWith("ProofInputs"),
  );

  if (!resultElement) {
    throw new TransactionError(
      TransactionErrorCode.VERIFIER_CONFIG_UNDEFINED,
      "getVerifierConfig",
      "No matching element found",
    );
  }
  interface Field {
    name: string;
    type: any;
  }

  const fields = resultElement.type.fields;
  const publicNullifierField = fields.find(
    (field) =>
      field.name === "publicNullifier" || field.name === "publicInUtxoHash",
  ) as Field;
  const publicOutUtxoHashField = fields.find(
    (field) => field.name === "publicOutUtxoHash",
  ) as Field;
  // TODO: add new errors which are reliable
  // old error: "publicNullifier field not found or has an incorrect type"
  // old error: "publicOutUtxoHash field not found or has an incorrect type"
  // if (!publicNullifierField || !publicNullifierField.type.array) {
  // if (!publicOutUtxoHashField || !publicOutUtxoHashField.type.array) {

  const publicNullifierLength = publicNullifierField.type.array[1];
  const publicOutUtxoHash = publicOutUtxoHashField.type.array[1];

  return { in: publicNullifierLength, out: publicOutUtxoHash };
}

/**
 * @description Adds empty utxos until the desired number of utxos is reached.
 * @note The zero knowledge proof circuit needs all inputs to be defined.
 * @note Therefore, we have to pass in empty inputs for values we don't use.
 */
export function addFillingOutUtxos(
  utxos: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[] = [],
  len: number,
  lightWasm: LightWasm,
  owner: BN,
): (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[] {
  while (utxos.length < len) {
    utxos.push(
      createFillingOutUtxo({
        lightWasm,
        owner,
      }),
    );
  }
  return utxos;
}

export function addFillingUtxos(
  utxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [],
  len: number,
  lightWasm: LightWasm,
  account: Account,
): (Utxo | ProgramUtxo<PlaceHolderTData>)[] {
  while (utxos.length < len) {
    utxos.push(
      createFillingUtxo({
        lightWasm,
        account,
      }),
    );
  }
  return utxos;
}

/**
 * Assigns spl and sol senderSpl or recipientSpl accounts
 * to transaction parameters based on action.
 */
// solanaTransaction
export function assignAccountsDecompress(
  assetPubkeys: PublicKey[],
  recipientSol?: PublicKey,
  recipientSpl?: PublicKey,
): {
  senderSol: PublicKey;
  senderSpl: PublicKey;
  recipientSol: PublicKey;
  recipientSpl: PublicKey;
} {
  if (!assetPubkeys)
    throw new TransactionParametersError(
      TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
      "assignAccounts assetPubkeys undefined",
      "assignAccounts",
    );
  const senderSpl = MerkleTreeConfig.getSplPoolPdaToken(
    assetPubkeys[1],
    merkleTreeProgramId,
  );
  const senderSol = MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;

  // AUTHORITY is used as placeholder in case no spl recipient is decompressed
  const assignedRecipientSpl = recipientSpl ? recipientSpl : AUTHORITY;
  // AUTHORITY is used as placeholder in case no sol recipient is decompressed
  const assignedRecipientSol = recipientSol ? recipientSol : AUTHORITY;
  return {
    senderSol,
    senderSpl,
    recipientSol: assignedRecipientSol,
    recipientSpl: assignedRecipientSpl,
  };
}

// solanaTransaction assign accounts for compressed transfer
export function assignAccounts(assetPubkeys: PublicKey[]): {
  senderSol: PublicKey;
  senderSpl: PublicKey;
  recipientSol: PublicKey;
  recipientSpl: PublicKey;
} {
  if (!assetPubkeys)
    throw new TransactionParametersError(
      TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
      "assignAccounts assetPubkeys undefined",
      "assignAccounts",
    );
  const senderSpl = MerkleTreeConfig.getSplPoolPdaToken(
    assetPubkeys[1],
    merkleTreeProgramId,
  );
  const senderSol = MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;

  // AUTHORITY is used as placeholder in case no spl recipient is decompressed
  const assignedRecipientSpl = AUTHORITY;
  // AUTHORITY is used as placeholder in case no sol recipient is decompressed
  const assignedRecipientSol = AUTHORITY;
  return {
    senderSol,
    senderSpl,
    recipientSol: assignedRecipientSol,
    recipientSpl: assignedRecipientSpl,
  };
}

export function assignAccountsCompress(
  assetPubkeys: PublicKey[],
  systemPspId: PublicKey,
  senderSpl?: PublicKey,
) {
  const recipientSpl = MerkleTreeConfig.getSplPoolPdaToken(
    assetPubkeys[1],
    merkleTreeProgramId,
  );
  const recipientSol = MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;
  // AUTHORITY is used as placeholder in case no spl recipient is decompressed
  const assignedSenderSpl = senderSpl ? senderSpl : AUTHORITY;

  const senderSol = getEscrowPda(systemPspId);
  return {
    recipientSol,
    recipientSpl,
    senderSol,
    senderSpl: assignedSenderSpl,
    systemPspId,
  };
}

export function getEscrowPda(verifierProgramId: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [utils.bytes.utf8.encode("escrow")],
    verifierProgramId,
  )[0];
}

/**
 * @description Calculates the external amount for one asset.
 * @note This function might be too specific since the circuit allows assets to be in any index
 * @param assetIndex the index of the asset the external amount should be computed for
 * @returns {BN} the public amount of the asset
 */
// pspTransaction
export function getExternalAmount(
  assetIndex: number,
  inputUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[],
  outputUtxos: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[],
  assetPubkeysCircuit: string[],
): BN {
  return new BN(0)
    .add(
      outputUtxos
        .filter((utxo: OutUtxo | ProgramOutUtxo<PlaceHolderTData>) => {
          const assetCircuitInput = stringifyAssetsToCircuitInput(utxo.assets)[
            assetIndex
          ].toString();
          return assetCircuitInput === assetPubkeysCircuit![assetIndex];
        })
        .reduce(
          (sum, utxo) =>
            // add all utxos of the same asset
            sum.add(utxo.amounts[assetIndex]),
          new BN(0),
        ),
    )
    .sub(
      inputUtxos
        .filter((utxo) => {
          const assetCircuitInput = stringifyAssetsToCircuitInput(utxo.assets)[
            assetIndex
          ].toString();
          return assetCircuitInput === assetPubkeysCircuit[assetIndex];
        })
        .reduce((sum, utxo) => sum.add(utxo.amounts[assetIndex]), new BN(0)),
    )
    .add(FIELD_SIZE)
    .mod(FIELD_SIZE);
}

/**
 * Computes the integrity Poseidon hash over transaction inputs that are not part of
 * the proof, but are included to prevent the rpc from changing any input of the
 * transaction.
 *
 * The hash is computed over the following inputs in the given order:
 * 1. Recipient SPL Account
 * 2. Recipient Solana Account
 * 3. Rpc Public Key
 * 4. Rpc Fee
 * 5. Encrypted UTXOs (limited to 512 bytes)
 *
 * @param {any} poseidon - Poseidon hash function instance.
 * @returns {Promise<BN>} A promise that resolves to the computed transaction integrity hash.
 * @throws {TransactionError} Throws an error if the rpc, recipient SPL or Solana accounts,
 * rpc fee, or encrypted UTXOs are undefined, or if the encryption of UTXOs fails.
 *
 * @example
 * const integrityHash = await getTxIntegrityHash(poseidonInstance);
 */
export function getTxIntegrityHash(
  rpcFee: BN,
  encryptedUtxos: Uint8Array,
  accounts: DecompressAccounts,
  verifierConfig: VerifierConfig,
  verifierProgramId: PublicKey,
  message?: Uint8Array,
): BN {
  if (!rpcFee)
    throw new TransactionError(
      TransactionErrorCode.RPC_UNDEFINED,
      "getTxIntegrityHash",
      "",
    );
  if (!accounts.recipientSpl)
    throw new TransactionError(
      TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
      "getTxIntegrityHash",
      "",
    );
  if (!accounts.recipientSol)
    throw new TransactionError(
      TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
      "getTxIntegrityHash",
      "",
    );

  if (encryptedUtxos && encryptedUtxos.length > 128 * verifierConfig.out)
    throw new TransactionParametersError(
      TransactionParametersErrorCode.ENCRYPTED_UTXOS_TOO_LONG,
      "getTxIntegrityHash",
      `Encrypted utxos are too long: ${encryptedUtxos.length} > ${
        128 * verifierConfig.out
      }`,
    );

  const messageHash = message ? sha256(message) : new Uint8Array(32);

  // TODO(vadorovsky): Try to get rid of this hack during Verifier class
  // refactoring / removal
  // For example, we could derive which accounts exist in the IDL of the
  // verifier program method.
  const recipientSpl =
    verifierProgramId.toBase58() === lightPsp2in2outStorageId.toBase58()
      ? new Uint8Array(32)
      : accounts.recipientSpl.toBytes();

  const hash = sha256
    .create()
    .update(messageHash)
    .update(recipientSpl)
    .update(accounts.recipientSol.toBytes())
    .update(accounts.rpcPublicKey.toBytes())
    .update(rpcFee.toArrayLike(Buffer, "be", 8)) // TODO: make be
    .update(encryptedUtxos)
    .digest();
  const txIntegrityHash = truncateToCircuit(hash);
  return txIntegrityHash;
}

// pspTransaction
/** Encrypts a batch of output utxos */
export async function encryptOutUtxos(
  account: Account,
  outputUtxos: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[],
  transactionMerkleTree: PublicKey,
  verifierConfig: VerifierConfig,
  assetLookupTable: string[],
  lightWasm: LightWasm,
): Promise<Uint8Array> {
  let encryptedOutputs = new Array<any>();
  for (const utxo in outputUtxos) {
    // if (outputUtxos[utxo].dataHash.toString() !== "0")
    //   // TODO: implement encryption for utxos with app data
    //   console.log(
    //     "Warning encrypting utxos with app data as normal utxo without app data. App data will not be encrypted.",
    //   );

    encryptedOutputs.push(
      await encryptOutUtxo({
        lightWasm,
        account,
        utxo: outputUtxos[utxo],
        merkleTreePdaPublicKey: transactionMerkleTree,
        compressed: true,
        assetLookupTable,
      }),
    );
  }
  encryptedOutputs = encryptedOutputs.map((elem) => Array.from(elem)).flat();
  if (
    encryptedOutputs.length < 128 * verifierConfig.out &&
    verifierConfig.out === 2
  ) {
    return new Uint8Array([
      ...encryptedOutputs,
      ...new Array(128 * verifierConfig.out - encryptedOutputs.length).fill(0),
      // for verifier zero and one these bytes are not sent and just added for the integrity hash
      // to be consistent, if the bytes were sent to the chain use rnd bytes for padding
    ]);
  }
  if (encryptedOutputs.length < 128 * verifierConfig.out) {
    return new Uint8Array([
      ...encryptedOutputs,
      ...nacl.randomBytes(128 * verifierConfig.out - encryptedOutputs.length),
    ]);
  }
  return new Uint8Array(encryptedOutputs);
}

// pspTransaction
export function getTransactionHash(
  inputUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[],
  outputUtxos: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[],
  txIntegrityHash: BN,
  lightWasm: LightWasm,
): string {
  const inputHasher = lightWasm.poseidonHashString(
    inputUtxos?.map((utxo) => utxo.hash),
  );
  const outputHasher = lightWasm.poseidonHashString(
    outputUtxos?.map((utxo) => utxo.hash),
  );
  return lightWasm.poseidonHashString([
    inputHasher,
    outputHasher,
    txIntegrityHash.toString(),
  ]);
}
// add createCompressSolanaTransaction
export async function createCompressTransaction<
  T extends Utxo | ProgramUtxo<PlaceHolderTData>,
  TOut extends OutUtxo | ProgramOutUtxo<PlaceHolderTData>,
>({
  mint,
  message,
  merkleTreeSetPubkey,
  senderSpl,
  inputUtxos,
  outputUtxos,
  signer,
  lightWasm,
  systemPspId,
  pspId,
  account,
  assetLookUpTable,
}: {
  mint?: PublicKey;
  message?: Buffer;
  merkleTreeSetPubkey: PublicKey;
  senderSpl?: PublicKey;
  inputUtxos?: T[];
  outputUtxos?: TOut[];
  signer: PublicKey;
  lightWasm: LightWasm;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  assetLookUpTable?: string[];
}): Promise<CompressTransaction> {
  assetLookUpTable = assetLookUpTable ?? [
    SystemProgram.programId.toBase58(),
    MINT.toBase58(),
  ];

  const action = Action.COMPRESS;
  const verifierIdl = getSystemPspIdl(systemPspId);
  const verifierConfig = getVerifierConfig(verifierIdl);

  const privateVars = createPrivateTransactionVariables({
    inputUtxos,
    outputUtxos,
    lightWasm,
    account,
    verifierConfig,
  });
  const publicAmountSol = getExternalAmount(
    0,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    privateVars.assetPubkeysCircuit,
  );
  const publicAmountSpl = getExternalAmount(
    1,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    privateVars.assetPubkeysCircuit,
  );

  const accounts = assignAccountsCompress(
    privateVars.assetPubkeys,
    systemPspId,
    senderSpl,
  );
  const completeAccounts = {
    ...accounts,
    rpcPublicKey: signer,
    pspId,
  };

  // TODO: double check onchain code for consistency between utxo merkle trees and inserted merkle tree
  const encryptedUtxos = await encryptOutUtxos(
    account,
    privateVars.outputUtxos,
    merkleTreeSetPubkey,
    verifierConfig,
    assetLookUpTable,
    lightWasm,
  );
  const txIntegrityHash = getTxIntegrityHash(
    BN_0,
    encryptedUtxos,
    completeAccounts,
    verifierConfig,
    systemPspId,
    message,
  );

  const transactionHash = getTransactionHash(
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    txIntegrityHash,
    lightWasm,
  );

  const transaction: CompressTransaction = {
    action,
    private: privateVars,
    public: {
      transactionHash,
      publicMintPubkey: mint
        ? hashAndTruncateToCircuit(mint.toBytes()).toString()
        : "0",
      txIntegrityHash,
      accounts: {
        ...completeAccounts,
        merkleTreeSet: merkleTreeSetPubkey,
      },
      publicAmountSpl,
      publicAmountSol,
      rpcFee: BN_0,
      ataCreationFee: false,
      encryptedUtxos,
      message,
    },
  };

  return transaction;
}

export function createPrivateTransactionVariables({
  inputUtxos,
  outputUtxos,
  lightWasm,
  account,
  verifierConfig,
}: {
  inputUtxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outputUtxos?: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[];
  lightWasm: LightWasm;
  account: Account;
  verifierConfig: VerifierConfig;
}): PrivateTransactionVariables {
  const filledInputUtxos = addFillingUtxos(
    inputUtxos,
    verifierConfig.in,
    lightWasm,
    account,
  );

  const filledOutputUtxos = addFillingOutUtxos(
    outputUtxos,
    verifierConfig.out,
    lightWasm,
    account.keypair.publicKey,
  );

  const { assetPubkeysCircuit, assetPubkeys } = getAssetPubkeys(
    filledInputUtxos,
    filledOutputUtxos,
  );

  return {
    inputUtxos: filledInputUtxos,
    outputUtxos: filledOutputUtxos,
    assetPubkeys,
    assetPubkeysCircuit,
  };
}
// add createCompressSolanaTransaction
export async function createDecompressTransaction(
  decompressTransactionInput: DecompressTransactionInput,
): Promise<DecompressTransaction> {
  const {
    message,
    merkleTreeSetPubkey,
    mint,
    recipientSpl,
    recipientSol,
    inputUtxos,
    outputUtxos,
    rpcPublicKey,
    lightWasm,
    systemPspId,
    pspId,
    account,
    rpcFee,
    ataCreationFee,
  } = decompressTransactionInput;
  const assetLookUpTable = decompressTransactionInput.assetLookUpTable
    ? decompressTransactionInput.assetLookUpTable
    : [SystemProgram.programId.toBase58(), MINT.toBase58()];

  const action = Action.DECOMPRESS;
  const verifierIdl = getSystemPspIdl(systemPspId);
  const verifierConfig = getVerifierConfig(verifierIdl);

  const privateVars = createPrivateTransactionVariables({
    inputUtxos,
    outputUtxos,
    lightWasm,
    account,
    verifierConfig,
  });

  const publicAmountSol = getExternalAmount(
    0,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    privateVars.assetPubkeysCircuit,
  );
  const publicAmountSpl = getExternalAmount(
    1,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    privateVars.assetPubkeysCircuit,
  );

  const accounts = assignAccountsDecompress(
    privateVars.assetPubkeys,
    recipientSol,
    recipientSpl,
  );
  const completeAccounts = {
    ...accounts,
    rpcPublicKey,
    systemPspId,
    pspId,
    merkleTreeSet: merkleTreeSetPubkey,
  };

  // TODO: double check onchain code for consistency between utxo merkle trees and inserted merkle tree
  const encryptedUtxos = await encryptOutUtxos(
    account,
    privateVars.outputUtxos,
    merkleTreeSetPubkey,
    verifierConfig,
    assetLookUpTable,
    lightWasm,
  );
  const txIntegrityHash = getTxIntegrityHash(
    rpcFee,
    encryptedUtxos,
    completeAccounts,
    verifierConfig,
    systemPspId,
    message,
  );

  const transactionHash = getTransactionHash(
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    txIntegrityHash,
    lightWasm,
  );

  const transaction: DecompressTransaction = {
    action,
    private: privateVars,
    public: {
      transactionHash,
      publicMintPubkey: mint
        ? hashAndTruncateToCircuit(mint.toBytes()).toString()
        : "0",
      txIntegrityHash,
      accounts: completeAccounts,
      publicAmountSpl,
      publicAmountSol,
      rpcFee,
      ataCreationFee,
      encryptedUtxos,
      message,
    },
  };

  return transaction;
}
export async function createTransaction(
  transactionInput: TransactionInput,
): Promise<Transaction> {
  const {
    message,
    merkleTreeSetPubkey,
    inputUtxos,
    outputUtxos,
    rpcPublicKey,
    lightWasm,
    pspId,
    systemPspId,
    account,
    rpcFee,
  } = transactionInput;
  const assetLookUpTable = transactionInput.assetLookUpTable
    ? transactionInput.assetLookUpTable
    : [SystemProgram.programId.toBase58(), MINT.toBase58()];
  const verifierProgramId = pspId ?? systemPspId;
  // TODO: unify systemPspId and verifierProgramId by adding verifierConfig to psps
  const verifierConfig = getVerifierConfig(getSystemPspIdl(systemPspId));

  const privateVars = createPrivateTransactionVariables({
    inputUtxos,
    outputUtxos,
    lightWasm,
    account,
    verifierConfig,
  });
  const publicAmountSol = getExternalAmount(
    0,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    privateVars.assetPubkeysCircuit,
  );
  const completeAccounts = {
    senderSol: MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda,
    senderSpl: AUTHORITY,
    recipientSol: AUTHORITY,
    recipientSpl: AUTHORITY,
    rpcPublicKey,
  };

  // TODO: double check onchain code for consistency between utxo merkle trees and inserted merkle tree
  const encryptedUtxos = await encryptOutUtxos(
    account,
    privateVars.outputUtxos,
    merkleTreeSetPubkey,
    verifierConfig,
    assetLookUpTable,
    lightWasm,
  );

  const txIntegrityHash = getTxIntegrityHash(
    rpcFee,
    encryptedUtxos,
    completeAccounts,
    verifierConfig,
    verifierProgramId,
    message,
  );

  const transactionHash = getTransactionHash(
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    txIntegrityHash,
    lightWasm,
  );

  const transaction: Transaction = {
    private: privateVars,
    public: {
      transactionHash,
      publicMintPubkey: "0",
      txIntegrityHash,
      accounts: {
        ...completeAccounts,
        merkleTreeSet: merkleTreeSetPubkey,
        systemPspId,
        pspId,
      },
      publicAmountSpl: BN_0,
      publicAmountSol,
      rpcFee,
      ataCreationFee: false,
      encryptedUtxos,
      message,
    },
  };

  return transaction;
}

export function getSystemPspIdl(programId: PublicKey): Idl {
  const idl = SYSTEM_PROGRAM_IDLS.get(programId.toBase58());
  if (!idl) {
    throw new TransactionError(
      TransactionErrorCode.INVALID_SYSTEM_PROGRAM_ID,
      "getSystemPspIdl",
      `Invalid system program provided program id ${programId.toBase58()}`,
    );
  }
  return idl;
}

export async function getTxParams({
  tokenCtx,
  publicAmountSpl = BN_0,
  publicAmountSol = BN_0,
  action,
  userSplAccount = AUTHORITY,
  account,
  utxos,
  inUtxos,
  // for decompress
  recipientSol,
  recipientSplAddress,
  // for transfer
  outUtxos,
  rpc,
  provider,
  ataCreationFee, // associatedTokenAccount = ata
  appUtxo,
  addInUtxos = true,
  addOutUtxos = true,
  verifierIdl,
  mergeUtxos = false,
  message,
  assetLookupTable,
  verifierProgramLookupTable,
  separateSolUtxo = false,
}: {
  tokenCtx: TokenData;
  publicAmountSpl?: BN;
  publicAmountSol?: BN;
  userSplAccount?: PublicKey;
  account: Account;
  utxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  recipientSol?: PublicKey;
  recipientSplAddress?: PublicKey;
  inUtxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outUtxos?: OutUtxo[];
  action: Action;
  provider: Provider;
  rpc: Rpc;
  ataCreationFee?: boolean;
  appUtxo?: AppUtxoConfig;
  addInUtxos?: boolean;
  addOutUtxos?: boolean;
  verifierIdl: Idl;
  mergeUtxos?: boolean;
  message?: Buffer;
  assetLookupTable: string[];
  verifierProgramLookupTable: string[];
  separateSolUtxo?: boolean;
}): Promise<Transaction | CompressTransaction | DecompressTransaction> {
  if (action === Action.TRANSFER && !outUtxos && !mergeUtxos)
    throw new TransactionParametersError(
      UserErrorCode.COMPRESSED_RECIPIENT_UNDEFINED,
      "getTxParams",
      "Recipient outUtxo not provided for transfer",
    );
  if (!rpc) {
    throw new TransactionParametersError(
      TransactionErrorCode.RPC_UNDEFINED,
      "getTxParams",
      "Fetching root from rpc failed.",
    );
  }
  if (action !== Action.COMPRESS && !rpc.getRpcFee(ataCreationFee)) {
    // TODO: could make easier to read by adding separate if/cases
    throw new TransactionParametersError(
      RpcErrorCode.RPC_FEE_UNDEFINED,
      "getTxParams",
      `No rpcFee provided for ${action.toLowerCase()}}`,
    );
  }
  if (!account) {
    throw new TransactionParametersError(
      CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
      "getTxParams",
      "account for change utxo is undefined",
    );
  }

  let inputUtxos = inUtxos ? [...inUtxos] : [];
  let outputUtxos: OutUtxo[] = outUtxos ? [...outUtxos] : [];

  if (addInUtxos) {
    inputUtxos = selectInUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl,
      publicAmountSol,
      lightWasm: provider.lightWasm,
      inUtxos,
      outUtxos,
      utxos,
      rpcFee:
        action == Action.COMPRESS ? undefined : rpc.getRpcFee(ataCreationFee),
      action,
      numberMaxInUtxos: getVerifierConfig(verifierIdl).in,
      numberMaxOutUtxos: getVerifierConfig(verifierIdl).out,
    });
  }
  if (addOutUtxos) {
    outputUtxos = createOutUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl,
      inUtxos: inputUtxos,
      publicAmountSol, // TODO: add support for extra sol for decompress & transfer
      lightWasm: provider.lightWasm,
      rpcFee:
        action == Action.COMPRESS ? undefined : rpc.getRpcFee(ataCreationFee),
      changeUtxoAccount: account,
      outUtxos,
      action,
      appUtxo,
      numberMaxOutUtxos: getVerifierConfig(verifierIdl).out,
      assetLookupTable,
      verifierProgramLookupTable,
      separateSolUtxo,
    });
  }

  if (action == Action.COMPRESS) {
    return createCompressTransaction({
      message,
      merkleTreeSetPubkey: rpc.accounts.merkleTreeSet,
      mint:
        publicAmountSpl && !publicAmountSpl.eq(BN_0)
          ? tokenCtx.mint
          : undefined,
      senderSpl: userSplAccount,
      inputUtxos,
      outputUtxos,
      signer: provider.wallet.publicKey,
      systemPspId: getVerifierProgramId(verifierIdl),
      account,
      lightWasm: provider.lightWasm,
    });
  } else if (action == Action.DECOMPRESS) {
    return createDecompressTransaction({
      message,
      merkleTreeSetPubkey: rpc.accounts.merkleTreeSet,
      mint:
        publicAmountSpl && !publicAmountSpl.eq(BN_0)
          ? tokenCtx.mint
          : undefined,
      recipientSol,
      recipientSpl: recipientSplAddress,
      inputUtxos,
      outputUtxos,
      lightWasm: provider.lightWasm,
      systemPspId: getVerifierProgramId(verifierIdl),
      account,
      ataCreationFee: ataCreationFee ? true : false,
      rpcPublicKey: rpc!.accounts.rpcPubkey,
      rpcFee: rpc.getRpcFee(ataCreationFee),
    });
  } else if (action == Action.TRANSFER) {
    return createTransaction({
      message,
      merkleTreeSetPubkey: rpc.accounts.merkleTreeSet,
      inputUtxos,
      outputUtxos,
      lightWasm: provider.lightWasm,
      systemPspId: getVerifierProgramId(verifierIdl),
      account,
      rpcPublicKey: rpc!.accounts.rpcPubkey,
      rpcFee: rpc.getRpcFee(ataCreationFee),
    });
  } else {
    throw new TransactionParametersError(
      TransactionErrorCode.UNIMPLEMENTED,
      "getTxParams",
      `Action ${action} not implemented`,
    );
  }
}
