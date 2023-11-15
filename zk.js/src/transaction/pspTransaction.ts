import { BN, Idl } from "@coral-xyz/anchor";
import { upperCamelCase, camelCase } from "case-anything";
import { TransactionParameters } from "./transactionParameters";
import {
  BN_0,
  N_ASSET_PUBKEYS,
  STANDARD_SHIELDED_PRIVATE_KEY,
  STANDARD_SHIELDED_PUBLIC_KEY,
} from "../constants";
import {
  Account,
  ProviderErrorCode,
  SolMerkleTree,
  TransactionError,
  TransactionErrorCode,
  hashAndTruncateToCircuit,
  Utxo
} from "../index";
import { Poseidon } from "@lightprotocol/account.rs";
import { getIndices3D } from "@lightprotocol/circuit-lib.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

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
    throw new Error(
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
  poseidon: Poseidon,
  utxos: Utxo[],
  commitHashUtxo: string,
) => {
  const isAppInUtxo = new Array(4).fill(new BN(0));
  for (const i in utxos) {
    if (utxos[i].getCommitment(poseidon) === commitHashUtxo) {
      isAppInUtxo[i] = new BN(1);
    }
  }
  return isAppInUtxo;
};

export const createPspProofInputs = (
  poseidon: Poseidon,
  pspTransaction: PspTransactionInput,
  inputUtxos: Utxo[],
  outputUtxos: Utxo[],
  transactionHash: string,
): any => {
  const inUtxosInputs = {};
  pspTransaction.checkedInUtxos?.forEach(({ utxoName, utxo }) => {
    for (const field in utxo.appData) {
      // @ts-ignore
      inUtxosInputs[`${utxoName}${upperCamelCase(field)}`] =
        utxo.appData[field];
    }

    const isAppUtxo = createUtxoIndices(
      poseidon,
      inputUtxos,
      utxo.getCommitment(poseidon),
    );
    // @ts-ignore
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
        poseidon,
        outputUtxos,
        utxo.getCommitment(poseidon),
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
    TransactionParameters.getVerifierProgramId(
      pspTransaction.verifierIdl,
    ).toBuffer(),
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

export async function getSystemProof({
  account,
  transaction,
  systemProofInputs,
}: {
  account: Account;
  transaction: TransactionParameters;
  systemProofInputs: any;
}) {
  const path = require("path");
  const firstPath = path.resolve(__dirname, "../../build-circuits/");
  return account.getProofInternal(
    firstPath,
    transaction,
    systemProofInputs,
    true,
  );
}

/**
 * @description Prepares proof inputs.
 */
export function createSystemProofInputs({
  transaction,
  solMerkleTree,
  poseidon,
  account,
}: {
  transaction: TransactionParameters;
  solMerkleTree: SolMerkleTree;
  poseidon: Poseidon;
  account: Account;
}) {
  if (!solMerkleTree)
    throw new TransactionError(
      ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
      "getProofInput",
    );
  if (!transaction.txIntegrityHash)
    throw new TransactionError(
      TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
      "compile",
    );

  const { inputMerklePathIndices, inputMerklePathElements } =
    solMerkleTree.getMerkleProofs(poseidon, transaction.inputUtxos);
  const inputNullifier = transaction.inputUtxos.map((x) => {
    let _account = account;
    if (x.publicKey.eq(STANDARD_SHIELDED_PUBLIC_KEY)) {
      _account = Account.fromPrivkey(
        poseidon,
        bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
        bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
        bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
      );
    }
    return x.getNullifier({
      poseidon: poseidon,
      account: _account,
    });
  });
  const proofInput = {
    root: solMerkleTree.merkleTree.root(),
    inputNullifier,
    publicAmountSpl: transaction.publicAmountSpl.toString(),
    publicAmountSol: transaction.publicAmountSol.toString(),
    publicMintPubkey: getTransactionMint(transaction),
    inPathIndices: inputMerklePathIndices,
    inPathElements: inputMerklePathElements,
    internalTxIntegrityHash: transaction.txIntegrityHash.toString(),
    transactionVersion: "0",
    txIntegrityHash: transaction.txIntegrityHash.toString(),
    outputCommitment: transaction.outputUtxos.map((x) =>
      x.getCommitment(poseidon),
    ),
    inAmount: transaction.inputUtxos?.map((x) => x.amounts),
    inBlinding: transaction.inputUtxos?.map((x) => x.blinding),
    assetPubkeys: transaction.assetPubkeysCircuit,
    outAmount: transaction.outputUtxos?.map((x) => x.amounts),
    outBlinding: transaction.outputUtxos?.map((x) => x.blinding),
    outPubkey: transaction.outputUtxos?.map((x) => x.publicKey),
    inIndices: getIndices3D(
      transaction.inputUtxos[0].assets.length,
      N_ASSET_PUBKEYS,
      transaction.inputUtxos.map((utxo) => utxo.assetsCircuit),
      transaction.assetPubkeysCircuit,
    ),
    outIndices: getIndices3D(
      transaction.inputUtxos[0].assets.length,
      N_ASSET_PUBKEYS,
      transaction.outputUtxos.map((utxo) => utxo.assetsCircuit),
      transaction.assetPubkeysCircuit,
    ),
    inAppDataHash: transaction.inputUtxos?.map((x) => x.appDataHash),
    outAppDataHash: transaction.outputUtxos?.map((x) => x.appDataHash),
    inPoolType: transaction.inputUtxos?.map((x) => x.poolType),
    outPoolType: transaction.outputUtxos?.map((x) => x.poolType),
    inVerifierPubkey: transaction.inputUtxos?.map(
      (x) => x.verifierAddressCircuit,
    ),
    outVerifierPubkey: transaction.outputUtxos?.map(
      (x) => x.verifierAddressCircuit,
    ),
  };
  return proofInput;
}

export function getTransactionMint(transaction: TransactionParameters) {
  if (transaction.publicAmountSpl.eq(BN_0)) {
    return BN_0;
  } else if (transaction.assetPubkeysCircuit) {
    return transaction.assetPubkeysCircuit[1];
  } else {
    throw new TransactionError(
      TransactionErrorCode.GET_MINT_FAILED,
      "getMint",
      "Failed to retrieve mint. The transaction parameters should contain 'assetPubkeysCircuit' after initialization, but it's missing.",
    );
  }
}

// compileProofInputs
export function createProofInputs({
  transaction,
  solMerkleTree,
  poseidon,
  account,
  pspTransaction,
}: {
  pspTransaction: PspTransactionInput;
  transaction: TransactionParameters;
  solMerkleTree: SolMerkleTree;
  poseidon: Poseidon;
  account: Account;
}): compiledProofInputs {
  const systemProofInputs = createSystemProofInputs({
    transaction,
    solMerkleTree,
    poseidon,
    account,
  });
  const pspProofInputs = createPspProofInputs(
    poseidon,
    pspTransaction,
    transaction.inputUtxos,
    transaction.outputUtxos,
    transaction.getTransactionHash(poseidon).toString(),
  );
  return {
    ...systemProofInputs,
    ...pspProofInputs,
  };
}
