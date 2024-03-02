import {
  AccountMeta,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { Utxo, UtxoSerde, UtxoWithMerkleContext } from "../state";
import { pushUniqueItems, toArray } from "../utils/conversion";
import { LightSystemProgram } from "../programs/compressed-pda";
import { ValidityProof, checkValidityProofShape } from "./validity-proof";

/// TODO: from static anchor idl
export interface InstructionDataTransfer2 {
  proof_a: number[];
  proof_b: number[];
  proof_c: number[];
  low_element_indices: number[];
  root_indices: number[];
  rpc_fee: bigint;
  utxos: UtxoSerde;
}
/** Instruction context for state  */
export type InputState = {
  /** The utxos describing the state that is to be consumed  */
  inputUtxos: UtxoWithMerkleContext[];
  /** The indices of the state roots of the input utxos */
  inputStateRootIndices: number[];
  inputStateNullifierQueueAccounts: PublicKey[];
};

/** Instruction context for state' */
export type NewStateParams = {
  /** utxos describing state' */
  outputUtxos: Utxo[];
  /**
   * The pubkeys of the state trees that the utxos should be inserted into
   * If undefined, the utxos are inserted into the state tree of the 1st input utxo
   */
  outputMerkleTrees?: PublicKey[];
};

/** Format instruction data struct to align with anchor idl */
const rawInstructionData = (
  inputUtxos: UtxoWithMerkleContext[],
  recentInputStateRootIndices: number[],
  recentValidityProof: ValidityProof,
  serializedUtxos: UtxoSerde
): InstructionDataTransfer2 => {
  return {
    proof_a: Array.from(recentValidityProof.proofA),
    proof_b: Array.from(recentValidityProof.proofB),
    proof_c: Array.from(recentValidityProof.proofC),
    low_element_indices: inputUtxos.map((_) => 0), // TODO: impl.!
    root_indices: recentInputStateRootIndices,
    rpc_fee: BigInt(0),
    utxos: serializedUtxos,
  };
};

/** Pad output state trees with default tree */
function padOutputStateTrees(
  outputStateTrees: PublicKey[] | undefined,
  defaultTree: PublicKey,
  length: number
): PublicKey[] {
  if (!outputStateTrees || outputStateTrees.length < length) {
    const paddedTrees = new Array(length).fill(defaultTree);
    if (outputStateTrees) {
      outputStateTrees.forEach((tree, index) => {
        paddedTrees[index] = tree;
      });
    }
    return paddedTrees;
  }
  return outputStateTrees;
}

/**
 * Compresses instruction data.
 */
export interface PackInstructionParams {
  /** Utxos describing the current state to be consumed in the instruction */
  inputState: UtxoWithMerkleContext[] | UtxoWithMerkleContext;
  /** Utxos describing the new state that is to be created */
  outputState: Utxo[] | Utxo;
  /** The indices of the state roots of the input utxos. Expire with validityProof */
  recentInputStateRootIndices: number[];
  /** A recent validity proof for the input state */
  recentValidityProof: ValidityProof;
  /** Optional state trees that the new state should be inserted into. Defaults to 1st state tree of input state */
  outputStateTrees?: PublicKey[];
  /** TODO: account for separate signers */
  payer: PublicKey;
  /** static acccounts  */
  staticAccounts: PublicKey[];
}

/**
 * Compresses instruction data
 * TODO: check if can replace coder with sync operation
 */
export async function packInstruction(
  params: PackInstructionParams
): Promise<TransactionInstruction> {
  /// validate params
  checkValidityProofShape(params.recentValidityProof);

  const inputUtxos = toArray<UtxoWithMerkleContext>(params.inputState);
  const outputUtxos = toArray<Utxo>(params.outputState);

  /// pad output state trees with 1st input state tree
  const outputStateTrees = padOutputStateTrees(
    params.outputStateTrees,
    inputUtxos[0].merkleTree,
    outputUtxos.length
  );

  /// map unique accounts
  const remainingAccounts: PublicKey[] = [];
  const inputMerkleTrees = inputUtxos.map((utxo) => utxo.merkleTree);
  const stateNullifierQueues = inputUtxos.map(
    (utxo) => utxo.stateNullifierQueue
  );

  pushUniqueItems<PublicKey>(inputMerkleTrees, remainingAccounts);
  pushUniqueItems<PublicKey>(stateNullifierQueues, remainingAccounts);
  pushUniqueItems<PublicKey>(outputStateTrees, remainingAccounts);

  const remainingAccountMetas = remainingAccounts.map(
    (account): AccountMeta => ({
      pubkey: account,
      isWritable: true, // TODO: inputmerkletrees no write
      isSigner: false,
    })
  );

  /// combine static and remaining accounts
  const staticAccounts = [params.payer, ...params.staticAccounts];
  const staticAccountMetas = staticAccounts.map(
    (account): AccountMeta => ({
      pubkey: account,
      isWritable: false,
      isSigner: true, // signers
    })
  );
  const allAccounts = [...staticAccounts, ...remainingAccounts];
  const leafIndices = inputUtxos.map((utxo) => utxo.leafIndex);

  const serializedUtxos = new UtxoSerde()
    .addinputUtxos(
      inputUtxos,
      allAccounts,
      leafIndices,
      inputMerkleTrees,
      stateNullifierQueues
    )
    .addoutputUtxos(
      outputUtxos,
      allAccounts,
      remainingAccounts,
      outputStateTrees
    );

  /// make instruction data
  const rawInputs = rawInstructionData(
    inputUtxos,
    params.recentInputStateRootIndices,
    params.recentValidityProof,
    serializedUtxos
  );
  const data = await LightSystemProgram.program.coder.accounts.encode(
    "InstructionDataTransfer2",
    rawInputs
  );
  // TODO: check whether other conv. required here
  return new TransactionInstruction({
    keys: [...staticAccountMetas, ...remainingAccountMetas],
    data,
    programId: PublicKey.default,
  });
}
