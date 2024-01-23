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
import { hashAndTruncateToCircuit, truncateToCircuit } from "../utils";
import { Action } from "../types";
import { MerkleTreeConfig } from "../merkle-tree";
import { TokenData } from "../types";
import { MINT } from "../test-utils";
import {
  createFillingOutUtxo,
  createFillingUtxo,
  createOutUtxos,
  selectInUtxos,
} from "../utxo";
import { Utxo, OutUtxo, encryptOutUtxo } from "../utxo";
import { AppUtxoConfig } from "../types";
import { Rpc } from "../rpc";
import { Provider } from "../provider";

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

//TODO: make part of transaction parameters(Transaction)
export type PspTransactionInput = {
  proofInputs: any;
  verifierIdl: Idl;
  circuitName: string;
  path: string;
  checkedInUtxos?: { utxoName: string; utxo: Utxo }[];
  checkedOutUtxos?: { utxoName: string; utxo: OutUtxo }[];
  inUtxos?: Utxo[];
  outUtxos?: OutUtxo[];
  accounts?: any;
};
8;
type compiledProofInputs = {
  systemProofInputs: any;
  pspProofInputs: any;
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
  utxos: Utxo[] | OutUtxo[],
  utxoHash: string,
) => {
  const isAppInUtxo = new Array(4).fill(new BN(0));
  for (const i in utxos) {
    if (utxos[i].utxoHash === utxoHash) {
      isAppInUtxo[i] = new BN(1);
    }
  }
  return isAppInUtxo;
};

// TODO: resolve out utxo vs program utxo type use
export const createPspProofInputs = (
  lightWasm: LightWasm,
  pspTransaction: PspTransactionInput,
  inputUtxos: Utxo[],
  outputUtxos: OutUtxo[],
  publicTransactionHash: string,
): any => {
  const inUtxosInputs = {};
  pspTransaction.checkedInUtxos?.forEach(({ utxoName, utxo: programUtxo }) => {
    const utxo = programUtxo;
    for (const field in programUtxo.utxoData) {
      // @ts-ignore
      inUtxosInputs[`${utxoName}${upperCamelCase(field)}`] =
        programUtxo.utxoData[field];
    }

    const isAppUtxo = createUtxoIndices(inputUtxos, utxo.utxoHash);
    // @ts-ignore
    inUtxosInputs[`isInProgramUtxo${upperCamelCase(utxoName)}`] = isAppUtxo;
    inUtxosInputs[`${camelCase(utxoName)}Blinding`] = utxo.blinding;
    inUtxosInputs[`${camelCase(utxoName)}AmountSol`] = utxo.amounts[0];
    inUtxosInputs[`${camelCase(utxoName)}AmountSpl`] =
      utxo.amounts.length === 2 ? utxo.amounts[1] : BN_0;
    inUtxosInputs[`${camelCase(utxoName)}AssetSpl`] = utxo.assetsCircuit[1];
    inUtxosInputs[`${camelCase(utxoName)}Owner`] = utxo.publicKey;
    inUtxosInputs[`${camelCase(utxoName)}Type`] = utxo.poolType;
    inUtxosInputs[`${camelCase(utxoName)}Version`] = BN_0;
    inUtxosInputs[`${camelCase(utxoName)}MetaHash`] = utxo.metaHash || BN_0;
    inUtxosInputs[`${camelCase(utxoName)}Address`] = utxo.address || BN_0;
    // utxo data hash is calculated in the circuit
  });

  // TODO: think about how to make outUtxos and programOutUtxos consistent, do I need utxoData in outUtxos?
  const outUtxosInputs = {};
  pspTransaction.checkedOutUtxos?.forEach(
    ({ utxoName, utxo: programUtxo }: { utxoName: string; utxo: OutUtxo }) => {
      const utxo = programUtxo;
      for (const field in utxo.utxoData) {
        // @ts-ignore
        outUtxosInputs[`${utxoName}${upperCamelCase(field)}`] =
          utxo.utxoData[field];
      }

      const isAppUtxoIndices = createUtxoIndices(outputUtxos, utxo.utxoHash);
      // @ts-ignore
      outUtxosInputs[`isOutProgramUtxo${upperCamelCase(utxoName)}`] =
        isAppUtxoIndices;
      outUtxosInputs[`${camelCase(utxoName)}Blinding`] = utxo.blinding;
      outUtxosInputs[`${camelCase(utxoName)}AmountSol`] = utxo.amounts[0];
      outUtxosInputs[`${camelCase(utxoName)}AmountSpl`] =
        utxo.amounts.length === 2 ? utxo.amounts[1] : BN_0;
      outUtxosInputs[`${camelCase(utxoName)}AssetSpl`] = utxo.assetsCircuit[1];
      outUtxosInputs[`${camelCase(utxoName)}Owner`] = utxo.publicKey;
      outUtxosInputs[`${camelCase(utxoName)}Type`] = utxo.poolType;
      outUtxosInputs[`${camelCase(utxoName)}Version`] = BN_0;
      outUtxosInputs[`${camelCase(utxoName)}MetaHash`] = utxo.metaHash || BN_0;
      outUtxosInputs[`${camelCase(utxoName)}Address`] = utxo.address || BN_0;
    },
  );

  const publicProgramId = hashAndTruncateToCircuit(
    getVerifierProgramId(pspTransaction.verifierIdl).toBuffer(),
  );

  const compiledProofInputs = {
    ...pspTransaction.proofInputs,
    inOwner: inputUtxos?.map((utxo) => utxo.publicKey),
    publicTransactionHash,
    publicProgramId,
    ...inUtxosInputs,
    ...outUtxosInputs,
    inAsset: inputUtxos?.map((utxo) => [
      utxo.assetsCircuit[0],
      utxo.assetsCircuit[1],
    ]),
    outAsset: outputUtxos?.map((utxo) => [
      utxo.assetsCircuit[0],
      utxo.assetsCircuit[1],
    ]),
    inAddress: inputUtxos?.map((utxo) => utxo.address || BN_0),
    outAddress: outputUtxos?.map((utxo) => utxo.address || BN_0),
    inMetaHash: inputUtxos?.map((utxo) => utxo.metaHash || BN_0),
    outMetaHash: outputUtxos?.map((utxo) => utxo.metaHash || BN_0),
  };
  return compiledProofInputs;
};

// TODO: add check that length input utxos is as expected by the verifier idl
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
  inputUtxos: Utxo[];
  systemProofInputs: any;
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
 * @description Prepares proof inputs.
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
}) {
  if (!transaction.public.txIntegrityHash)
    throw new TransactionError(
      TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
      "compile",
    );

  const publicNullifier = transaction.private.inputUtxos.map((x) => {
    let _account = account;
    if (new BN(x.publicKey).eq(STANDARD_COMPRESSION_PUBLIC_KEY)) {
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
    publicInUtxoDataHash: transaction.private.inputUtxos?.map(
      (x) => x.utxoDataHash,
    ),
    publicInUtxoHash: transaction.private.inputUtxos.map((x) => x.utxoHash),
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
      if (utxo.utxoDataHash.toString() !== "0") return new BN(1);
      else return new BN(0);
    }),
  };

  const programCircuitInputs = {
    isInProgramUtxo: transaction.private.inputUtxos.map((utxo) => {
      if (utxo.utxoDataHash !== "0") return new BN(1);
      else return new BN(0);
    }),
    inOwner: transaction.private.inputUtxos.map((utxo) => utxo.publicKey),
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
    publicOutUtxoHash: transaction.private.outputUtxos.map((x) => x.utxoHash),
    inAmount: transaction.private.inputUtxos?.map((x) => x.amounts),
    inBlinding: transaction.private.inputUtxos?.map((x) => x.blinding),
    assetPublicKeys: transaction.private.assetPubkeysCircuit,
    outAmount: transaction.private.outputUtxos?.map((x) => x.amounts),
    outBlinding: transaction.private.outputUtxos?.map((x) => x.blinding),
    outOwner: transaction.private.outputUtxos?.map((x) => x.publicKey),
    inIndices: getIndices3D(
      transaction.private.inputUtxos[0].assets.length,
      N_ASSET_PUBKEYS,
      transaction.private.inputUtxos.map((utxo) => utxo.assetsCircuit),
      transaction.private.assetPubkeysCircuit,
    ),
    outIndices: getIndices3D(
      transaction.private.inputUtxos[0].assets.length,
      N_ASSET_PUBKEYS,
      transaction.private.outputUtxos.map((utxo) => utxo.assetsCircuit),
      transaction.private.assetPubkeysCircuit,
    ),
    inDataHash: transaction.private.inputUtxos?.map((x) => x.utxoDataHash),
    outDataHash: transaction.private.outputUtxos?.map((x) => x.utxoDataHash),
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

export function getTransactionMint(transaction: Transaction) {
  if (transaction.public.publicAmountSpl.eq(BN_0)) {
    return BN_0;
  } else if (transaction.private.assetPubkeysCircuit) {
    return transaction.private.assetPubkeysCircuit[1];
  } else {
    throw new TransactionError(
      TransactionErrorCode.GET_MINT_FAILED,
      "getMint",
      "Failed to retrieve mint. The transaction parameters should contain 'assetPubkeysCircuit' after initialization, but it's missing.",
    );
  }
}

// TODO: implement privacy preserving fetching, this fetching strategy is not priaacy preserving for the rpc
export async function syncInputUtxosMerkleProofs({
  inputUtxos,
  rpc,
  merkleTreePublicKey,
}: {
  inputUtxos: Utxo[];
  merkleTreePublicKey: PublicKey;
  rpc: Rpc;
}): Promise<{ syncedUtxos: Utxo[]; root: string; index: number }> {
  // skip empty utxos
  const { merkleProofs, root, index } = (await rpc.getMerkleProofByIndexBatch(
    merkleTreePublicKey,
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
  const systemProofInputs = createSystemProofInputs({
    transaction,
    root,
    lightWasm,
    account,
  });
  const pspProofInputs = createPspProofInputs(
    lightWasm,
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
  transactionMerkleTree: PublicKey;
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
  inputUtxos: Array<Utxo>;
  outputUtxos: Array<OutUtxo>;
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

export function getVerifierProgramId(verifierIdl: Idl): PublicKey {
  const programIdObj = verifierIdl.constants!.find(
    (constant) => constant.name === "PROGRAM_ID",
  );
  if (!programIdObj || typeof programIdObj.value !== "string") {
    throw new TransactionParametersError(
      TransactionParametersErrorCode.PROGRAM_ID_CONSTANT_UNDEFINED,
      'PROGRAM_ID constant not found in idl. Example: pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";',
    );
  }

  // Extracting the public key string value from the object and removing quotes.
  const programIdStr = programIdObj.value.slice(1, -1);
  return new PublicKey(programIdStr);
}

export function getVerifierProgram(
  verifierIdl: Idl,
  anchorProvider: AnchorProvider,
): Program<Idl> {
  const programId = getVerifierProgramId(verifierIdl);
  const verifierProgram = new Program(verifierIdl, programId, anchorProvider);
  return verifierProgram;
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
 * @param utxos
 * @param len
 * @returns
 */
export function addFillingOutUtxos(
  utxos: OutUtxo[] = [],
  len: number,
  lightWasm: LightWasm,
  publicKey: BN,
): OutUtxo[] {
  while (utxos.length < len) {
    utxos.push(
      createFillingOutUtxo({
        lightWasm,
        publicKey,
      }),
    );
  }
  return utxos;
}

export function addFillingUtxos(
  utxos: Utxo[] = [],
  len: number,
  lightWasm: LightWasm,
  account: Account,
): Utxo[] {
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
 * @description Assigns spl and sol senderSpl or recipientSpl accounts to transaction parameters based on action.
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

// pspTransaction
export function getAssetPubkeys(
  inputUtxos?: Utxo[],
  outputUtxos?: OutUtxo[],
): { assetPubkeysCircuit: string[]; assetPubkeys: PublicKey[] } {
  const assetPubkeysCircuit: string[] = [
    hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
  ];

  const assetPubkeys: PublicKey[] = [SystemProgram.programId];

  if (inputUtxos) {
    inputUtxos.map((utxo) => {
      let found = false;
      if (
        assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1].toString()) !== -1
      ) {
        found = true;
      }

      if (!found && utxo.assetsCircuit[1].toString() != "0") {
        assetPubkeysCircuit.push(utxo.assetsCircuit[1].toString());
        assetPubkeys.push(utxo.assets[1]);
      }
    });
  }

  if (outputUtxos) {
    outputUtxos.map((utxo) => {
      let found = false;
      for (const _asset in assetPubkeysCircuit) {
        if (
          assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1].toString()) !== -1
        ) {
          found = true;
        }
      }
      if (!found && utxo.assetsCircuit[1].toString() != "0") {
        assetPubkeysCircuit.push(utxo.assetsCircuit[1].toString());
        assetPubkeys.push(utxo.assets[1]);
      }
    });
  }

  if (
    (!inputUtxos && !outputUtxos) ||
    (inputUtxos?.length == 0 && outputUtxos?.length == 0)
  ) {
    throw new TransactionError(
      TransactionErrorCode.NO_UTXOS_PROVIDED,
      "getAssetPubkeys",
      "No input or output utxos provided.",
    );
  }

  // TODO: test this better
  // if (assetPubkeys.length > params?.verifier.config.out) {
  //   throw new TransactionError(
  //     TransactionErrorCode.EXCEEDED_MAX_ASSETS,
  //     "getAssetPubkeys",
  //     `Utxos contain too many different assets ${params?.verifier.config.out} > max allowed: ${N_ASSET_PUBKEYS}`,
  //   );
  // }

  if (assetPubkeys.length > N_ASSET_PUBKEYS) {
    throw new TransactionError(
      TransactionErrorCode.EXCEEDED_MAX_ASSETS,
      "getAssetPubkeys",
      `Utxos contain too many different assets ${assetPubkeys.length} > max allowed: ${N_ASSET_PUBKEYS}`,
    );
  }

  while (assetPubkeysCircuit.length < N_ASSET_PUBKEYS) {
    assetPubkeysCircuit.push(BN_0.toString());
    assetPubkeys.push(SystemProgram.programId);
  }

  return { assetPubkeysCircuit, assetPubkeys };
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
  inputUtxos: Utxo[],
  outputUtxos: OutUtxo[],
  assetPubkeysCircuit: string[],
): BN {
  return new BN(0)
    .add(
      outputUtxos
        .filter((utxo: OutUtxo) => {
          return (
            utxo.assetsCircuit[assetIndex].toString() ==
            assetPubkeysCircuit![assetIndex]
          );
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
          return (
            utxo.assetsCircuit[assetIndex].toString() ==
            assetPubkeysCircuit[assetIndex]
          );
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
export async function encryptOutUtxos(
  account: Account,
  outputUtxos: OutUtxo[],
  transactionMerkleTree: PublicKey,
  verifierConfig: VerifierConfig,
  assetLookupTable: string[],
  lightWasm: LightWasm,
): Promise<Uint8Array> {
  let encryptedOutputs = new Array<any>();
  for (const utxo in outputUtxos) {
    if (outputUtxos[utxo].utxoDataHash.toString() !== "0")
      // TODO: implement encryption for utxos with app data
      console.log(
        "Warning encrypting utxos with app data as normal utxo without app data. App data will not be encrypted.",
      );

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
  inputUtxos: Utxo[],
  outputUtxos: OutUtxo[],
  txIntegrityHash: BN,
  lightWasm: LightWasm,
): string {
  const inputHasher = lightWasm.poseidonHashString(
    inputUtxos?.map((utxo) => utxo.utxoHash),
  );
  const outputHasher = lightWasm.poseidonHashString(
    outputUtxos?.map((utxo) => utxo.utxoHash),
  );
  return lightWasm.poseidonHashString([
    inputHasher,
    outputHasher,
    txIntegrityHash.toString(),
  ]);
}
export type CompressTransactionInput = {
  mint?: PublicKey;
  message?: Buffer;
  transactionMerkleTreePubkey: PublicKey;
  senderSpl?: PublicKey;
  inputUtxos?: Utxo[];
  outputUtxos?: OutUtxo[];
  signer: PublicKey;
  lightWasm: LightWasm;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  assetLookUpTable?: string[];
};

// add createCompressSolanaTransaction
export async function createCompressTransaction(
  compressTransactionInput: CompressTransactionInput,
): Promise<CompressTransaction> {
  const {
    message,
    transactionMerkleTreePubkey,
    mint,
    senderSpl,
    inputUtxos,
    outputUtxos,
    signer,
    systemPspId,
    pspId,
    account,
    lightWasm,
  } = compressTransactionInput;
  const assetLookUpTable = compressTransactionInput.assetLookUpTable
    ? compressTransactionInput.assetLookUpTable
    : [SystemProgram.programId.toBase58(), MINT.toBase58()];

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
    transactionMerkleTreePubkey,
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
        transactionMerkleTree: transactionMerkleTreePubkey,
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
  inputUtxos?: Utxo[];
  outputUtxos?: OutUtxo[];
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

export type DecompressTransactionInput = {
  mint?: PublicKey;
  message?: Buffer;
  transactionMerkleTreePubkey: PublicKey;
  recipientSpl?: PublicKey;
  recipientSol?: PublicKey;
  inputUtxos?: Utxo[];
  outputUtxos?: OutUtxo[];
  rpcPublicKey: PublicKey;
  lightWasm: LightWasm;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  rpcFee: BN;
  ataCreationFee: boolean;
  assetLookUpTable?: string[];
};

// add createCompressSolanaTransaction
export async function createDecompressTransaction(
  decompressTransactionInput: DecompressTransactionInput,
): Promise<DecompressTransaction> {
  const {
    message,
    transactionMerkleTreePubkey,
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
    transactionMerkleTree: transactionMerkleTreePubkey,
  };

  // TODO: double check onchain code for consistency between utxo merkle trees and inserted merkle tree
  const encryptedUtxos = await encryptOutUtxos(
    account,
    privateVars.outputUtxos,
    transactionMerkleTreePubkey,
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

export type TransactionInput = {
  message?: Buffer;
  transactionMerkleTreePubkey: PublicKey;
  inputUtxos?: Utxo[];
  outputUtxos?: OutUtxo[];
  rpcPublicKey: PublicKey;
  lightWasm: LightWasm;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  rpcFee: BN;
  assetLookUpTable?: string[];
};

export async function createTransaction(
  transactionInput: TransactionInput,
): Promise<Transaction> {
  const {
    message,
    transactionMerkleTreePubkey,
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
  const verifierProgramId = pspId ? pspId : systemPspId;
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
    transactionMerkleTreePubkey,
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
        transactionMerkleTree: transactionMerkleTreePubkey,
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
  utxos?: Utxo[];
  recipientSol?: PublicKey;
  recipientSplAddress?: PublicKey;
  inUtxos?: Utxo[];
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

  let inputUtxos: Utxo[] = inUtxos ? [...inUtxos] : [];
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
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
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
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
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
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
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
