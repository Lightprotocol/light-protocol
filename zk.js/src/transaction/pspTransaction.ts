import { AnchorProvider, BN, Idl, Program, utils } from "@coral-xyz/anchor";
import { upperCamelCase, camelCase } from "case-anything";
import {
  AUTHORITY,
  BN_0,
  FIELD_SIZE,
  N_ASSET_PUBKEYS,
  STANDARD_SHIELDED_PRIVATE_KEY,
  STANDARD_SHIELDED_PUBLIC_KEY,
  SYSTEM_PROGRAM_IDLS,
  lightPsp2in2outStorageId,
  merkleTreeProgramId,
} from "../constants";
import {
  Account,
  ProviderErrorCode,
  SolMerkleTree,
  TransactionError,
  TransactionErrorCode,
  hashAndTruncateToCircuit,
  Utxo,
  truncateToCircuit,
  TransactionParametersError,
  TransactionParametersErrorCode,
  Action,
  MerkleTreeConfig,
  TokenData,
  Provider,
  Relayer,
  AppUtxoConfig,
  UserErrorCode,
  RelayerErrorCode,
  CreateUtxoErrorCode,
  selectInUtxos,
  createOutUtxos,
} from "../index";
import { Hasher } from "@lightprotocol/account.rs";
import { getIndices3D } from "@lightprotocol/circuit-lib.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { sha256 } from "@noble/hashes/sha256";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import nacl from "tweetnacl";

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
  const lastSytemField = "transactionVersion";
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
  checkedOutUtxos?: { utxoName: string; utxo: Utxo }[];
  inUtxos?: Utxo[];
  outUtxos?: Utxo[];
  accounts?: any;
};
type compiledProofInputs = {
  systemProofInputs: any;
  pspProofInputs: any;
};

// how do I best steamline the transaction generation process for psps?
// 1. define circuit specific proof inputs which are not part of the utxos appData - check whether inputs which are not utxos pausible
// 2. define in and out utxos
// 3.1 filter utxos that go into selection for input utxos -> select miising utxos
// 3.2 create output utxos
// 3.3 create transaction parameters
// 4. compile app parameters
// 5. compile and prove etc.
export const createUtxoIndices = (
  hasher: Hasher,
  utxos: Utxo[],
  commitHashUtxo: string,
) => {
  const isAppInUtxo = new Array(4).fill(new BN(0));
  for (const i in utxos) {
    if (utxos[i].getCommitment(hasher) === commitHashUtxo) {
      isAppInUtxo[i] = new BN(1);
    }
  }
  return isAppInUtxo;
};

export const createPspProofInputs = (
  hasher: Hasher,
  pspTransaction: PspTransactionInput,
  inputUtxos: Utxo[],
  outputUtxos: Utxo[],
  transactionHash: string,
): any => {
  /// disabling type-checking
  const inUtxosInputs: { [key: string]: any } = {};
  pspTransaction.checkedInUtxos?.forEach(({ utxoName, utxo }) => {
    for (const field in utxo.appData) {
      inUtxosInputs[`${utxoName}${upperCamelCase(field)}`] =
        utxo.appData[field];
    }

    const isAppUtxo = createUtxoIndices(
      hasher,
      inputUtxos,
      utxo.getCommitment(hasher),
    );

    inUtxosInputs[`isInAppUtxo${upperCamelCase(utxoName)}`] = isAppUtxo;
    inUtxosInputs[`${camelCase(utxoName)}Blinding`] = utxo.blinding;
    inUtxosInputs[`${camelCase(utxoName)}AmountSol`] = utxo.amounts[0];
    inUtxosInputs[`${camelCase(utxoName)}AmountSpl`] =
      utxo.amounts.length === 2 ? utxo.amounts[1] : BN_0;
    inUtxosInputs[`${camelCase(utxoName)}AssetSpl`] = utxo.assetsCircuit[1];
    inUtxosInputs[`${camelCase(utxoName)}PublicKey`] = utxo.publicKey;
    inUtxosInputs[`${camelCase(utxoName)}PoolType`] = utxo.poolType;
    inUtxosInputs[`${camelCase(utxoName)}PspOwner`] =
      utxo.verifierAddressCircuit;
    inUtxosInputs[`${camelCase(utxoName)}TxVersion`] = BN_0;
    // utxo data hash is calculated in the circuit
  });

  const outUtxosInputs = {};
  pspTransaction.checkedOutUtxos?.forEach(
    ({ utxoName, utxo }: { utxoName: string; utxo: Utxo }) => {
      for (const field in utxo.appData) {
        // @ts-ignore
        outUtxosInputs[`${utxoName}${upperCamelCase(field)}`] =
          utxo.appData[field];
      }

      const isAppUtxoIndices = createUtxoIndices(
        hasher,
        outputUtxos,
        utxo.getCommitment(hasher),
      );
      // @ts-ignore
      outUtxosInputs[`isOutAppUtxo${upperCamelCase(utxoName)}`] =
        isAppUtxoIndices;
      inUtxosInputs[`${camelCase(utxoName)}Blinding`] = utxo.blinding;
      inUtxosInputs[`${camelCase(utxoName)}AmountSol`] = utxo.amounts[0];
      inUtxosInputs[`${camelCase(utxoName)}AmountSpl`] =
        utxo.amounts.length === 2 ? utxo.amounts[1] : BN_0;
      inUtxosInputs[`${camelCase(utxoName)}AssetSpl`] = utxo.assetsCircuit[1];
      inUtxosInputs[`${camelCase(utxoName)}PublicKey`] = utxo.publicKey;
      inUtxosInputs[`${camelCase(utxoName)}PoolType`] = utxo.poolType;
      inUtxosInputs[`${camelCase(utxoName)}PspOwner`] =
        utxo.verifierAddressCircuit;
      inUtxosInputs[`${camelCase(utxoName)}TxVersion`] = BN_0;
    },
  );

  const publicAppVerifier = hashAndTruncateToCircuit(
    getVerifierProgramId(pspTransaction.verifierIdl).toBuffer(),
  );

  const compiledProofInputs = {
    ...pspTransaction.proofInputs,
    inPublicKey: inputUtxos?.map((utxo) => utxo.publicKey),
    transactionHash,
    publicAppVerifier,
    ...inUtxosInputs,
    ...outUtxosInputs,
  };
  return compiledProofInputs;
};
// TODO: add check that lenght input utxos is as expected by the verifier idl
export async function getSystemProof({
  account,
  inputUtxos,
  systemProofInputs,
  verifierIdl,
}: {
  account: Account;
  verifierIdl: Idl;
  inputUtxos: Utxo[];
  systemProofInputs: any;
}) {
  const path = require("path");
  const firstPath = path.resolve(__dirname, "../../build-circuits/");
  return account.getProofInternal({
    firstPath,
    verifierIdl,
    proofInput: systemProofInputs,
    addPrivateKey: true,
    inputUtxos,
  });
}

/**
 * @description Prepares proof inputs.
 */
export function createSystemProofInputs({
  transaction,
  hasher,
  account,
}: {
  transaction: Transaction;
  hasher: Hasher;
  account: Account;
}) {
  if (!transaction.public.txIntegrityHash)
    throw new TransactionError(
      TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
      "compile",
    );

  const inputNullifier = transaction.private.inputUtxos.map((x) => {
    let _account = account;
    if (x.publicKey.eq(STANDARD_SHIELDED_PUBLIC_KEY)) {
      _account = Account.fromPrivkey(
        hasher,
        bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
        bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
        bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
      );
    }
    return x.getNullifier({
      hasher: hasher,
      account: _account,
    });
  });
  const proofInput = {
    root: transaction.public.root,
    inputNullifier,
    publicAmountSpl: transaction.public.publicAmountSpl.toString(),
    publicAmountSol: transaction.public.publicAmountSol.toString(),
    publicMintPubkey: transaction.public.publicMintPubkey,
    inPathIndices: transaction.private.inputUtxos?.map((x) => x.index),
    inPathElements: transaction.private.inputUtxos?.map((x) => x.merkleProof),
    internalTxIntegrityHash: transaction.public.txIntegrityHash.toString(),
    transactionVersion: "0",
    txIntegrityHash: transaction.public.txIntegrityHash.toString(),
    outputCommitment: transaction.private.outputUtxos.map((x) =>
      x.getCommitment(hasher),
    ),
    inAmount: transaction.private.inputUtxos?.map((x) => x.amounts),
    inBlinding: transaction.private.inputUtxos?.map((x) => x.blinding),
    assetPubkeys: transaction.private.assetPubkeysCircuit,
    outAmount: transaction.private.outputUtxos?.map((x) => x.amounts),
    outBlinding: transaction.private.outputUtxos?.map((x) => x.blinding),
    outPubkey: transaction.private.outputUtxos?.map((x) => x.publicKey),
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
    inAppDataHash: transaction.private.inputUtxos?.map((x) => x.appDataHash),
    outAppDataHash: transaction.private.outputUtxos?.map((x) => x.appDataHash),
    inPoolType: transaction.private.inputUtxos?.map((x) => x.poolType),
    outPoolType: transaction.private.outputUtxos?.map((x) => x.poolType),
    inVerifierPubkey: transaction.private.inputUtxos?.map(
      (x) => x.verifierAddressCircuit,
    ),
    outVerifierPubkey: transaction.private.outputUtxos?.map(
      (x) => x.verifierAddressCircuit,
    ),
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

export function syncInputUtxosMerkleProofs({
  inputUtxos,
  hasher,
  solMerkleTree,
}: {
  inputUtxos: Utxo[];
  hasher: Hasher;
  solMerkleTree: SolMerkleTree;
}): { syncedUtxos: Utxo[]; root: string } {
  const proofs = solMerkleTree.getMerkleProofs(hasher, inputUtxos);
  const syncedUtxos = inputUtxos?.map((utxo, index) => {
    utxo.merkleProof = proofs.inputMerklePathElements[index];
    utxo.index = Number(proofs.inputMerklePathIndices[index]);
    return utxo;
  });
  return { syncedUtxos, root: solMerkleTree.merkleTree.root() };
}

// compileProofInputs
export function createProofInputs({
  transaction,
  hasher,
  account,
  pspTransaction,
}: {
  transaction: Transaction;
  pspTransaction: PspTransactionInput;
  hasher: Hasher;
  account: Account;
}): compiledProofInputs {
  const systemProofInputs = createSystemProofInputs({
    transaction,
    hasher,
    account,
  });
  const pspProofInputs = createPspProofInputs(
    hasher,
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
export type UnshieldAccounts = {
  recipientSol: PublicKey;
  recipientSpl: PublicKey;
  relayerPublicKey: PublicKey;
};

// TODO: make all inputs part of integrity hash
export type TransactionAccounts = {
  senderSpl: PublicKey;
  senderSol: PublicKey;
  recipientSpl: PublicKey;
  recipientSol: PublicKey;
  relayerPublicKey: PublicKey;
  transactionMerkleTree: PublicKey;
  systemPspId: PublicKey;
  pspId?: PublicKey;
};

export type PublicTransactionVariables = {
  accounts: TransactionAccounts;
  publicAmountSpl: BN;
  publicAmountSol: BN;
  relayerFee: BN;
  ataCreationFee: boolean;
  encryptedUtxos: Uint8Array;
  publicMintPubkey: string;
  message?: Buffer;
  transactionHash: string;
  // TODO: rename to publicDataHash
  txIntegrityHash: BN;
  root: string;
};

export type PrivateTransactionVariables = {
  inputUtxos: Array<Utxo>;
  outputUtxos: Array<Utxo>;
  assetPubkeys: PublicKey[];
  assetPubkeysCircuit: string[];
};

export type Transaction = {
  private: PrivateTransactionVariables;
  public: PublicTransactionVariables;
};

export type ShieldTransaction = Transaction & {
  action: Action;
};

export type UnshieldTransaction = Transaction & {
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
  const inputNullifierField = fields.find(
    (field) => field.name === "inputNullifier",
  ) as Field;
  const outputCommitmentField = fields.find(
    (field) => field.name === "outputCommitment",
  ) as Field;

  if (!inputNullifierField || !inputNullifierField.type.array) {
    throw new TransactionError(
      TransactionErrorCode.FIELD_NOT_FOUND,
      "getVerifierIdl",
      "inputNullifier field not found or has an incorrect type",
    );
  }

  if (!outputCommitmentField || !outputCommitmentField.type.array) {
    throw new TransactionError(
      TransactionErrorCode.FIELD_NOT_FOUND,
      "getVerifierIdl",
      "outputCommitment field not found or has an incorrect type",
    );
  }

  const inputNullifierLength = inputNullifierField.type.array[1];
  const outputCommitmentLength = outputCommitmentField.type.array[1];

  return { in: inputNullifierLength, out: outputCommitmentLength };
}

/**
 * @description Adds empty utxos until the desired number of utxos is reached.
 * @note The zero knowledge proof circuit needs all inputs to be defined.
 * @note Therefore, we have to pass in empty inputs for values we don't use.
 * @param utxos
 * @param len
 * @returns
 */
export function addEmptyUtxos(
  utxos: Utxo[] = [],
  len: number,
  hasher: Hasher,
  publicKey: BN,
): Utxo[] {
  while (utxos.length < len) {
    utxos.push(
      new Utxo({
        hasher: hasher,
        publicKey,
        assetLookupTable: [SystemProgram.programId.toBase58()],
        isFillingUtxo: true,
      }),
    );
  }
  return utxos;
}

/**
 * @description Assigns spl and sol senderSpl or recipientSpl accounts to transaction parameters based on action.
 */
// solanaTransaction
export function assignAccountsUnshield(
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

  // AUTHORITY is used as placeholder in case no spl recipient is unshielded
  const assignedRecipientSpl = recipientSpl ? recipientSpl : AUTHORITY;
  // AUTHORITY is used as placeholder in case no sol recipient is unshielded
  const assignedRecipientSol = recipientSol ? recipientSol : AUTHORITY;
  return {
    senderSol,
    senderSpl,
    recipientSol: assignedRecipientSol,
    recipientSpl: assignedRecipientSpl,
  };
}

// solanaTransaction assign accounts for shielded transfer
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

  // AUTHORITY is used as placeholder in case no spl recipient is unshielded
  const assignedRecipientSpl = AUTHORITY;
  // AUTHORITY is used as placeholder in case no sol recipient is unshielded
  const assignedRecipientSol = AUTHORITY;
  return {
    senderSol,
    senderSpl,
    recipientSol: assignedRecipientSol,
    recipientSpl: assignedRecipientSpl,
  };
}

export function assignAccountsShield(
  assetPubkeys: PublicKey[],
  systemPspId: PublicKey,
  senderSpl?: PublicKey,
) {
  const recipientSpl = MerkleTreeConfig.getSplPoolPdaToken(
    assetPubkeys[1],
    merkleTreeProgramId,
  );
  const recipientSol = MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;
  // AUTHORITY is used as placeholder in case no spl recipient is unshielded
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
  outputUtxos?: Utxo[],
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
  outputUtxos: Utxo[],
  assetPubkeysCircuit: string[],
): BN {
  return new BN(0)
    .add(
      outputUtxos
        .filter((utxo: Utxo) => {
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
 * the proof, but are included to prevent the relayer from changing any input of the
 * transaction.
 *
 * The hash is computed over the following inputs in the given order:
 * 1. Recipient SPL Account
 * 2. Recipient Solana Account
 * 3. Relayer Public Key
 * 4. Relayer Fee
 * 5. Encrypted UTXOs (limited to 512 bytes)
 *
 * @param {any} poseidon - Poseidon hash function instance.
 * @returns {Promise<BN>} A promise that resolves to the computed transaction integrity hash.
 * @throws {TransactionError} Throws an error if the relayer, recipient SPL or Solana accounts,
 * relayer fee, or encrypted UTXOs are undefined, or if the encryption of UTXOs fails.
 *
 * @example
 * const integrityHash = await getTxIntegrityHash(poseidonInstance);
 */
export function getTxIntegrityHash(
  relayerFee: BN,
  encryptedUtxos: Uint8Array,
  accounts: UnshieldAccounts,
  verifierConfig: VerifierConfig,
  verifierProgramId: PublicKey,
  message?: Uint8Array,
): BN {
  if (!relayerFee)
    throw new TransactionError(
      TransactionErrorCode.RELAYER_UNDEFINED,
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
  // console.log("messageHash ", messageHash);
  // console.log("recipientSpl ", Array.from(recipientSpl));
  // console.log("accounts.recipientSol ", Array.from(accounts.recipientSol.toBytes()));
  // console.log("accounts.relayerPublicKey ", Array.from(accounts.relayerPublicKey.toBytes()));
  // console.log("relayerFee ", Array.from(relayerFee.toArrayLike(Buffer, "be", 8)));
  // console.log("encryptedUtxos ", Array.from(encryptedUtxos));
  const hash = sha256
    .create()
    .update(messageHash)
    .update(recipientSpl)
    .update(accounts.recipientSol.toBytes())
    .update(accounts.relayerPublicKey.toBytes())
    .update(relayerFee.toArrayLike(Buffer, "be", 8)) // TODO: make be
    .update(encryptedUtxos)
    .digest();
  const txIntegrityHash = truncateToCircuit(hash);
  return txIntegrityHash;
}

// pspTransaction
export async function encryptOutUtxos(
  hasher: Hasher,
  account: Account,
  outputUtxos: Utxo[],
  transactionMerkleTree: PublicKey,
  verifierConfig: VerifierConfig,
): Promise<Uint8Array> {
  let encryptedOutputs = new Array<any>();
  for (const utxo in outputUtxos) {
    if (
      outputUtxos[utxo].appDataHash.toString() !== "0" &&
      outputUtxos[utxo].includeAppData
    )
      // TODO: implement encryption for utxos with app data
      console.log(
        "Warning encrypting utxos with app data as normal utxo without app data. App data will not be encrypted.",
      );
    encryptedOutputs.push(
      await outputUtxos[utxo].encrypt({
        hasher,
        account: account,
        merkleTreePdaPublicKey: transactionMerkleTree,
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
  hasher: Hasher,
  inputUtxos: Utxo[],
  outputUtxos: Utxo[],
  txIntegrityHash: BN,
): string {
  const inputHasher = hasher.poseidonHashString(
    inputUtxos?.map((utxo) => utxo.getCommitment(hasher)),
  );
  const outputHasher = hasher.poseidonHashString(
    outputUtxos?.map((utxo) => utxo.getCommitment(hasher)),
  );

  return hasher.poseidonHashString([
    inputHasher,
    outputHasher,
    txIntegrityHash.toString(),
  ]);
}
export type ShieldTransactionInput = {
  mint?: PublicKey;
  message?: Buffer;
  transactionMerkleTreePubkey: PublicKey;
  senderSpl?: PublicKey;
  inputUtxos?: Utxo[];
  outputUtxos?: Utxo[];
  signer: PublicKey;
  hasher: Hasher;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  root: string;
};

// add createShieldSolanaTransaction
export async function createShieldTransaction(
  shieldTransactionInput: ShieldTransactionInput,
): Promise<ShieldTransaction> {
  const {
    message,
    transactionMerkleTreePubkey,
    mint,
    senderSpl,
    inputUtxos,
    outputUtxos,
    signer,
    hasher,
    systemPspId,
    pspId,
    account,
    root,
  } = shieldTransactionInput;

  const action = Action.SHIELD;
  const verifierIdl = getSystemPspIdl(systemPspId);
  const verifierConfig = getVerifierConfig(verifierIdl);

  const privateVars = createPrivateTransactionVariables({
    inputUtxos,
    outputUtxos,
    hasher,
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

  const accounts = assignAccountsShield(
    privateVars.assetPubkeys,
    systemPspId,
    senderSpl,
  );
  const completeAccounts = {
    ...accounts,
    relayerPublicKey: signer,
    pspId,
  };

  // TODO: double check onchain code for consistency between utxo merkle trees and inserted merkle tree
  const encryptedUtxos = await encryptOutUtxos(
    hasher,
    account,
    privateVars.outputUtxos,
    transactionMerkleTreePubkey,
    verifierConfig,
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
    hasher,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    txIntegrityHash,
  );

  const transaction: ShieldTransaction = {
    action,
    private: privateVars,
    public: {
      root,
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
      relayerFee: BN_0,
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
  hasher,
  account,
  verifierConfig,
}: {
  inputUtxos?: Utxo[];
  outputUtxos?: Utxo[];
  hasher: Hasher;
  account: Account;
  verifierConfig: VerifierConfig;
}): PrivateTransactionVariables {
  const filledInputUtxos = addEmptyUtxos(
    inputUtxos,
    verifierConfig.in,
    hasher,
    account.pubkey,
  );
  const filledOutputUtxos = addEmptyUtxos(
    outputUtxos,
    verifierConfig.out,
    hasher,
    account.pubkey,
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

export type UnshieldTransactionInput = {
  mint?: PublicKey;
  message?: Buffer;
  transactionMerkleTreePubkey: PublicKey;
  recipientSpl?: PublicKey;
  recipientSol?: PublicKey;
  inputUtxos?: Utxo[];
  outputUtxos?: Utxo[];
  relayerPublicKey: PublicKey;
  hasher: Hasher;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  relayerFee: BN;
  ataCreationFee: boolean;
  root: string;
};

// add createShieldSolanaTransaction
export async function createUnshieldTransaction(
  unshieldTransactionInput: UnshieldTransactionInput,
): Promise<UnshieldTransaction> {
  const {
    message,
    transactionMerkleTreePubkey,
    mint,
    recipientSpl,
    recipientSol,
    inputUtxos,
    outputUtxos,
    relayerPublicKey,
    hasher,
    systemPspId,
    pspId,
    account,
    relayerFee,
    ataCreationFee,
    root,
  } = unshieldTransactionInput;

  const action = Action.UNSHIELD;
  const verifierIdl = getSystemPspIdl(systemPspId);
  const verifierConfig = getVerifierConfig(verifierIdl);

  const privateVars = createPrivateTransactionVariables({
    inputUtxos,
    outputUtxos,
    hasher,
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

  const accounts = assignAccountsUnshield(
    privateVars.assetPubkeys,
    recipientSol,
    recipientSpl,
  );
  const completeAccounts = {
    ...accounts,
    relayerPublicKey,
    systemPspId,
    pspId,
    transactionMerkleTree: transactionMerkleTreePubkey,
  };

  // TODO: double check onchain code for consistency between utxo merkle trees and inserted merkle tree
  const encryptedUtxos = await encryptOutUtxos(
    hasher,
    account,
    privateVars.outputUtxos,
    transactionMerkleTreePubkey,
    verifierConfig,
  );
  const txIntegrityHash = getTxIntegrityHash(
    relayerFee,
    encryptedUtxos,
    completeAccounts,
    verifierConfig,
    systemPspId,
    message,
  );

  const transactionHash = getTransactionHash(
    hasher,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    txIntegrityHash,
  );

  const transaction: UnshieldTransaction = {
    action,
    private: privateVars,
    public: {
      root,
      transactionHash,
      publicMintPubkey: mint
        ? hashAndTruncateToCircuit(mint.toBytes()).toString()
        : "0",
      txIntegrityHash,
      accounts: completeAccounts,
      publicAmountSpl,
      publicAmountSol,
      relayerFee,
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
  outputUtxos?: Utxo[];
  relayerPublicKey: PublicKey;
  hasher: Hasher;
  systemPspId: PublicKey;
  pspId?: PublicKey;
  account: Account;
  relayerFee: BN;
  root: string;
};

export async function createTransaction(
  transactionInput: TransactionInput,
): Promise<Transaction> {
  const {
    message,
    transactionMerkleTreePubkey,
    inputUtxos,
    outputUtxos,
    relayerPublicKey,
    hasher,
    pspId,
    systemPspId,
    account,
    relayerFee,
    root,
  } = transactionInput;
  const verifierProgramId = pspId ? pspId : systemPspId;
  // TODO: unifiy systemPspId and verifierProgramId by adding verifierConfig to psps
  const verifierConfig = getVerifierConfig(getSystemPspIdl(systemPspId));

  const privateVars = createPrivateTransactionVariables({
    inputUtxos,
    outputUtxos,
    hasher,
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
    relayerPublicKey,
  };

  // TODO: double check onchain code for consistency between utxo merkle trees and inserted merkle tree
  const encryptedUtxos = await encryptOutUtxos(
    hasher,
    account,
    privateVars.outputUtxos,
    transactionMerkleTreePubkey,
    verifierConfig,
  );
  const txIntegrityHash = getTxIntegrityHash(
    relayerFee,
    encryptedUtxos,
    completeAccounts,
    verifierConfig,
    verifierProgramId,
    message,
  );

  const transactionHash = getTransactionHash(
    hasher,
    privateVars.inputUtxos,
    privateVars.outputUtxos,
    txIntegrityHash,
  );

  const transaction: Transaction = {
    private: privateVars,
    public: {
      root,
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
      relayerFee,
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
  // for unshield
  recipientSol,
  recipientSplAddress,
  // for transfer
  outUtxos,
  relayer,
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
  outUtxos?: Utxo[];
  action: Action;
  provider: Provider;
  relayer?: Relayer;
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
}): Promise<Transaction | ShieldTransaction | UnshieldTransaction> {
  if (action === Action.TRANSFER && !outUtxos && !mergeUtxos)
    throw new TransactionParametersError(
      UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
      "getTxParams",
      "Recipient outUtxo not provided for transfer",
    );

  if (action !== Action.SHIELD && !relayer?.getRelayerFee(ataCreationFee)) {
    // TODO: could make easier to read by adding separate if/cases
    throw new TransactionParametersError(
      RelayerErrorCode.RELAYER_FEE_UNDEFINED,
      "getTxParams",
      `No relayerFee provided for ${action.toLowerCase()}}`,
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
  let outputUtxos: Utxo[] = outUtxos ? [...outUtxos] : [];

  if (addInUtxos) {
    inputUtxos = selectInUtxos({
      publicMint: tokenCtx.mint,
      publicAmountSpl,
      publicAmountSol,
      hasher: provider.hasher,
      inUtxos,
      outUtxos,
      utxos,
      relayerFee: relayer?.getRelayerFee(ataCreationFee),
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
      publicAmountSol, // TODO: add support for extra sol for unshield & transfer
      hasher: provider.hasher,
      relayerFee: relayer?.getRelayerFee(ataCreationFee),
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

  if (inputUtxos && inputUtxos.length != 0) {
    const { syncedUtxos } = syncInputUtxosMerkleProofs({
      inputUtxos,
      solMerkleTree: provider.solMerkleTree!,
      hasher: provider.hasher,
    });
    inputUtxos = syncedUtxos;
  }

  if (action == Action.SHIELD) {
    return createShieldTransaction({
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
      hasher: provider.hasher,
      systemPspId: getVerifierProgramId(verifierIdl),
      account,
      root: provider.solMerkleTree!.merkleTree.root(),
    });
  } else if (action == Action.UNSHIELD) {
    return createUnshieldTransaction({
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
      hasher: provider.hasher,
      systemPspId: getVerifierProgramId(verifierIdl),
      account,
      ataCreationFee: ataCreationFee ? true : false,
      relayerPublicKey: relayer!.accounts.relayerPubkey,
      relayerFee: relayer!.getRelayerFee(ataCreationFee),
      root: provider.solMerkleTree!.merkleTree.root(),
    });
  } else if (action == Action.TRANSFER) {
    return createTransaction({
      message,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      inputUtxos,
      outputUtxos,
      hasher: provider.hasher,
      systemPspId: getVerifierProgramId(verifierIdl),
      account,
      relayerPublicKey: relayer!.accounts.relayerPubkey,
      relayerFee: relayer!.getRelayerFee(ataCreationFee),
      root: provider.solMerkleTree!.merkleTree.root(),
    });
  } else {
    throw new TransactionParametersError(
      TransactionErrorCode.UNIMPLEMENTED,
      "getTxParams",
      `Action ${action} not implemented`,
    );
  }
}
