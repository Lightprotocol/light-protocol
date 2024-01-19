import {
  ComputeBudgetProgram,
  TransactionInstruction,
  TransactionMessage,
  Blockhash,
  VersionedTransaction,
  AddressLookupTableAccountArgs,
} from "@solana/web3.js";

import { PrioritizationFee } from "../types";

/**
 * Creates a Light Transaction (array of VersionedTransactions) from a given array of TransactionInstructions.
 * Txs must be executed in the order they appear in the array.
 * The last tx verifies the correctness of the state transition and updates the protocol state.
 * All preceding txs (if any) send data to the chain.
 */
export function createSolanaTransactions(
  ixs: TransactionInstruction[],
  recentBlockhash: Blockhash,
  versionedTransactionLookupTableAccountArgs: AddressLookupTableAccountArgs,
  prioritizationFee?: PrioritizationFee,
): VersionedTransaction[] {
  const versionedTransactions: VersionedTransaction[] = [];

  for (const ix of ixs) {
    versionedTransactions.push(
      createVersionedTransaction(
        ix,
        versionedTransactionLookupTableAccountArgs,
        recentBlockhash,
        prioritizationFee,
      ),
    );
  }
  return versionedTransactions;
}

/**
 * Creates and compiles a VersionedTransaction from a TransactionInstruction.
 */
export function createVersionedTransaction(
  ix: TransactionInstruction,
  versionedTransactionLookupTableAccountArgs: AddressLookupTableAccountArgs,
  recentBlockhash: Blockhash,
  prioritizationFee?: PrioritizationFee,
): VersionedTransaction {
  const payerKey = ix.keys.find((key) => key.isSigner)?.pubkey;

  if (!payerKey) {
    throw new Error(`Instruction must have exactly one signer`);
  }

  // TODO: we should set cu to minimum required for execution to save cost and maximize throughput.
  const txMsg = new TransactionMessage({
    payerKey,
    instructions: prioritizationFee
      ? [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ComputeBudgetProgram.setComputeUnitPrice({
            microLamports: prioritizationFee,
          }),
          ix,
        ]
      : [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }), ix],
    recentBlockhash,
  });

  const { state, key } = versionedTransactionLookupTableAccountArgs;

  const compiledTx = txMsg.compileToV0Message([
    {
      state,
      key,
      isActive: () => true,
    },
  ]);

  const versionedTransaction = new VersionedTransaction(compiledTx);
  return versionedTransaction;
}
